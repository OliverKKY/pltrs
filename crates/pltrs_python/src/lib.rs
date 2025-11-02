use pltrs_backend_wgpu::run_with_figure;
use pltrs_core::{
    scale::Scale,
    scene::{Axes, Line, Node, Rect},
    Color, Figure, Size,
};
use pyo3::prelude::*;

#[pyfunction]
fn show() -> PyResult<()> {
    run_with_figure(None)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{e}")))
}

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

#[pymodule]
fn pltrs(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(show, m)?)?;
    m.add_function(wrap_pyfunction!(demo_line, m)?)?;
    Ok(())
}
