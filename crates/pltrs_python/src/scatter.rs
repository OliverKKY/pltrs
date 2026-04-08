use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyIterator;

use pltrs_backend_wgpu::{run_with_plot, save_figure_png};
use pltrs_core::{plot::PlotDefinition, scene::Marker, Color};

use crate::data::{
    compute_limits, extract_rgb, parse_series_collection, resolve_numeric_arg, try_extract_rgb,
};
use crate::plot::{build_plot_definition, scatter_series, PlotOptions};
use crate::{
    next_figure_id, register_handle, resolve_output_path, take_registered_handle, PlotHandle,
};

/// A lazy scatter-plot descriptor.
///
/// Captures data and configuration at construction time.
/// No rendering happens until `.show()` is called.
#[pyclass(name = "Scatter")]
pub struct PyScatter {
    id: u64,
    plot: PlotDefinition,
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
    /// title, x_label, y_label : str, optional
    ///     Plot title and axis labels.
    /// grid : bool, optional
    ///     Draw background grid lines and labeled axes. Enabled by default.
    #[new]
    #[pyo3(signature = (data, *, x=None, y=None, color=None, size=None, marker=None, annotations=None, title=None, x_label=None, y_label=None, grid=true))]
    fn new(
        data: &Bound<'_, PyAny>,
        x: Option<(f64, f64)>,
        y: Option<(f64, f64)>,
        color: Option<&Bound<'_, PyAny>>,
        size: Option<&Bound<'_, PyAny>>,
        marker: Option<&Bound<'_, PyAny>>,
        annotations: Option<Vec<(f64, f64, String)>>,
        title: Option<String>,
        x_label: Option<String>,
        y_label: Option<String>,
        grid: bool,
    ) -> PyResult<Self> {
        let series = parse_series_collection(data)?;
        let all_xs: Vec<f64> = series
            .iter()
            .flat_map(|series| series.xs.iter().copied())
            .collect();
        let all_ys: Vec<f64> = series
            .iter()
            .flat_map(|series| series.ys.iter().copied())
            .collect();

        let xlim = x.unwrap_or_else(|| compute_limits(&all_xs, 0.05));
        let ylim = y.unwrap_or_else(|| compute_limits(&all_ys, 0.05));

        let colors = resolve_scatter_colors(color, series.len())?;
        let sizes = resolve_numeric_arg(size, series.len(), 15.0, "size")?;
        let markers = resolve_markers(marker, series.len())?;
        let id = next_figure_id();

        let plot = build_plot_definition(
            PlotOptions {
                xlim,
                ylim,
                annotations: annotations.unwrap_or_default(),
                title,
                x_label,
                y_label,
                grid,
            },
            series
                .into_iter()
                .zip(colors)
                .zip(sizes)
                .zip(markers)
                .map(|(((series, (r, g, b)), marker_size), marker_shape)| {
                    scatter_series(
                        series.xs,
                        series.ys,
                        Color { r, g, b, a: 0.9 },
                        marker_size,
                        marker_shape,
                    )
                })
                .collect(),
        );

        register_handle(id, PlotHandle::Plot(plot.clone()));

        Ok(Self { id, plot })
    }

    /// Render this figure in a window.
    fn show(&self) -> PyResult<()> {
        let plot = match take_registered_handle(self.id) {
            Some(PlotHandle::Plot(plot)) => plot,
            Some(PlotHandle::Figure(_)) => self.plot.clone(),
            None => self.plot.clone(),
        };
        run_with_plot(plot).map_err(|e| PyRuntimeError::new_err(format!("{e}")))
    }

    /// Render this figure offscreen and save it as a PNG.
    fn save(&self, path: &str) -> PyResult<()> {
        let plot = match take_registered_handle(self.id) {
            Some(PlotHandle::Plot(plot)) => plot,
            Some(PlotHandle::Figure(_)) => self.plot.clone(),
            None => self.plot.clone(),
        };
        let fig = plot.build_figure(&plot.initial_view());
        let output_path = resolve_output_path(path);
        save_figure_png(&fig, &output_path).map_err(|e| PyRuntimeError::new_err(format!("{e}")))
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
                .map_err(|_| {
                    PyRuntimeError::new_err("color must be an RGB tuple or iterable of RGB tuples")
                })?
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

fn resolve_markers(
    marker: Option<&Bound<'_, PyAny>>,
    series_count: usize,
) -> PyResult<Vec<Marker>> {
    match marker {
        None => Ok(vec![Marker::Circle; series_count]),
        Some(obj) => {
            if let Ok(single) = obj.extract::<String>() {
                let marker = parse_marker(&single)?;
                return Ok(vec![marker; series_count]);
            }

            let markers = PyIterator::from_object(obj)
                .map_err(|_| {
                    PyValueError::new_err("marker must be a string or iterable of strings")
                })?
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
