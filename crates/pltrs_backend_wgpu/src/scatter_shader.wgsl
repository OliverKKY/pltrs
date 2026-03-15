struct VertexInput {
    @location(0) position: vec2<f32>,
}

struct InstanceInput {
    @location(1) instance_pos: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) size: f32, // Size in pixels (radius for circle, side for square)
    @location(4) marker_type: u32, // 0 = Circle, 1 = Square
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>, // Local coordinates [-1, 1] for shaping
    @location(2) @interpolate(flat) marker_type: u32,
}

struct Uniforms {
    viewport_size: vec2<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;

    // model.position is a quad vertex [-0.5, 0.5]
    // scale determines the size in pixels
    let scale = instance.size;

    // Convert pixel size to normalized device coordinates (NDC)
    // NDC ranges from -1 to 1, length 2.
    // viewport_size is in pixels.
    let aspect = uniforms.viewport_size.x / uniforms.viewport_size.y;

    let pixel_offset = model.position * scale;

    let ndc_offset_x = (pixel_offset.x / uniforms.viewport_size.x) * 2.0;
    let ndc_offset_y = (pixel_offset.y / uniforms.viewport_size.y) * 2.0;

    // Instance position is in normalized figure coords [0, 1]
    // Convert to NDC [-1, 1]
    let base_ndc_x = instance.instance_pos.x * 2.0 - 1.0;
    let base_ndc_y = instance.instance_pos.y * 2.0 - 1.0;

    out.clip_position = vec4<f32>(
        base_ndc_x + ndc_offset_x,
        base_ndc_y + ndc_offset_y,
        0.0,
        1.0
    );

    out.color = instance.color;
    out.uv = model.position * 2.0; // Map [-0.5, 0.5] to [-1, 1] for SDF
    out.marker_type = instance.marker_type;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    if (in.marker_type == 0u) {
        // Circle SDF
        let dist = length(in.uv);
        let alpha = 1.0 - smoothstep(0.85, 1.0, dist);
        if (dist > 1.0) {
            discard;
        }
        return vec4<f32>(in.color.rgb, in.color.a * alpha);
    } else {
        // Square (just fill)
        return in.color;
    }
}
