use crate::scale::Scale;
use crate::scene::{Axes, Figure, Rect, Size};

pub struct LayoutParams {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

impl Default for LayoutParams {
    fn default() -> Self {
        Self {
            left: 60.0,
            right: 20.0,
            top: 20.0,
            bottom: 40.0,
        }
    }
}

pub fn single_axes(size: Size, xlim: (f64, f64), ylim: (f64, f64)) -> (Figure, usize) {
    let mut fig = Figure::new(size);
    let lp = LayoutParams::default();
    let w = size.width as f32;
    let h = size.height as f32;
    let rect = Rect {
        x: lp.left / w,
        y: lp.bottom / h,
        w: (w - lp.left - lp.right) / w,
        h: (h - lp.top - lp.bottom) / h,
    };

    let axes = Axes::new(
        rect,
        Scale::linear(xlim, (0.0, 1.0)),
        Scale::linear(ylim, (0.0, 1.0)),
    );

    fig.add_axes(axes);

    let idx = fig.axes.len() - 1;
    (fig, idx)
}
