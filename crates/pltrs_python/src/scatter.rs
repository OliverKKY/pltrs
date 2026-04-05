use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyIterator;

use pltrs_backend_wgpu::{run_with_figure, save_figure_png};
use pltrs_core::{
    scale::Scale,
    scene::{Axes, Marker, Node, Rect, Scatter, Text},
    Color, Figure, Size,
};

use crate::data::{
    compute_limits, extract_rgb, parse_series_collection, resolve_numeric_arg, try_extract_rgb,
};
use crate::{next_figure_id, register_figure, take_registered_figure};

/// A lazy scatter-plot descriptor.
///
/// Captures data and configuration at construction time.
/// No rendering happens until `.show()` is called.
#[pyclass(name = "Scatter")]
pub struct PyScatter {
    id: u64,
    figure: Figure,
}

#[pymethods]
impl PyScatter {
    /// Create a new scatter plot.
    ///
    /// Parameters
    /// ----------
    /// data : list
    ///     Data as a single series (`[(x, y), ...]` or `[y0, y1, ...]`), or
    ///     a list of such series.
    /// x : tuple(float, float), optional
    ///     Explicit x-axis range `(min, max)`. Inferred across all series if omitted.
    /// y : tuple(float, float), optional
    ///     Explicit y-axis range `(min, max)`. Inferred across all series if omitted.
    /// color : tuple(float, float, float) or list[tuple(float, float, float)], optional
    ///     One RGB color or one per series.
    /// size : float or list[float], optional
    ///     One marker size or one per series.
    /// marker : str or list[str], optional
    ///     One marker shape or one per series.
    /// annotations : list[tuple(float, float, str)], optional
    ///     Text labels given as `(x, y, label)` in data coordinates.
    #[new]
    #[pyo3(signature = (data, *, x=None, y=None, color=None, size=None, marker=None, annotations=None))]
    fn new(
        data: &Bound<'_, PyAny>,
        x: Option<(f64, f64)>,
        y: Option<(f64, f64)>,
        color: Option<&Bound<'_, PyAny>>,
        size: Option<&Bound<'_, PyAny>>,
        marker: Option<&Bound<'_, PyAny>>,
        annotations: Option<Vec<(f64, f64, String)>>,
    ) -> PyResult<Self> {
        let series = parse_series_collection(data)?;
        let all_xs: Vec<f64> = series.iter().flat_map(|series| series.xs.iter().copied()).collect();
        let all_ys: Vec<f64> = series.iter().flat_map(|series| series.ys.iter().copied()).collect();

        let xlim = x.unwrap_or_else(|| compute_limits(&all_xs, 0.05));
        let ylim = y.unwrap_or_else(|| compute_limits(&all_ys, 0.05));

        let colors = resolve_scatter_colors(color, series.len())?;
        let sizes = resolve_numeric_arg(size, series.len(), 15.0, "size")?;
        let markers = resolve_markers(marker, series.len())?;
        let id = next_figure_id();

        // Build a single-axes figure.
        let fig_size = Size {
            width: 800,
            height: 600,
            dpi: 1.0,
        };
        let mut fig = Figure::new(fig_size);
        let rect = Rect {
            x: 0.1,
            y: 0.1,
            w: 0.8,
            h: 0.8,
        };
        let xscale = Scale::linear(xlim, (0.0, 1.0));
        let yscale = Scale::linear(ylim, (0.0, 1.0));
        let mut ax = Axes::new(rect, xscale, yscale);

        for (((series, (r, g, b)), marker_size), marker_shape) in
            series.into_iter().zip(colors).zip(sizes).zip(markers)
        {
            let scatter = Scatter {
                xs: series.xs,
                ys: series.ys,
                color: Color { r, g, b, a: 0.9 },
                size: marker_size,
                marker: marker_shape,
            };
            ax.add(Node::Scatter(scatter));
        }

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

    /// Render this figure offscreen and save it as a PNG.
    fn save(&self, path: &str) -> PyResult<()> {
        let fig = take_registered_figure(self.id).unwrap_or_else(|| self.figure.clone());
        save_figure_png(&fig, path).map_err(|e| PyRuntimeError::new_err(format!("{e}")))
    }
}

fn resolve_scatter_colors(
    color: Option<&Bound<'_, PyAny>>,
    series_count: usize,
) -> PyResult<Vec<(f32, f32, f32)>> {
    let defaults = [
        (0.8, 0.2, 0.1),
        (0.15, 0.45, 0.9),
        (0.1, 0.7, 0.3),
        (0.8, 0.55, 0.15),
        (0.55, 0.25, 0.75),
    ];

    match color {
        None => Ok((0..series_count)
            .map(|idx| defaults[idx % defaults.len()])
            .collect()),
        Some(obj) => {
            if let Some(rgb) = try_extract_rgb(obj)? {
                return Ok(vec![rgb; series_count]);
            }

            let colors = PyIterator::from_object(obj)
                .map_err(|_| PyRuntimeError::new_err("color must be an RGB tuple or iterable of RGB tuples"))?
                .map(|item| item.and_then(|item| extract_rgb(&item)))
                .collect::<PyResult<Vec<_>>>()?;

            if colors.len() != series_count {
                return Err(PyRuntimeError::new_err(format!(
                    "color expected 1 value or {series_count} values, got {}",
                    colors.len()
                )));
            }

            Ok(colors)
        }
    }
}

fn resolve_markers(marker: Option<&Bound<'_, PyAny>>, series_count: usize) -> PyResult<Vec<Marker>> {
    match marker {
        None => Ok(vec![Marker::Circle; series_count]),
        Some(obj) => {
            if let Ok(single) = obj.extract::<String>() {
                let marker = parse_marker(&single)?;
                return Ok(vec![marker; series_count]);
            }

            let markers = PyIterator::from_object(obj)
                .map_err(|_| PyValueError::new_err("marker must be a string or iterable of strings"))?
                .map(|item| {
                    let marker = item?
                        .extract::<String>()
                        .map_err(|_| PyValueError::new_err("marker values must be strings"))?;
                    parse_marker(&marker)
                })
                .collect::<PyResult<Vec<_>>>()?;

            if markers.len() != series_count {
                return Err(PyValueError::new_err(format!(
                    "marker expected 1 value or {series_count} values, got {}",
                    markers.len()
                )));
            }

            Ok(markers)
        }
    }
}

fn parse_marker(value: &str) -> PyResult<Marker> {
    match value {
        "circle" => Ok(Marker::Circle),
        "square" => Ok(Marker::Square),
        other => Err(PyValueError::new_err(format!(
            "unknown marker '{other}', expected 'circle' or 'square'"
        ))),
    }
}
