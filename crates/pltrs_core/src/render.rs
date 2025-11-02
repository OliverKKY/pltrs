use crate::scene::{Color, Figure, Node};

pub struct RenderTargetDesc {
    pub width: u32,
    pub height: u32,
    pub dpi: f32,
}

/// The `RenderBackend` trait is the contract that any rendering engine must follow.
pub trait RenderBackend {
    fn begin_frame(&mut self, clear: Color);
    fn draw_scene(&mut self, fig: &Figure);
    fn end_frame(&mut self);
    fn resize(&mut self, width: u32, height: u32);
}

/// --- Batching System ---
#[derive(Debug)]
pub struct LineBatch {
    pub vertices: Vec<[f32; 2]>,
    pub color: Color,
    pub width: f32,
}

#[derive(Debug, Default)]
pub struct Batches {
    pub lines: Vec<LineBatch>,
    // pub markers: Vec<MarkerBatch>, // TODO
    // pub bars: Vec<BarBatch>,       // TODO
}

pub fn build_batches(fig: &Figure) -> Batches {
    let mut batches = Batches::default();

    for axes in &fig.axes {
        let axes_rect = axes.rect;

        for node in &axes.children {
            match node {
                Node::Line(line) => {
                    let mut vertices = Vec::with_capacity(line.xs.len());
                    for (&x, &y) in line.xs.iter().zip(&line.ys) {
                        // Step 1: Map from data space to normalized axes space ([0, 1])
                        let x_norm_axes = axes.x.map(x) as f32;
                        let y_norm_axes = axes.y.map(y) as f32;

                        // Step 2: Map from normalized axes space to normalized figure space ([0, 1])
                        let x_norm_fig = axes_rect.x + axes_rect.w * x_norm_axes;
                        let y_norm_fig = axes_rect.y + axes_rect.h * y_norm_axes;

                        vertices.push([x_norm_fig, y_norm_fig]);
                    }

                    batches.lines.push(LineBatch {
                        vertices,
                        color: line.color,
                        width: line.width,
                    });
                }
                Node::Scatter(_) => {
                    // TODO
                }
                Node::Bar(_) => {
                    // TODO
                }
            }
        }
    }
    batches
}
