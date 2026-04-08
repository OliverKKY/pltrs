use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Mutex,
};

use pltrs_backend_wgpu::{run_with_figure, run_with_plot};
use pltrs_core::{
    plot::PlotDefinition,
    scale::Scale,
    scene::{Axes, Line, Node, Rect},
    Color, Figure, Size,
};
use pyo3::prelude::*;

mod bar;
mod data;
mod line;
mod plot;
mod scatter;

/// Global registry of figures created by `Line(...)`, `Scatter(...)`, etc.
/// Calling `pltrs.show()` renders all of them in sequence and clears the registry.
#[derive(Clone)]
pub enum PlotHandle {
    Figure(Figure),
    Plot(PlotDefinition),
}

pub struct RegisteredFigure {
    pub id: u64,
    pub handle: PlotHandle,
}

pub static FIGURE_REGISTRY: Mutex<Vec<RegisteredFigure>> = Mutex::new(Vec::new());
static NEXT_FIGURE_ID: AtomicU64 = AtomicU64::new(1);

pub fn next_figure_id() -> u64 {
    NEXT_FIGURE_ID.fetch_add(1, Ordering::Relaxed)
}

pub fn register_handle(id: u64, handle: PlotHandle) {
    let mut reg = FIGURE_REGISTRY.lock().unwrap();
    reg.push(RegisteredFigure { id, handle });
}

pub fn take_registered_handle(id: u64) -> Option<PlotHandle> {
    let mut reg = FIGURE_REGISTRY.lock().unwrap();
    let idx = reg.iter().position(|entry| entry.id == id)?;
    Some(reg.swap_remove(idx).handle)
}

pub fn drain_registered_handles() -> Vec<PlotHandle> {
    let mut reg = FIGURE_REGISTRY.lock().unwrap();
    reg.drain(..).map(|entry| entry.handle).collect()
}

pub fn resolve_output_path(path: &str) -> PathBuf {
    if path == "~" {
        return std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(path));
    }

    if let Some(stripped) = path.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join(stripped);
        }
    }

    PathBuf::from(path)
}

/// Render all queued figures in sequence, then clear the registry.
///
/// Each figure is displayed in its own window. Close the window (or press
/// Escape) to proceed to the next figure.
#[pyfunction]
fn show() -> PyResult<()> {
    let figures = drain_registered_handles();

    if figures.is_empty() {
        // Nothing queued — open an empty window (original behaviour).
        return run_with_figure(None)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")));
    }

    for fig in figures {
        match fig {
            PlotHandle::Figure(fig) => run_with_figure(Some(fig))
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))?,
            PlotHandle::Plot(plot) => run_with_plot(plot)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))?,
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Legacy demo helpers (kept for backward compatibility)
// ---------------------------------------------------------------------------

#[pyfunction]
fn demo_line() -> PyResult<()> {
    let size = Size {
        width: 800,
        height: 600,
        dpi: 1.0,
    };
    let mut fig = Figure::new(size);
    let rect = Rect {
        x: 0.1,
        y: 0.1,
        w: 0.8,
        h: 0.8,
    };
    let xscale = Scale::linear((0.0, 10.0), (0.0, 1.0));
    let yscale = Scale::linear((0.0, 1.0), (0.0, 1.0));
    let mut ax = Axes::new(rect, xscale, yscale);

    let xs: Vec<f64> = (0..101).map(|i| i as f64 / 10.0).collect();
    let ys: Vec<f64> = xs.iter().map(|x| (0.5 * x).sin() * 0.5 + 0.5).collect();

    let line = Line {
        xs,
        ys,
        color: Color {
            r: 0.1,
            g: 0.2,
            b: 0.8,
            a: 1.0,
        },
        width: 2.0,
    };
    ax.add(Node::Line(line));
    fig.add_axes(ax);

    run_with_figure(Some(fig))
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
}

#[pyfunction]
fn demo_scatter() -> PyResult<()> {
    let size = Size {
        width: 800,
        height: 600,
        dpi: 1.0,
    };
    let mut fig = Figure::new(size);
    let rect = Rect {
        x: 0.1,
        y: 0.1,
        w: 0.8,
        h: 0.8,
    };
    let xscale = Scale::linear((0.0, 1.0), (0.0, 1.0));
    let yscale = Scale::linear((0.0, 1.0), (0.0, 1.0));
    let mut ax = Axes::new(rect, xscale, yscale);

    let count = 50;
    let xs: Vec<f64> = (0..count).map(|i| i as f64 / count as f64).collect();
    let ys: Vec<f64> = xs.iter().map(|x| (x * 6.28).sin() * 0.4 + 0.5).collect();

    use pltrs_core::scene::{Marker, Scatter};
    let scatter = Scatter {
        xs,
        ys,
        color: Color {
            r: 0.8,
            g: 0.2,
            b: 0.1,
            a: 0.8,
        },
        size: 20.0,
        marker: Marker::Circle,
    };
    ax.add(Node::Scatter(scatter));

    let xs2: Vec<f64> = (0..count).map(|i| i as f64 / count as f64).collect();
    let ys2: Vec<f64> = xs2.iter().map(|x| (x * 6.28).cos() * 0.4 + 0.5).collect();
    let scatter2 = Scatter {
        xs: xs2,
        ys: ys2,
        color: Color {
            r: 0.1,
            g: 0.8,
            b: 0.2,
            a: 0.8,
        },
        size: 15.0,
        marker: Marker::Square,
    };
    ax.add(Node::Scatter(scatter2));

    fig.add_axes(ax);

    run_with_figure(Some(fig))
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
}

// ---------------------------------------------------------------------------
// Module registration
// ---------------------------------------------------------------------------

#[pymodule]
fn pltrs(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // New API classes
    m.add_class::<bar::PyBar>()?;
    m.add_class::<line::PyLine>()?;
    m.add_class::<scatter::PyScatter>()?;

    // Functions
    m.add_function(wrap_pyfunction!(show, m)?)?;

    // Legacy demos
    m.add_function(wrap_pyfunction!(demo_line, m)?)?;
    m.add_function(wrap_pyfunction!(demo_scatter, m)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pltrs_core::Size;
    use pyo3::types::IntoPyDict;

    #[test]
    fn take_registered_figure_consumes_only_matching_entry() {
        let id_a = next_figure_id();
        let id_b = next_figure_id();
        register_handle(
            id_a,
            PlotHandle::Figure(Figure::new(Size {
                width: 1,
                height: 1,
                dpi: 1.0,
            })),
        );
        register_handle(
            id_b,
            PlotHandle::Figure(Figure::new(Size {
                width: 2,
                height: 2,
                dpi: 1.0,
            })),
        );

        let taken = take_registered_handle(id_a).unwrap();
        match taken {
            PlotHandle::Figure(fig) => assert_eq!(fig.size.width, 1),
            PlotHandle::Plot(_) => panic!("expected a static figure"),
        }

        let remaining = drain_registered_handles();
        assert_eq!(remaining.len(), 1);
        match &remaining[0] {
            PlotHandle::Figure(fig) => assert_eq!(fig.size.width, 2),
            PlotHandle::Plot(_) => panic!("expected a static figure"),
        }
    }

    #[test]
    fn drain_registered_figures_preserves_multi_series_line_figure() {
        Python::attach(|py| {
            let module = PyModule::new(py, "pltrs_test").unwrap();
            module.add_class::<bar::PyBar>().unwrap();
            module.add_class::<line::PyLine>().unwrap();

            let locals = [("pltrs_test", module)].into_py_dict(py).unwrap();
            py.run(
                pyo3::ffi::c_str!(
                    r#"
fig = pltrs_test.Line(
    [[0.0, 1.0, 0.5], [1.0, 0.25, 0.75]],
    color=[(0.1, 0.2, 0.8), (0.8, 0.2, 0.1)],
    width=[2.0, 4.0],
    title="Multi-series",
    x_label="sample",
    y_label="value",
)
"#
                ),
                None,
                Some(&locals),
            )
            .unwrap();

            let figures = drain_registered_handles();
            assert_eq!(figures.len(), 1);
            match &figures[0] {
                PlotHandle::Plot(plot) => {
                    let fig = plot.build_figure(&plot.initial_view());
                    assert_eq!(fig.axes.len(), 2);
                    assert!(fig.axes[0].children.len() >= 2);
                    assert!(
                        fig.axes[1]
                            .children
                            .iter()
                            .any(|node| matches!(node, pltrs_core::Node::Text(text) if text.content == "Multi-series"))
                    );
                }
                PlotHandle::Figure(_) => panic!("expected an interactive plot"),
            }
        });
    }

    #[test]
    fn bar_registers_bar_nodes_for_multiple_series() {
        Python::attach(|py| {
            let module = PyModule::new(py, "pltrs_test").unwrap();
            module.add_class::<bar::PyBar>().unwrap();

            let locals = [("pltrs_test", module)].into_py_dict(py).unwrap();
            py.run(
                pyo3::ffi::c_str!(
                    r#"
fig = pltrs_test.Bar(
    [[1.0, 2.0, 3.0], [1.5, 1.0, 2.5]],
    title="Bars",
    x_label="category",
    y_label="value",
)
"#
                ),
                None,
                Some(&locals),
            )
            .unwrap();

            let figures = drain_registered_handles();
            assert_eq!(figures.len(), 1);
            match &figures[0] {
                PlotHandle::Plot(plot) => {
                    let fig = plot.build_figure(&plot.initial_view());
                    assert_eq!(fig.axes.len(), 2);
                    let bar_count = fig.axes[0]
                        .children
                        .iter()
                        .filter(|node| matches!(node, pltrs_core::Node::Bar(_)))
                        .count();
                    assert_eq!(bar_count, 2);
                }
                PlotHandle::Figure(_) => panic!("expected an interactive plot"),
            }
        });
    }
}
