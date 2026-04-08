use pltrs_core::{
    plot::{PlotDefinition, PlotSeries},
    scene::{Bar, Color, Line, Marker, Rect, Scatter, Size, Text},
};

pub struct PlotOptions {
    pub xlim: (f64, f64),
    pub ylim: (f64, f64),
    pub annotations: Vec<(f64, f64, String)>,
    pub title: Option<String>,
    pub x_label: Option<String>,
    pub y_label: Option<String>,
    pub grid: bool,
}

pub fn default_figure_size() -> Size {
    Size {
        width: 800,
        height: 600,
        dpi: 1.0,
    }
}

pub fn plot_rect() -> Rect {
    Rect {
        x: 0.16,
        y: 0.18,
        w: 0.74,
        h: 0.68,
    }
}

pub fn build_plot_definition(options: PlotOptions, series: Vec<PlotSeries>) -> PlotDefinition {
    PlotDefinition {
        size: default_figure_size(),
        clear_color: Color::WHITE,
        plot_rect: plot_rect(),
        base_xlim: options.xlim,
        base_ylim: options.ylim,
        title: options.title,
        x_label: options.x_label,
        y_label: options.y_label,
        grid: options.grid,
        annotations: options
            .annotations
            .into_iter()
            .map(|(x, y, content)| Text {
                content,
                x,
                y,
                color: Color::BLACK,
                size: 18.0,
            })
            .collect(),
        series,
    }
}

pub fn line_series(xs: Vec<f64>, ys: Vec<f64>, color: Color, width: f32) -> PlotSeries {
    PlotSeries::Line(Line {
        xs,
        ys,
        color,
        width,
    })
}

pub fn scatter_series(
    xs: Vec<f64>,
    ys: Vec<f64>,
    color: Color,
    size: f32,
    marker: Marker,
) -> PlotSeries {
    PlotSeries::Scatter(Scatter {
        xs,
        ys,
        color,
        size,
        marker,
    })
}

pub fn bar_series(xs: Vec<f64>, heights: Vec<f64>, color: Color, width: f32) -> PlotSeries {
    PlotSeries::Bar(Bar {
        xs,
        heights,
        width,
        color,
    })
}
