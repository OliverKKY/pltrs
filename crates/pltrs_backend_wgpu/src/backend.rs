use crate::vertex::{LineVertex, ScatterInstance, ScatterVertex};
use anyhow::{anyhow, Context};
use bytemuck::{Pod, Zeroable};
use pltrs_core::{Color, Figure, RenderBackend};
use pltrs_text::TextRenderer;
use std::sync::Arc;
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

pub struct WgpuBackend {
    window: Arc<Window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface<'static>,
    surface_format: wgpu::TextureFormat,
    config: wgpu::SurfaceConfiguration,
    // is_surface_configured: bool, // unused for now
    line_pipeline: wgpu::RenderPipeline,
    line_bind_group_layout: wgpu::BindGroupLayout,
    scatter_pipeline: wgpu::RenderPipeline,
    scatter_bind_group_layout: wgpu::BindGroupLayout,
    text_renderer: TextRenderer,

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
        let surface = instance.create_surface(window.clone()).unwrap();
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
        if surface_caps.formats.is_empty() {
            return Err(anyhow!("surface reported no supported texture formats"));
        }
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        // --- Line Pipeline Setup ---
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
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None, // No culling for lines
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

        // --- Scatter Pipeline Setup ---
        let scatter_shader =
            device.create_shader_module(wgpu::include_wgsl!("scatter_shader.wgsl")); // TODO: Add shader file

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

        let scatter_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
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
                    format: config.format,
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

        let text_renderer = TextRenderer::new(&device, size.width, size.height, config.format)?;

        let backend = Self {
            window,
            device,
            queue,
            size,
            surface,
            surface_format,
            config,
            // is_surface_configured: false,
            line_pipeline,
            line_bind_group_layout,
            scatter_pipeline,
            scatter_bind_group_layout,
            text_renderer,
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
            width: self.size.width,
            height: self.size.height,
            desired_maximum_frame_latency: 2,
            present_mode: wgpu::PresentMode::AutoVsync,
        };
        self.surface.configure(&self.device, &surface_config);
    }

    pub fn handle_key(&self, event_loop: &ActiveEventLoop, code: KeyCode, is_pressed: bool) {
        match (code, is_pressed) {
            (KeyCode::Escape, true) => event_loop.exit(),
            _ => {}
        }
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

        let mut encoder = self.device.create_command_encoder(&Default::default());

        // Perform clear pass
        {
            let _rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Clear Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &texture_view,
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

        self.current_texture = Some(surface_texture);
        self.current_view = Some(texture_view);
        self.current_encoder = Some(encoder);
    }

    fn draw_scene(&mut self, fig: &Figure) {
        let batches = pltrs_core::build_batches(fig);
        self.text_renderer.queue(
            &self.device,
            &self.queue,
            self.size.width,
            self.size.height,
            &batches.texts,
        );

        let (view, encoder) = match (&self.current_view, &mut self.current_encoder) {
            (Some(v), Some(e)) => (v, e),
            _ => return,
        };

        // Process line batches
        for line_batch in &batches.lines {
            if line_batch.vertices.len() < 2 {
                continue;
            }

            // Create vertex buffer
            let vertices: Vec<LineVertex> = line_batch
                .vertices
                .windows(2)
                .flat_map(|segment| {
                    let [p0, p1] = [segment[0], segment[1]];
                    let dx = (p1[0] - p0[0]) * self.size.width as f32;
                    let dy = (p1[1] - p0[1]) * self.size.height as f32;
                    let len = (dx * dx + dy * dy).sqrt();

                    if len <= f32::EPSILON {
                        return Vec::new();
                    }

                    let half_width = line_batch.width.max(1.0) * 0.5;
                    let offset_x = (-dy / len) * half_width / self.size.width as f32;
                    let offset_y = (dx / len) * half_width / self.size.height as f32;

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

            let vertex_buffer = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Line Vertex Buffer"),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });

            // Create uniform buffer
            let uniforms = LineUniforms {
                color: [
                    line_batch.color.r,
                    line_batch.color.g,
                    line_batch.color.b,
                    line_batch.color.a,
                ],
                _padding: [0.0; 4],
            };

            let uniform_buffer =
                self.device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Line Uniform Buffer"),
                        contents: bytemuck::bytes_of(&uniforms),
                        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    });

            // Create bind group
            let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Line Bind Group"),
                layout: &self.line_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                }],
            });

            // Draw
            {
                let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Line Draw Pass"),
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

                rpass.set_pipeline(&self.line_pipeline);
                rpass.set_bind_group(0, &bind_group, &[]);
                rpass.set_vertex_buffer(0, vertex_buffer.slice(..));
                rpass.draw(0..vertices.len() as u32, 0..1);
            }
        }

        // Process scatter batches
        if !batches.markers.is_empty() {
            // Update global uniforms (viewport size)
            let uniforms = ScatterGlobalUniforms {
                viewport_size: [self.size.width as f32, self.size.height as f32],
            };
            let uniform_buffer =
                self.device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Scatter Global Uniforms"),
                        contents: bytemuck::bytes_of(&uniforms),
                        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    });

            let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Scatter Bind Group"),
                layout: &self.scatter_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                }],
            });

            // Common quad vertices for the marker geometry
            let strip_vertices = [
                ScatterVertex {
                    position: [-0.5, 0.5],
                }, // TL
                ScatterVertex {
                    position: [-0.5, -0.5],
                }, // BL
                ScatterVertex {
                    position: [0.5, 0.5],
                }, // TR
                ScatterVertex {
                    position: [0.5, -0.5],
                }, // BR
            ];

            let vertex_buffer = self
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
                    self.device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("Scatter Instance Buffer"),
                            contents: bytemuck::cast_slice(&instances),
                            usage: wgpu::BufferUsages::VERTEX,
                        });

                {
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

                    rpass.set_pipeline(&self.scatter_pipeline);
                    rpass.set_bind_group(0, &bind_group, &[]);
                    rpass.set_vertex_buffer(0, vertex_buffer.slice(..));
                    rpass.set_vertex_buffer(1, instance_buffer.slice(..));
                    rpass.draw(0..4, 0..instances.len() as u32);
                }
            }
        }

        self.text_renderer
            .draw(encoder, view, !batches.texts.is_empty());
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

        self.queue.submit(Some(encoder.finish()));
        self.window.pre_present_notify();
        texture.present();
    }

    fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.text_renderer.resize(width, height, &self.queue);
            // self.is_surface_configured = true;
        }
        self.size = winit::dpi::PhysicalSize { width, height };
        self.configure_surface();
    }
}
