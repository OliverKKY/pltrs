use crate::Scale;

#[derive(Clone, Copy, Debug)]
pub struct Size {
    pub width: u32,
    pub height: u32,
    pub dpi: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const GREEN: Color = Color {
        r: 0.0,
        g: 1.0,
        b: 0.0,
        a: 1.0,
    };
    pub const WHITE: Color = Color {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };
    pub const BLACK: Color = Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
}

impl Default for Color {
    fn default() -> Self {
        Self::WHITE
    }
}

#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct Figure {
    pub size: Size,
    pub axes: Vec<Axes>,
    pub clear_color: Color,
}

impl Figure {
    pub fn new(size: Size) -> Self {
        Self {
            size,
            axes: vec![],
            clear_color: Color::WHITE,
        }
    }

    pub fn add_axes(&mut self, axes: Axes) {
        self.axes.push(axes);
    }
}

#[derive(Clone, Debug)]
pub struct Axes {
    pub rect: Rect,
    pub x: Scale,
    pub y: Scale,
    pub children: Vec<Node>,
}

impl Axes {
    pub fn new(rect: Rect, x: Scale, y: Scale) -> Self {
        Self {
            rect,
            x,
            y,
            children: vec![],
        }
    }

    pub fn add(&mut self, node: Node) {
        self.children.push(node);
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Clone, Debug)]
pub enum Node {
    Line(Line),
    Scatter(Scatter),
    Bar(Bar),
    // Text(Text) // TODO
}

#[derive(Clone, Debug)]
pub struct Line {
    pub xs: Vec<f64>,
    pub ys: Vec<f64>,
    pub color: Color,
    pub width: f32,
}

#[derive(Clone, Debug)]
pub struct Scatter {
    pub xs: Vec<f64>,
    pub ys: Vec<f64>,
    pub color: Color,
    pub size: f32,
    pub marker: Marker,
}

#[derive(Clone, Copy, Debug)]
pub enum Marker {
    Circle,
    Square,
}

#[derive(Clone, Debug)]
pub struct Bar {
    pub xs: Vec<f64>,
    pub heights: Vec<f64>,
    pub width: f32,
    pub color: Color,
}
