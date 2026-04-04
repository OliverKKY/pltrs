use crate::scene::{Color, Figure, Marker, Node};

/// Description of the render target (window or texture).
pub struct RenderTargetDesc {
    pub width: u32,
    pub height: u32,
    pub dpi: f32,
}

/// The `RenderBackend` trait is the contract that any rendering engine must follow.
pub trait RenderBackend {
    /// Start a new frame.
    fn begin_frame(&mut self, clear: Color);
    /// Draw the scene.
    fn draw_scene(&mut self, fig: &Figure);
    /// End the current frame and present it.
    fn end_frame(&mut self);
    /// Resize the render surface.
    fn resize(&mut self, width: u32, height: u32);
}

/// --- Batching System ---

/// A batch of lines to be rendered.
#[derive(Debug)]
pub struct LineBatch {
    pub vertices: Vec<[f32; 2]>,
    pub color: Color,
    pub width: f32,
}

/// A batch of markers (scatter plot points) to be rendered.
#[derive(Debug)]
pub struct MarkerBatch {
    pub positions: Vec<[f32; 2]>,
    pub color: Color,
    pub size: f32,
    pub marker: Marker,
}

/// A batch of text labels to be rendered.
#[derive(Debug)]
pub struct TextBatch {
    pub content: String,
    pub position: [f32; 2],
    pub color: Color,
    pub size: f32,
}

/// Collection of renderable batches.
#[derive(Debug, Default)]
pub struct Batches {
    pub lines: Vec<LineBatch>,
    pub markers: Vec<MarkerBatch>,
    pub texts: Vec<TextBatch>,
    // pub bars: Vec<BarBatch>,       // TODO
}

/// Build renderable batches from the Scene Graph.
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
                Node::Scatter(scatter) => {
                    let mut positions = Vec::with_capacity(scatter.xs.len());
                    for (&x, &y) in scatter.xs.iter().zip(&scatter.ys) {
                        let x_norm_axes = axes.x.map(x) as f32;
                        let y_norm_axes = axes.y.map(y) as f32;

                        let x_norm_fig = axes_rect.x + axes_rect.w * x_norm_axes;
                        let y_norm_fig = axes_rect.y + axes_rect.h * y_norm_axes;

                        positions.push([x_norm_fig, y_norm_fig]);
                    }

                    batches.markers.push(MarkerBatch {
                        positions,
                        color: scatter.color,
                        size: scatter.size,
                        marker: scatter.marker,
                    });
                }
                Node::Bar(_) => {
                    // TODO
                }
                Node::Text(text) => {
                    let x_norm_axes = axes.x.map(text.x) as f32;
                    let y_norm_axes = axes.y.map(text.y) as f32;

                    let x_norm_fig = axes_rect.x + axes_rect.w * x_norm_axes;
                    let y_norm_fig = axes_rect.y + axes_rect.h * y_norm_axes;

                    batches.texts.push(TextBatch {
                        content: text.content.clone(),
                        position: [x_norm_fig, y_norm_fig],
                        color: text.color,
                        size: text.size,
                    });
                }
            }
        }
    }
    batches
}
