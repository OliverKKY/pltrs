use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;

use pltrs_backend_wgpu::run_with_figure;
use pltrs_core::{
    scale::Scale,
    scene::{Axes, Line, Node, Rect, Text},
    Color, Figure, Size,
};

use crate::data::{compute_limits, parse_data};
use crate::{next_figure_id, register_figure, take_registered_figure};

/// A lazy line-plot descriptor.
///
/// Captures data and configuration at construction time.
/// No rendering happens until `.show()` is called.
#[pyclass(name = "Line")]
pub struct PyLine {
    id: u64,
    figure: Figure,
}

#[pymethods]
impl PyLine {
    /// Create a new line plot.
    ///
    /// Parameters
    /// ----------
    /// data : list
    ///     Data as a list of `(x, y)` pairs, or a 1-D list of y-values
    ///     (x is inferred as `0, 1, 2, …`).
    /// x : tuple(float, float), optional
    ///     Explicit x-axis range `(min, max)`. Inferred from data if omitted.
    /// y : tuple(float, float), optional
    ///     Explicit y-axis range `(min, max)`. Inferred from data if omitted.
    /// color : tuple(float, float, float), optional
    ///     RGB color as `(r, g, b)` with values in `[0, 1]`. Defaults to blue.
    /// width : float, optional
    ///     Line width in pixels. Defaults to `2.0`.
    /// annotations : list[tuple(float, float, str)], optional
    ///     Text labels given as `(x, y, label)` in data coordinates.
    #[new]
    #[pyo3(signature = (data, *, x=None, y=None, color=None, width=None, annotations=None))]
    fn new(
        data: &Bound<'_, PyAny>,
        x: Option<(f64, f64)>,
        y: Option<(f64, f64)>,
        color: Option<(f32, f32, f32)>,
        width: Option<f32>,
        annotations: Option<Vec<(f64, f64, String)>>,
    ) -> PyResult<Self> {
        let (xs, ys) = parse_data(data)?;

        let xlim = x.unwrap_or_else(|| compute_limits(&xs, 0.05));
        let ylim = y.unwrap_or_else(|| compute_limits(&ys, 0.05));

        let (r, g, b) = color.unwrap_or((0.1, 0.2, 0.8));
        let line_width = width.unwrap_or(9.0);
        let id = next_figure_id();

        // Build a single-axes figure.
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
        let xscale = Scale::linear(xlim, (0.0, 1.0));
        let yscale = Scale::linear(ylim, (0.0, 1.0));
        let mut ax = Axes::new(rect, xscale, yscale);

        let line = Line {
            xs,
            ys,
            color: Color { r, g, b, a: 1.0 },
            width: line_width,
        };
        ax.add(Node::Line(line));

        for (x, y, content) in annotations.unwrap_or_default() {
            ax.add(Node::Text(Text {
                content,
                x,
                y,
                color: Color::BLACK,
                size: 18.0,
            }));
        }

        fig.add_axes(ax);

        // Register for pltrs.show().
        register_figure(id, fig.clone());

        Ok(Self { id, figure: fig })
    }

    /// Render this figure in a window.
    fn show(&self) -> PyResult<()> {
        let fig = take_registered_figure(self.id).unwrap_or_else(|| self.figure.clone());
        run_with_figure(Some(fig)).map_err(|e| PyRuntimeError::new_err(format!("{e}")))
    }
}
