use crate::backend::WgpuBackend;
use pltrs_core::{Color, Figure, RenderBackend};
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::{KeyEvent, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::PhysicalKey,
    dpi::PhysicalSize,
    window::{Window, WindowAttributes, WindowId},
};

#[derive(Default)]
pub struct App {
    pub backend: Option<WgpuBackend>,
    pub figure: Option<Figure>,
    pub clear: Color,
    pub init_error: Option<String>,
}

impl App {
    pub fn new(figure: Option<Figure>) -> Self {
        let clear = figure
            .as_ref()
            .map(|fig| fig.clear_color)
            .unwrap_or(Color::WHITE);
        Self {
            backend: None,
            figure,
            clear,
            init_error: None,
        }
    }

    fn window_attributes(&self) -> WindowAttributes {
        let mut attrs = Window::default_attributes();
        if let Some(fig) = &self.figure {
            attrs = attrs.with_inner_size(PhysicalSize::new(fig.size.width, fig.size.height));
        }
        attrs
    }
}

impl ApplicationHandler for App {
    /// Handles the winit application lifecycle, window events, and connects the window to the backend.
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(event_loop.create_window(self.window_attributes()).unwrap());
        match pollster::block_on(WgpuBackend::new(window.clone())) {
            Ok(backend) => {
                self.backend = Some(backend);
                window.request_redraw();
            }
            Err(err) => {
                self.init_error = Some(err.to_string());
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let state = match &mut self.backend {
            Some(canvas) => canvas,
            None => return,
        };
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => {
                state.begin_frame(self.clear);
                if let Some(fig) = &self.figure {
                    state.draw_scene(fig);
                }
                state.end_frame();
            }
            WindowEvent::Resized(size) => {
                state.resize(size.width, size.height);
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state: key_state,
                        ..
                    },
                ..
            } => state.handle_key(event_loop, code, key_state.is_pressed()),
            _ => {}
        }
    }
}
