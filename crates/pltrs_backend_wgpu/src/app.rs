use crate::{backend::WgpuBackend, KEYBOARD_INTERRUPT_ERROR};
use pltrs_core::{Color, Figure, PlotDefinition, PlotView, RenderBackend};
use pyo3::{ffi, Python};
use std::{env, sync::Arc, time::Instant};
use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalPosition, PhysicalSize},
    event::{KeyEvent, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes, WindowId},
};

enum AppContent {
    Static(Option<Figure>),
    Interactive(PlotDefinition),
}

struct DragState {
    last_cursor: PhysicalPosition<f64>,
}

pub struct App {
    pub backend: Option<WgpuBackend>,
    pub clear: Color,
    pub init_error: Option<String>,
    content: AppContent,
    interactive_view: Option<PlotView>,
    cursor_position: Option<PhysicalPosition<f64>>,
    middle_drag: Option<DragState>,
    window_size: PhysicalSize<u32>,
    benchmark_start: Option<Instant>,
    benchmark_oneshot: bool,
}

impl App {
    pub fn new(figure: Option<Figure>) -> Self {
        let clear = figure
            .as_ref()
            .map(|fig| fig.clear_color)
            .unwrap_or(Color::WHITE);
        let window_size = figure
            .as_ref()
            .map(|fig| PhysicalSize::new(fig.size.width, fig.size.height))
            .unwrap_or_else(|| PhysicalSize::new(800, 600));

        Self {
            backend: None,
            clear,
            init_error: None,
            content: AppContent::Static(figure),
            interactive_view: None,
            cursor_position: None,
            middle_drag: None,
            window_size,
            benchmark_start: benchmark_enabled().then(Instant::now),
            benchmark_oneshot: benchmark_enabled(),
        }
    }

    pub fn new_interactive(plot: PlotDefinition) -> Self {
        let clear = plot.clear_color;
        let window_size = PhysicalSize::new(plot.size.width, plot.size.height);
        let interactive_view = Some(plot.initial_view());

        Self {
            backend: None,
            clear,
            init_error: None,
            content: AppContent::Interactive(plot),
            interactive_view,
            cursor_position: None,
            middle_drag: None,
            window_size,
            benchmark_start: benchmark_enabled().then(Instant::now),
            benchmark_oneshot: benchmark_enabled(),
        }
    }

    fn window_attributes(&self) -> WindowAttributes {
        let mut attrs = Window::default_attributes().with_title("pltrs plot");
        match &self.content {
            AppContent::Static(Some(fig)) => {
                attrs = attrs.with_inner_size(PhysicalSize::new(fig.size.width, fig.size.height));
            }
            AppContent::Interactive(plot) => {
                attrs = attrs.with_inner_size(PhysicalSize::new(plot.size.width, plot.size.height));
            }
            AppContent::Static(None) => {}
        }
        attrs
    }

    fn current_figure(&self) -> Option<Figure> {
        match (&self.content, self.interactive_view) {
            (AppContent::Static(fig), _) => fig.clone(),
            (AppContent::Interactive(plot), Some(view)) => Some(plot.build_figure(&view)),
            (AppContent::Interactive(_), None) => None,
        }
    }

    fn interactive_plot(&self) -> Option<&PlotDefinition> {
        match &self.content {
            AppContent::Interactive(plot) => Some(plot),
            AppContent::Static(_) => None,
        }
    }

    fn handle_zoom(&mut self, delta: MouseScrollDelta) -> bool {
        let Some(plot) = self.interactive_plot() else {
            return false;
        };
        let Some(cursor) = self.cursor_position else {
            return false;
        };
        let Some(plot_pos) = plot.plot_normalized_position(
            (cursor.x, cursor.y),
            (self.window_size.width, self.window_size.height),
        ) else {
            return false;
        };

        let amount = match delta {
            MouseScrollDelta::LineDelta(_, y) => y as f64,
            MouseScrollDelta::PixelDelta(pos) => pos.y / 40.0,
        };
        if amount.abs() <= f64::EPSILON {
            return false;
        }

        if let Some(view) = &mut self.interactive_view {
            view.zoom_at(plot_pos, 0.9_f64.powf(amount));
            return true;
        }

        false
    }

    fn start_drag(&mut self) {
        let Some(plot) = self.interactive_plot() else {
            return;
        };
        let Some(cursor) = self.cursor_position else {
            return;
        };
        if plot
            .plot_normalized_position(
                (cursor.x, cursor.y),
                (self.window_size.width, self.window_size.height),
            )
            .is_some()
        {
            self.middle_drag = Some(DragState {
                last_cursor: cursor,
            });
        }
    }

    fn update_drag(&mut self, position: PhysicalPosition<f64>) -> bool {
        let Some(plot_rect) = self.interactive_plot().map(|plot| plot.plot_rect) else {
            return false;
        };
        let Some(drag) = &mut self.middle_drag else {
            return false;
        };
        if self.window_size.width == 0 || self.window_size.height == 0 {
            return false;
        }

        let dx = (position.x - drag.last_cursor.x) / self.window_size.width as f64;
        let dy = -(position.y - drag.last_cursor.y) / self.window_size.height as f64;
        drag.last_cursor = position;

        if let Some(view) = &mut self.interactive_view {
            view.pan_by((dx / plot_rect.w as f64, dy / plot_rect.h as f64));
            return true;
        }

        false
    }

    fn reset_view(&mut self) -> bool {
        let Some(plot) = self.interactive_plot() else {
            return false;
        };
        self.interactive_view = Some(plot.initial_view());
        self.middle_drag = None;
        true
    }

    fn exit_if_interrupted(&mut self, event_loop: &ActiveEventLoop) {
        let interrupted = Python::attach(|_| unsafe { ffi::PyErr_CheckSignals() != 0 });
        if interrupted {
            self.init_error = Some(KEYBOARD_INTERRUPT_ERROR.to_string());
            event_loop.exit();
        }
    }
}

impl ApplicationHandler for App {
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
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => {
                let fig = self.current_figure();
                let Some(state) = &mut self.backend else {
                    return;
                };
                state.begin_frame(self.clear);
                if let Some(fig) = fig {
                    state.draw_scene(&fig);
                }
                state.end_frame();
                if self.benchmark_oneshot {
                    if let Some(start) = self.benchmark_start.take() {
                        println!(
                            "PLTRS_BENCHMARK_MS={:.3}",
                            start.elapsed().as_secs_f64() * 1_000.0
                        );
                    }
                    event_loop.exit();
                }
            }
            WindowEvent::Resized(size) => {
                let Some(state) = &mut self.backend else {
                    return;
                };
                self.window_size = size;
                state.resize(size.width, size.height);
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_position = Some(position);
                if self.update_drag(position) {
                    if let Some(state) = &self.backend {
                        state.request_redraw();
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                if self.handle_zoom(delta) {
                    if let Some(state) = &self.backend {
                        state.request_redraw();
                    }
                }
            }
            WindowEvent::MouseInput {
                state: button_state,
                button: MouseButton::Middle,
                ..
            } => {
                if button_state.is_pressed() {
                    self.start_drag();
                } else {
                    self.middle_drag = None;
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state: key_state,
                        ..
                    },
                ..
            } => {
                if key_state.is_pressed() && matches!(code, KeyCode::KeyR | KeyCode::Home) {
                    if self.reset_view() {
                        if let Some(state) = &self.backend {
                            state.request_redraw();
                        }
                    }
                }
                if let Some(state) = &self.backend {
                    state.handle_key(event_loop, code, key_state.is_pressed())
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.exit_if_interrupted(event_loop);
    }
}

fn benchmark_enabled() -> bool {
    matches!(env::var("PLTRS_BENCHMARK_ONESHOT").as_deref(), Ok("1"))
}
