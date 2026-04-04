use wgpu::vertex_attr_array;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LineVertex {
    pub position: [f32; 2],
}

impl LineVertex {
    pub const ATTRIBS: [wgpu::VertexAttribute; 1] = vertex_attr_array![0 => Float32x2];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<LineVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

/// Vertex for the quad geometry (instanced).
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ScatterVertex {
    pub position: [f32; 2],
}

impl ScatterVertex {
    pub const ATTRIBS: [wgpu::VertexAttribute; 1] = vertex_attr_array![0 => Float32x2];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<ScatterVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

/// Per-instance data for scatter points.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ScatterInstance {
    pub position: [f32; 2],
    pub color: [f32; 4],
    pub size: f32,
    pub marker_type: u32,
}

impl ScatterInstance {
    pub const ATTRIBS: [wgpu::VertexAttribute; 4] =
        vertex_attr_array![1 => Float32x2, 2 => Float32x4, 3 => Float32, 4 => Uint32];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<ScatterInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRIBS,
        }
    }
}
