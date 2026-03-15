use pltrs_core::Figure;
use winit::event_loop::{ControlFlow, EventLoop};

mod app;
mod backend;
mod vertex;

pub use app::App;
pub use backend::WgpuBackend;

/// Public entry: run a simple loop with an optional scene.
pub fn run_with_figure(fig: Option<Figure>) -> anyhow::Result<()> {
    let _ = env_logger::try_init();
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::new(fig);
    event_loop.run_app(&mut app)?;
    if let Some(err) = app.init_error.take() {
        return Err(anyhow::anyhow!(err));
    }
    Ok(())
}
