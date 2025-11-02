use pltrs_core::{Color, Figure, RenderBackend};
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

pub struct WgpuBackend {
    window: Arc<Window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface<'static>,
    surface_format: wgpu::TextureFormat,
}

impl WgpuBackend {
    async fn new(window: Arc<Window>) -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await
            .unwrap();

        let size = window.inner_size();
        let surface = instance.create_surface(window.clone()).unwrap();
        let cap = surface.get_capabilities(&adapter);
        let surface_format = cap.formats[0];

        let backend = Self {
            window,
            device,
            queue,
            size,
            surface,
            surface_format,
        };
        backend.configure_surface();
        backend
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

    fn render_green(&mut self, color: Color) {
        let surface_texture = self
            .surface
            .get_current_texture()
            .expect("failed to acquire surface texture");
        let texture_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                format: Some(self.surface_format.add_srgb_suffix()),
                ..Default::default()
            });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let _rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: color.r as f64,
                            g: color.g as f64,
                            b: color.b as f64,
                            a: color.a as f64,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }
        self.queue.submit([encoder.finish()]);
        self.window.pre_present_notify();
        surface_texture.present();
    }
}

impl RenderBackend for WgpuBackend {
    fn begin_frame(&mut self, clear: Color) {
        self.render_green(clear);
    }

    fn draw_scene(&mut self, fig: &Figure) {
        let batches = pltrs_core::build_batches(fig);

        for line_batch in &batches.lines {
            // This is where your wgpu-specific code will go:
            println!(
                "Drawing a line batch with {} vertices.",
                line_batch.vertices.len()
            );
        }
    }
    fn end_frame(&mut self) {
        // nothing yet
    }
    fn resize(&mut self, width: u32, height: u32) {
        self.size = winit::dpi::PhysicalSize { width, height };
        self.configure_surface();
    }
}

#[derive(Default)]
struct App {
    backend: Option<WgpuBackend>,
    figure: Option<Figure>,
    clear: Color,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes())
                .unwrap(),
        );
        let backend = pollster::block_on(WgpuBackend::new(window.clone()));
        self.clear = Color::WHITE;
        self.backend = Some(backend);
        window.request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let backend = self.backend.as_mut().unwrap();
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => {
                backend.begin_frame(self.clear);
                if let Some(fig) = &self.figure {
                    backend.draw_scene(fig);
                }
                backend.end_frame();
            }
            WindowEvent::Resized(size) => {
                backend.resize(size.width, size.height);
            }
            _ => {}
        }
    }
}

/// Public entry: run a simple loop with an optional scene.
pub fn run_with_figure(fig: Option<Figure>) -> anyhow::Result<()> {
    env_logger::init();
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::default();
    app.figure = fig;
    event_loop.run_app(&mut app)?;
    Ok(())
}
