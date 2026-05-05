use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::PyIterator;

use pltrs_backend_wgpu::{run_with_plot, save_figure_png};
use pltrs_core::{plot::PlotDefinition, Color};

use crate::data::{
    compute_limits, extract_rgb, parse_series_collection, resolve_numeric_arg, try_extract_rgb,
};
use crate::plot::{bar_series, build_plot_definition, PlotOptions};
use crate::{
    map_backend_error, next_figure_id, register_handle, resolve_output_path,
    take_registered_handle, PlotHandle,
};

#[pyclass(name = "Bar")]
pub struct PyBar {
    id: u64,
    plot: PlotDefinition,
}

#[pymethods]
impl PyBar {
    #[new]
    #[pyo3(signature = (data, *, x=None, y=None, color=None, width=None, annotations=None, title=None, x_label=None, y_label=None, grid=true))]
    fn new(
        data: &Bound<'_, PyAny>,
        x: Option<(f64, f64)>,
        y: Option<(f64, f64)>,
        color: Option<&Bound<'_, PyAny>>,
        width: Option<&Bound<'_, PyAny>>,
        annotations: Option<Vec<(f64, f64, String)>>,
        title: Option<String>,
        x_label: Option<String>,
        y_label: Option<String>,
        grid: bool,
    ) -> PyResult<Self> {
        let series = parse_series_collection(data)?;
        let series_count = series.len();
        let colors = resolve_bar_colors(color, series_count)?;
        let widths = resolve_bar_widths(width, &series, series_count)?;
        let shifted_series = shift_series_for_grouping(series, &widths);

        let all_xs: Vec<f64> = shifted_series
            .iter()
            .zip(widths.iter())
            .flat_map(|(series, width)| {
                series
                    .xs
                    .iter()
                    .flat_map(move |x| [*x - *width as f64 * 0.5, *x + *width as f64 * 0.5])
            })
            .collect();
        let mut all_ys: Vec<f64> = shifted_series
            .iter()
            .flat_map(|series| series.ys.iter().copied())
            .collect();
        all_ys.push(0.0);

        let xlim = x.unwrap_or_else(|| compute_limits(&all_xs, 0.05));
        let ylim = y.unwrap_or_else(|| compute_limits(&all_ys, 0.05));
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
            shifted_series
                .into_iter()
                .zip(colors)
                .zip(widths.iter().copied())
                .map(|((series, (r, g, b)), bar_width)| {
                    bar_series(series.xs, series.ys, Color { r, g, b, a: 0.92 }, bar_width)
                })
                .collect(),
        );

        register_handle(id, PlotHandle::Plot(plot.clone()));
        Ok(Self { id, plot })
    }

    fn show(&self) -> PyResult<()> {
        let plot = match take_registered_handle(self.id) {
            Some(PlotHandle::Plot(plot)) => plot,
            Some(PlotHandle::Figure(_)) => self.plot.clone(),
            None => self.plot.clone(),
        };
        run_with_plot(plot).map_err(map_backend_error)
    }

    #[pyo3(signature = (path=None))]
    fn save(&self, py: Python<'_>, path: Option<&str>) -> PyResult<()> {
        let plot = match take_registered_handle(self.id) {
            Some(PlotHandle::Plot(plot)) => plot,
            Some(PlotHandle::Figure(_)) => self.plot.clone(),
            None => self.plot.clone(),
        };
        let fig = plot.build_figure(&plot.initial_view());
        let output_path = resolve_output_path(py, path)?;
        save_figure_png(&fig, &output_path).map_err(|e| PyRuntimeError::new_err(format!("{e}")))
    }
}

fn resolve_bar_colors(
    color: Option<&Bound<'_, PyAny>>,
    series_count: usize,
) -> PyResult<Vec<(f32, f32, f32)>> {
    let defaults = [
        (0.18, 0.42, 0.85),
        (0.88, 0.4, 0.22),
        (0.16, 0.65, 0.38),
        (0.72, 0.52, 0.16),
        (0.52, 0.3, 0.78),
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

fn resolve_bar_widths(
    width: Option<&Bound<'_, PyAny>>,
    series: &[crate::data::SeriesData],
    series_count: usize,
) -> PyResult<Vec<f32>> {
    let inferred = infer_group_span(series) / series_count.max(1) as f64 * 0.9;
    resolve_numeric_arg(width, series_count, inferred as f32, "width")
}

fn infer_group_span(series: &[crate::data::SeriesData]) -> f64 {
    let mut xs: Vec<f64> = series
        .iter()
        .flat_map(|series| series.xs.iter().copied())
        .collect();
    xs.sort_by(f64::total_cmp);
    xs.dedup_by(|a, b| (*a - *b).abs() < f64::EPSILON);

    let min_gap = xs
        .windows(2)
        .map(|pair| (pair[1] - pair[0]).abs())
        .filter(|gap| *gap > f64::EPSILON)
        .min_by(f64::total_cmp)
        .unwrap_or(1.0);

    min_gap * 0.8
}

fn shift_series_for_grouping(
    series: Vec<crate::data::SeriesData>,
    widths: &[f32],
) -> Vec<crate::data::SeriesData> {
    let series_count = series.len();

    series
        .into_iter()
        .enumerate()
        .map(|(idx, mut series)| {
            let width = widths[idx] as f64;
            let offset = (idx as f64 - (series_count as f64 - 1.0) * 0.5) * width;
            for x in &mut series.xs {
                *x += offset;
            }
            series
        })
        .collect()
}
