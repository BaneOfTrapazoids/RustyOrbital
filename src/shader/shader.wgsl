
struct VertexInput {
    @location(0) pos: vec3<f32>,
    @location(1) colour: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) colour: vec3<f32>,
}

struct Camera {
  proj_view : mat4x4<f32>,
};
@group(0) @binding(0) var<uniform> camera : Camera;

@vertex
fn vs_main_triangle(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = camera.proj_view * vec4<f32>(in.pos, 1.0);
    out.colour = in.colour;
    return out;
}

@fragment
fn fs_main_triangle(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.colour, 1.0);
}



@vertex
fn vs_main_line(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    return out;
}

@fragment
fn fs_main_line(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(0.3, 0.2, 0.1, 1.0);
}



@vertex
fn vs_main_point(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    return out;
}

@fragment
fn fs_main_point(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(0.3, 0.2, 0.1, 1.0);
}