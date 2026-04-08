use crate::{
    scale::Scale,
    scene::{Axes, Color, Figure, Line, Node, Rect, Scatter, Size, Text},
};

const DEFAULT_TICK_COUNT: usize = 6;

#[derive(Clone, Debug)]
pub enum PlotSeries {
    Line(Line),
    Scatter(Scatter),
    Bar(crate::scene::Bar),
}

#[derive(Clone, Debug)]
pub struct PlotDefinition {
    pub size: Size,
    pub clear_color: Color,
    pub plot_rect: Rect,
    pub base_xlim: (f64, f64),
    pub base_ylim: (f64, f64),
    pub title: Option<String>,
    pub x_label: Option<String>,
    pub y_label: Option<String>,
    pub grid: bool,
    pub annotations: Vec<Text>,
    pub series: Vec<PlotSeries>,
}

#[derive(Clone, Copy, Debug)]
pub struct PlotView {
    pub xlim: (f64, f64),
    pub ylim: (f64, f64),
}

impl PlotDefinition {
    pub fn initial_view(&self) -> PlotView {
        PlotView {
            xlim: self.base_xlim,
            ylim: self.base_ylim,
        }
    }

    pub fn build_figure(&self, view: &PlotView) -> Figure {
        let mut fig = Figure::new(self.size);
        fig.clear_color = self.clear_color;

        let xscale = Scale::linear(view.xlim, (0.0, 1.0));
        let yscale = Scale::linear(view.ylim, (0.0, 1.0));
        let mut plot_axes = Axes::new(self.plot_rect, xscale, yscale);
        add_plot_frame(&mut plot_axes, view.xlim, view.ylim, self.grid);

        for annotation in &self.annotations {
            plot_axes.add(Node::Text(annotation.clone()));
        }

        for series in &self.series {
            let node = match series {
                PlotSeries::Line(line) => Node::Line(line.clone()),
                PlotSeries::Scatter(scatter) => Node::Scatter(scatter.clone()),
                PlotSeries::Bar(bar) => Node::Bar(bar.clone()),
            };
            plot_axes.add(node);
        }

        let mut overlay_axes = Axes::new(
            Rect {
                x: 0.0,
                y: 0.0,
                w: 1.0,
                h: 1.0,
            },
            Scale::linear((0.0, 1.0), (0.0, 1.0)),
            Scale::linear((0.0, 1.0), (0.0, 1.0)),
        );
        add_tick_labels(
            &mut overlay_axes,
            self.plot_rect,
            view.xlim,
            view.ylim,
            self.size,
        );
        add_axis_labels(
            &mut overlay_axes,
            self.plot_rect,
            self.title.as_deref(),
            self.x_label.as_deref(),
            self.y_label.as_deref(),
            self.size,
        );

        fig.add_axes(plot_axes);
        fig.add_axes(overlay_axes);
        fig
    }

    pub fn plot_normalized_position(
        &self,
        cursor_px: (f64, f64),
        window_size: (u32, u32),
    ) -> Option<(f64, f64)> {
        let (width, height) = window_size;
        if width == 0 || height == 0 {
            return None;
        }

        let x = cursor_px.0 / width as f64;
        let y = 1.0 - cursor_px.1 / height as f64;

        let x0 = self.plot_rect.x as f64;
        let x1 = (self.plot_rect.x + self.plot_rect.w) as f64;
        let y0 = self.plot_rect.y as f64;
        let y1 = (self.plot_rect.y + self.plot_rect.h) as f64;

        if x < x0 || x > x1 || y < y0 || y > y1 {
            return None;
        }

        Some((
            (x - x0) / self.plot_rect.w as f64,
            (y - y0) / self.plot_rect.h as f64,
        ))
    }
}

impl PlotView {
    pub fn zoom_at(&mut self, anchor: (f64, f64), factor: f64) {
        if !(factor.is_finite() && factor > 0.0) {
            return;
        }

        let anchor_x = self.xlim.0 + (self.xlim.1 - self.xlim.0) * anchor.0;
        let anchor_y = self.ylim.0 + (self.ylim.1 - self.ylim.0) * anchor.1;

        self.xlim = zoom_range(self.xlim, anchor_x, factor);
        self.ylim = zoom_range(self.ylim, anchor_y, factor);
    }

    pub fn pan_by(&mut self, delta: (f64, f64)) {
        let dx = delta.0 * (self.xlim.1 - self.xlim.0);
        let dy = delta.1 * (self.ylim.1 - self.ylim.0);
        self.xlim = (self.xlim.0 - dx, self.xlim.1 - dx);
        self.ylim = (self.ylim.0 - dy, self.ylim.1 - dy);
    }
}

fn zoom_range(range: (f64, f64), anchor: f64, factor: f64) -> (f64, f64) {
    let min = anchor - (anchor - range.0) * factor;
    let max = anchor + (range.1 - anchor) * factor;
    if (max - min).abs() < f64::EPSILON {
        range
    } else {
        (min, max)
    }
}

fn add_plot_frame(axes: &mut Axes, xlim: (f64, f64), ylim: (f64, f64), grid: bool) {
    let frame_color = Color {
        r: 0.15,
        g: 0.18,
        b: 0.22,
        a: 1.0,
    };
    let grid_color = Color {
        r: 0.82,
        g: 0.84,
        b: 0.88,
        a: 1.0,
    };

    add_segment(axes, [xlim.0, xlim.1], [ylim.0, ylim.0], frame_color, 2.0);
    add_segment(axes, [xlim.0, xlim.1], [ylim.1, ylim.1], frame_color, 2.0);
    add_segment(axes, [xlim.0, xlim.0], [ylim.0, ylim.1], frame_color, 2.0);
    add_segment(axes, [xlim.1, xlim.1], [ylim.0, ylim.1], frame_color, 2.0);

    let x_ticks = generate_ticks(xlim, DEFAULT_TICK_COUNT);
    let y_ticks = generate_ticks(ylim, DEFAULT_TICK_COUNT);
    let x_tick_len = (ylim.1 - ylim.0) * 0.015;
    let y_tick_len = (xlim.1 - xlim.0) * 0.015;

    for tick in x_ticks {
        if grid {
            add_segment(axes, [tick, tick], [ylim.0, ylim.1], grid_color, 1.0);
        }
        add_segment(
            axes,
            [tick, tick],
            [ylim.0, ylim.0 + x_tick_len],
            frame_color,
            1.5,
        );
    }

    for tick in y_ticks {
        if grid {
            add_segment(axes, [xlim.0, xlim.1], [tick, tick], grid_color, 1.0);
        }
        add_segment(
            axes,
            [xlim.0, xlim.0 + y_tick_len],
            [tick, tick],
            frame_color,
            1.5,
        );
    }
}

fn add_tick_labels(axes: &mut Axes, rect: Rect, xlim: (f64, f64), ylim: (f64, f64), size: Size) {
    let label_color = Color {
        r: 0.2,
        g: 0.22,
        b: 0.27,
        a: 1.0,
    };
    let x_ticks = generate_ticks(xlim, DEFAULT_TICK_COUNT);
    let y_ticks = generate_ticks(ylim, DEFAULT_TICK_COUNT);

    for tick in x_ticks {
        let x = rect.x + rect.w * normalize_value(tick, xlim);
        let text = format_tick(tick);
        axes.add(Node::Text(Text {
            content: text.clone(),
            x: centered_text_x(&text, 16.0, x as f64, size.width),
            y: (rect.y - 0.065).max(0.02) as f64,
            color: label_color,
            size: 16.0,
        }));
    }

    for tick in y_ticks {
        let y = rect.y + rect.h * normalize_value(tick, ylim);
        let text = format_tick(tick);
        let label_x = (rect.x - estimate_text_width(&text, 16.0, size.width) - 0.02).max(0.01);
        axes.add(Node::Text(Text {
            content: text,
            x: label_x as f64,
            y: (y - 0.015).max(0.01) as f64,
            color: label_color,
            size: 16.0,
        }));
    }
}

fn add_axis_labels(
    axes: &mut Axes,
    rect: Rect,
    title: Option<&str>,
    x_label: Option<&str>,
    y_label: Option<&str>,
    size: Size,
) {
    let label_color = Color {
        r: 0.08,
        g: 0.1,
        b: 0.14,
        a: 1.0,
    };

    if let Some(title) = title.filter(|value| !value.trim().is_empty()) {
        axes.add(Node::Text(Text {
            content: title.to_string(),
            x: centered_text_x(title, 24.0, (rect.x + rect.w * 0.5) as f64, size.width),
            y: (rect.y + rect.h + 0.08).min(0.96) as f64,
            color: label_color,
            size: 24.0,
        }));
    }

    if let Some(label) = x_label.filter(|value| !value.trim().is_empty()) {
        axes.add(Node::Text(Text {
            content: label.to_string(),
            x: centered_text_x(label, 20.0, (rect.x + rect.w * 0.5) as f64, size.width),
            y: (rect.y - 0.12).max(0.02) as f64,
            color: label_color,
            size: 20.0,
        }));
    }

    if let Some(label) = y_label.filter(|value| !value.trim().is_empty()) {
        axes.add(Node::Text(Text {
            content: label.to_string(),
            x: 0.02,
            y: (rect.y + rect.h * 0.5) as f64,
            color: label_color,
            size: 20.0,
        }));
    }
}

fn add_segment(axes: &mut Axes, xs: [f64; 2], ys: [f64; 2], color: Color, width: f32) {
    axes.add(Node::Line(Line {
        xs: xs.to_vec(),
        ys: ys.to_vec(),
        color,
        width,
    }));
}

fn normalize_value(value: f64, limits: (f64, f64)) -> f32 {
    let span = limits.1 - limits.0;
    if span.abs() <= f64::EPSILON {
        return 0.0;
    }
    ((value - limits.0) / span) as f32
}

fn nice_number(value: f64, round: bool) -> f64 {
    let exponent = value.log10().floor();
    let fraction = value / 10_f64.powf(exponent);

    let nice_fraction = if round {
        if fraction < 1.5 {
            1.0
        } else if fraction < 3.0 {
            2.0
        } else if fraction < 7.0 {
            5.0
        } else {
            10.0
        }
    } else if fraction <= 1.0 {
        1.0
    } else if fraction <= 2.0 {
        2.0
    } else if fraction <= 5.0 {
        5.0
    } else {
        10.0
    };

    nice_fraction * 10_f64.powf(exponent)
}

fn generate_ticks(limits: (f64, f64), target_count: usize) -> Vec<f64> {
    let (min, max) = limits;
    let range = (max - min).abs();
    if range <= f64::EPSILON {
        return vec![min];
    }

    let rough_step = range / (target_count.saturating_sub(1).max(1) as f64);
    let step = nice_number(rough_step, true);
    let start = (min / step).ceil() * step;
    let end = (max / step).floor() * step;

    let mut ticks = Vec::new();
    let mut value = start;
    while value <= end + step * 0.5 {
        ticks.push((value / step).round() * step);
        value += step;
    }

    if !ticks.iter().any(|tick| (*tick - min).abs() < 1e-9) {
        ticks.insert(0, min);
    }
    if !ticks.iter().any(|tick| (*tick - max).abs() < 1e-9) {
        ticks.push(max);
    }

    ticks
}

fn format_tick(value: f64) -> String {
    let rounded = value.round();
    if (value - rounded).abs() < 1e-9 {
        return format!("{}", rounded as i64);
    }

    let abs = value.abs();
    if abs >= 1000.0 || (abs > 0.0 && abs < 0.01) {
        format!("{value:.2e}")
    } else {
        let mut text = format!("{value:.3}");
        if text.contains('.') {
            while text.ends_with('0') {
                text.pop();
            }
            if text.ends_with('.') {
                text.pop();
            }
        }
        text
    }
}

fn estimate_text_width(text: &str, size: f32, figure_width: u32) -> f32 {
    let width_px = text.chars().count() as f32 * size * 0.38;
    width_px / figure_width as f32
}

fn centered_text_x(text: &str, size: f32, center_x: f64, figure_width: u32) -> f64 {
    (center_x as f32 - estimate_text_width(text, size, figure_width) * 0.5).max(0.01) as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plot_view_zoom_preserves_anchor() {
        let mut view = PlotView {
            xlim: (0.0, 10.0),
            ylim: (0.0, 20.0),
        };
        view.zoom_at((0.25, 0.5), 0.5);
        assert_eq!(view.xlim, (1.25, 6.25));
        assert_eq!(view.ylim, (5.0, 15.0));
    }

    #[test]
    fn plot_view_pan_translates_ranges() {
        let mut view = PlotView {
            xlim: (0.0, 10.0),
            ylim: (0.0, 20.0),
        };
        view.pan_by((0.1, -0.25));
        assert_eq!(view.xlim, (-1.0, 9.0));
        assert_eq!(view.ylim, (5.0, 25.0));
    }
}
