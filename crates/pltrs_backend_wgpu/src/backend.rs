use crate::vertex::{LineVertex, ScatterInstance, ScatterVertex};
use anyhow::{anyhow, Context};
use bytemuck::{Pod, Zeroable};
use pltrs_core::{Color, Figure, RenderBackend};
use pltrs_text::TextRenderer;
use std::{
    fs::File,
    io::BufWriter,
    path::Path,
    sync::{mpsc, Arc},
};
use wgpu::util::DeviceExt;
use winit::{event_loop::ActiveEventLoop, keyboard::KeyCode, window::Window};

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct LineUniforms {
    color: [f32; 4],
    _padding: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct ScatterGlobalUniforms {
    viewport_size: [f32; 2],
}

struct RenderResources {
    device: wgpu::Device,
    queue: wgpu::Queue,
    size: winit::dpi::PhysicalSize<u32>,
    line_pipeline: wgpu::RenderPipeline,
    line_bind_group_layout: wgpu::BindGroupLayout,
    scatter_pipeline: wgpu::RenderPipeline,
    scatter_bind_group_layout: wgpu::BindGroupLayout,
    text_renderer: TextRenderer,
}

pub struct WgpuBackend {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    surface_format: wgpu::TextureFormat,
    config: wgpu::SurfaceConfiguration,
    // is_surface_configured: bool, // unused for now
    resources: RenderResources,

    // Frame state
    current_texture: Option<wgpu::SurfaceTexture>,
    current_view: Option<wgpu::TextureView>,
    current_encoder: Option<wgpu::CommandEncoder>,
}

impl WgpuBackend {
    /// Contains the core WgpuBackend logic, including initialization, resource management, and rendering pipeline setup.
    pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let size = window.inner_size();
        let surface = instance
            .create_surface(window.clone())
            .context("failed to create render surface for the window")?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .context("failed to find a suitable GPU adapter for the current window surface")?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await
            .context("failed to create a logical device for the selected GPU adapter")?;
        let surface_caps = surface.get_capabilities(&adapter);
        let (surface_format, config) = surface_config_for_caps(&surface_caps, size)?;
        let resources = build_render_resources(device, queue, size, surface_format)?;

        let backend = Self {
            window,
            surface,
            surface_format,
            config,
            resources,
            current_texture: None,
            current_view: None,
            current_encoder: None,
        };
        backend.configure_surface();
        Ok(backend)
    }

    fn configure_surface(&self) {
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: self.surface_format,
            view_formats: vec![self.surface_format.add_srgb_suffix()],
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            width: self.resources.size.width,
            height: self.resources.size.height,
            desired_maximum_frame_latency: 2,
            present_mode: wgpu::PresentMode::AutoVsync,
        };
        self.surface
            .configure(&self.resources.device, &surface_config);
    }

    pub fn handle_key(&self, event_loop: &ActiveEventLoop, code: KeyCode, is_pressed: bool) {
        match (code, is_pressed) {
            (KeyCode::Escape, true) => event_loop.exit(),
            _ => {}
        }
    }

    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }
}

impl RenderBackend for WgpuBackend {
    fn begin_frame(&mut self, clear: Color) {
        let surface_texture = match self.surface.get_current_texture() {
            Ok(texture) => texture,
            Err(wgpu::SurfaceError::Lost) => {
                self.configure_surface();
                return; // Try next frame
            }
            Err(e) => {
                eprintln!("Surface error: {:?}", e);
                return;
            }
        };

        let texture_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                format: Some(self.surface_format.add_srgb_suffix()),
                ..Default::default()
            });

        let mut encoder = self
            .resources
            .device
            .create_command_encoder(&Default::default());
        clear_view(&mut encoder, &texture_view, clear);

        self.current_texture = Some(surface_texture);
        self.current_view = Some(texture_view);
        self.current_encoder = Some(encoder);
    }

    fn draw_scene(&mut self, fig: &Figure) {
        let (view, encoder) = match (&self.current_view, &mut self.current_encoder) {
            (Some(v), Some(e)) => (v, e),
            _ => return,
        };
        draw_figure(&mut self.resources, encoder, view, fig);
    }

    fn end_frame(&mut self) {
        let encoder = match self.current_encoder.take() {
            Some(e) => e,
            None => return,
        };
        let texture = match self.current_texture.take() {
            Some(t) => t,
            None => return,
        };

        self.resources.queue.submit(Some(encoder.finish()));
        self.window.pre_present_notify();
        texture.present();
    }

    fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.resources.device, &self.config);
            self.resources
                .text_renderer
                .resize(width, height, &self.resources.queue);
            // self.is_surface_configured = true;
        }
        self.resources.size = winit::dpi::PhysicalSize { width, height };
        self.configure_surface();
    }
}

pub async fn save_figure_png(fig: &Figure, path: &Path) -> anyhow::Result<()> {
    if !path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("png"))
    {
        return Err(anyhow!("only .png output is currently supported"));
    }

    if fig.size.width == 0 || fig.size.height == 0 {
        return Err(anyhow!(
            "figure size must be non-zero for offscreen rendering"
        ));
    }

    let size = winit::dpi::PhysicalSize::new(fig.size.width, fig.size.height);
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let mut resources = create_headless_resources(size, format).await?;

    let texture = resources.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Offscreen Render Target"),
        size: wgpu::Extent3d {
            width: size.width,
            height: size.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = resources
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Offscreen Render Encoder"),
        });

    clear_view(&mut encoder, &view, fig.clear_color);
    draw_figure(&mut resources, &mut encoder, &view, fig);

    let unpadded_bytes_per_row = size.width * 4;
    let padded_bytes_per_row = padded_bytes_per_row(unpadded_bytes_per_row);
    let output_buffer = resources.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Offscreen Output Buffer"),
        size: padded_bytes_per_row as u64 * size.height as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    encoder.copy_texture_to_buffer(
        wgpu::ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::ImageCopyBuffer {
            buffer: &output_buffer,
            layout: wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(padded_bytes_per_row),
                rows_per_image: Some(size.height),
            },
        },
        wgpu::Extent3d {
            width: size.width,
            height: size.height,
            depth_or_array_layers: 1,
        },
    );

    resources.queue.submit(Some(encoder.finish()));

    let buffer_slice = output_buffer.slice(..);
    let (tx, rx) = mpsc::channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = tx.send(result);
    });
    resources.device.poll(wgpu::Maintain::Wait);
    rx.recv()
        .context("failed waiting for GPU readback")?
        .context("failed mapping render output for readback")?;

    let mapped = buffer_slice.get_mapped_range();
    let mut pixels = vec![0_u8; (unpadded_bytes_per_row * size.height) as usize];
    for row in 0..size.height as usize {
        let src_offset = row * padded_bytes_per_row as usize;
        let dst_offset = row * unpadded_bytes_per_row as usize;
        let src = &mapped[src_offset..src_offset + unpadded_bytes_per_row as usize];
        let dst = &mut pixels[dst_offset..dst_offset + unpadded_bytes_per_row as usize];
        dst.copy_from_slice(src);
    }
    drop(mapped);
    output_buffer.unmap();

    let file = File::create(path)
        .with_context(|| format!("failed to create output image at {}", path.display()))?;
    let writer = BufWriter::new(file);
    let mut encoder = png::Encoder::new(writer, size.width, size.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut png_writer = encoder
        .write_header()
        .context("failed to write PNG header")?;
    png_writer
        .write_image_data(&pixels)
        .context("failed to encode PNG image data")?;

    Ok(())
}

async fn create_headless_resources(
    size: winit::dpi::PhysicalSize<u32>,
    format: wgpu::TextureFormat,
) -> anyhow::Result<RenderResources> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        ..Default::default()
    });
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: None,
            ..Default::default()
        })
        .await
        .context("failed to find a suitable GPU adapter for offscreen rendering")?;
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor::default(), None)
        .await
        .context("failed to create a logical device for offscreen rendering")?;

    build_render_resources(device, queue, size, format)
}

fn build_render_resources(
    device: wgpu::Device,
    queue: wgpu::Queue,
    size: winit::dpi::PhysicalSize<u32>,
    target_format: wgpu::TextureFormat,
) -> anyhow::Result<RenderResources> {
    let line_shader = device.create_shader_module(wgpu::include_wgsl!("line_shader.wgsl"));

    let line_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Line Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

    let line_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Line Pipeline Layout"),
        bind_group_layouts: &[&line_bind_group_layout],
        push_constant_ranges: &[],
    });

    let line_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Line Render Pipeline"),
        layout: Some(&line_pipeline_layout),
        vertex: wgpu::VertexState {
            module: &line_shader,
            entry_point: "vs_main",
            buffers: &[LineVertex::desc()],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &line_shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: target_format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
    });

    let scatter_shader = device.create_shader_module(wgpu::include_wgsl!("scatter_shader.wgsl"));

    let scatter_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Scatter Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

    let scatter_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Scatter Pipeline Layout"),
        bind_group_layouts: &[&scatter_bind_group_layout],
        push_constant_ranges: &[],
    });

    let scatter_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Scatter Render Pipeline"),
        layout: Some(&scatter_pipeline_layout),
        vertex: wgpu::VertexState {
            module: &scatter_shader,
            entry_point: "vs_main",
            buffers: &[ScatterVertex::desc(), ScatterInstance::desc()],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &scatter_shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: target_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleStrip,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
    });

    let text_renderer = TextRenderer::new(&device, size.width, size.height, target_format)?;

    Ok(RenderResources {
        device,
        queue,
        size,
        line_pipeline,
        line_bind_group_layout,
        scatter_pipeline,
        scatter_bind_group_layout,
        text_renderer,
    })
}

fn surface_config_for_caps(
    surface_caps: &wgpu::SurfaceCapabilities,
    size: winit::dpi::PhysicalSize<u32>,
) -> anyhow::Result<(wgpu::TextureFormat, wgpu::SurfaceConfiguration)> {
    if surface_caps.formats.is_empty() {
        return Err(anyhow!("surface reported no supported texture formats"));
    }

    let surface_format = surface_caps
        .formats
        .iter()
        .find(|f| f.is_srgb())
        .copied()
        .unwrap_or(surface_caps.formats[0]);

    Ok((
        surface_format,
        wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        },
    ))
}

fn clear_view(encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView, clear: Color) {
    let _rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Clear Pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color {
                    r: clear.r as f64,
                    g: clear.g as f64,
                    b: clear.b as f64,
                    a: clear.a as f64,
                }),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
    });
}

fn draw_figure(
    resources: &mut RenderResources,
    encoder: &mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    fig: &Figure,
) {
    let batches = pltrs_core::build_batches(fig);
    let plot_scissor = fig
        .axes
        .first()
        .and_then(|axes| scissor_rect_for_axes(axes.rect, resources.size));
    resources.text_renderer.queue(
        &resources.device,
        &resources.queue,
        resources.size.width,
        resources.size.height,
        &batches.texts,
    );

    for line_batch in &batches.lines {
        if line_batch.vertices.len() < 2 {
            continue;
        }

        let vertices: Vec<LineVertex> = line_batch
            .vertices
            .windows(2)
            .flat_map(|segment| {
                let [p0, p1] = [segment[0], segment[1]];
                let dx = (p1[0] - p0[0]) * resources.size.width as f32;
                let dy = (p1[1] - p0[1]) * resources.size.height as f32;
                let len = (dx * dx + dy * dy).sqrt();

                if len <= f32::EPSILON {
                    return Vec::new();
                }

                let half_width = line_batch.width.max(1.0) * 0.5;
                let offset_x = (-dy / len) * half_width / resources.size.width as f32;
                let offset_y = (dx / len) * half_width / resources.size.height as f32;

                vec![
                    LineVertex {
                        position: [p0[0] + offset_x, p0[1] + offset_y],
                    },
                    LineVertex {
                        position: [p0[0] - offset_x, p0[1] - offset_y],
                    },
                    LineVertex {
                        position: [p1[0] + offset_x, p1[1] + offset_y],
                    },
                    LineVertex {
                        position: [p1[0] + offset_x, p1[1] + offset_y],
                    },
                    LineVertex {
                        position: [p0[0] - offset_x, p0[1] - offset_y],
                    },
                    LineVertex {
                        position: [p1[0] - offset_x, p1[1] - offset_y],
                    },
                ]
            })
            .collect();

        if vertices.is_empty() {
            continue;
        }

        let vertex_buffer =
            resources
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Line Vertex Buffer"),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });

        draw_solid_triangles(
            resources,
            encoder,
            view,
            &vertex_buffer,
            vertices.len() as u32,
            line_batch.color,
            plot_scissor,
            "Line Draw Pass",
            "Line Uniform Buffer",
            "Line Bind Group",
        );
    }

    for solid_batch in &batches.solids {
        if solid_batch.vertices.is_empty() {
            continue;
        }

        let vertex_buffer =
            resources
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Solid Vertex Buffer"),
                    contents: bytemuck::cast_slice(&solid_batch.vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });

        draw_solid_triangles(
            resources,
            encoder,
            view,
            &vertex_buffer,
            solid_batch.vertices.len() as u32,
            solid_batch.color,
            plot_scissor,
            "Solid Draw Pass",
            "Solid Uniform Buffer",
            "Solid Bind Group",
        );
    }

    if !batches.markers.is_empty() {
        let uniforms = ScatterGlobalUniforms {
            viewport_size: [resources.size.width as f32, resources.size.height as f32],
        };
        let uniform_buffer =
            resources
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Scatter Global Uniforms"),
                    contents: bytemuck::bytes_of(&uniforms),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        let bind_group = resources
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Scatter Bind Group"),
                layout: &resources.scatter_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                }],
            });

        let strip_vertices = [
            ScatterVertex {
                position: [-0.5, 0.5],
            },
            ScatterVertex {
                position: [-0.5, -0.5],
            },
            ScatterVertex {
                position: [0.5, 0.5],
            },
            ScatterVertex {
                position: [0.5, -0.5],
            },
        ];

        let vertex_buffer =
            resources
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Scatter Quad Buffer"),
                    contents: bytemuck::cast_slice(&strip_vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });

        for batch in &batches.markers {
            let instances: Vec<ScatterInstance> = batch
                .positions
                .iter()
                .map(|p| ScatterInstance {
                    position: *p,
                    color: [batch.color.r, batch.color.g, batch.color.b, batch.color.a],
                    size: batch.size,
                    marker_type: match batch.marker {
                        pltrs_core::Marker::Circle => 0,
                        pltrs_core::Marker::Square => 1,
                    },
                })
                .collect();

            let instance_buffer =
                resources
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Scatter Instance Buffer"),
                        contents: bytemuck::cast_slice(&instances),
                        usage: wgpu::BufferUsages::VERTEX,
                    });

            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Scatter Draw Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            rpass.set_pipeline(&resources.scatter_pipeline);
            if let Some((x, y, width, height)) = plot_scissor {
                rpass.set_scissor_rect(x, y, width, height);
            }
            rpass.set_bind_group(0, &bind_group, &[]);
            rpass.set_vertex_buffer(0, vertex_buffer.slice(..));
            rpass.set_vertex_buffer(1, instance_buffer.slice(..));
            rpass.draw(0..4, 0..instances.len() as u32);
        }
    }

    resources
        .text_renderer
        .draw(encoder, view, !batches.texts.is_empty());
}

fn draw_solid_triangles(
    resources: &RenderResources,
    encoder: &mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    vertex_buffer: &wgpu::Buffer,
    vertex_count: u32,
    color: Color,
    scissor_rect: Option<(u32, u32, u32, u32)>,
    pass_label: &str,
    uniform_label: &str,
    bind_group_label: &str,
) {
    let uniforms = LineUniforms {
        color: [color.r, color.g, color.b, color.a],
        _padding: [0.0; 4],
    };

    let uniform_buffer = resources
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(uniform_label),
            contents: bytemuck::bytes_of(&uniforms),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

    let bind_group = resources
        .device
        .create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(bind_group_label),
            layout: &resources.line_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some(pass_label),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
    });

    rpass.set_pipeline(&resources.line_pipeline);
    if let Some((x, y, width, height)) = scissor_rect {
        rpass.set_scissor_rect(x, y, width, height);
    }
    rpass.set_bind_group(0, &bind_group, &[]);
    rpass.set_vertex_buffer(0, vertex_buffer.slice(..));
    rpass.draw(0..vertex_count, 0..1);
}

fn scissor_rect_for_axes(
    rect: pltrs_core::Rect,
    size: winit::dpi::PhysicalSize<u32>,
) -> Option<(u32, u32, u32, u32)> {
    if size.width == 0 || size.height == 0 {
        return None;
    }

    let x = (rect.x.clamp(0.0, 1.0) * size.width as f32).floor() as u32;
    let y_top = ((1.0 - (rect.y + rect.h).clamp(0.0, 1.0)) * size.height as f32).floor() as u32;
    let x_end = ((rect.x + rect.w).clamp(0.0, 1.0) * size.width as f32).ceil() as u32;
    let y_end = ((1.0 - rect.y.clamp(0.0, 1.0)) * size.height as f32).ceil() as u32;

    let width = x_end.saturating_sub(x);
    let height = y_end.saturating_sub(y_top);
    if width == 0 || height == 0 {
        None
    } else {
        Some((x, y_top, width, height))
    }
}

fn padded_bytes_per_row(unpadded_bytes_per_row: u32) -> u32 {
    let alignment = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    unpadded_bytes_per_row.div_ceil(alignment) * alignment
}
