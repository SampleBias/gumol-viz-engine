#import bevy_pbr::mesh_functions::{get_world_from_local, mesh_position_local_to_clip}

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) i_pos: vec3<f32>,
    @location(4) i_scale: f32,
    @location(5) i_color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) normal: vec3<f32>,
};

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    let world_pos = vertex.position * vertex.i_scale + vertex.i_pos;
    var out: VertexOutput;
    out.clip_position = mesh_position_local_to_clip(
        get_world_from_local(vertex.instance_index),
        vec4<f32>(world_pos, 1.0),
    );
    out.normal = vertex.normal;
    out.color = vertex.i_color;
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let n = normalize(in.normal);
    let sun = normalize(vec3<f32>(1.0, 2.0, 1.5));
    let fill = normalize(vec3<f32>(-0.5, -0.3, -1.0));

    let ambient = 0.2;
    let diff_sun = max(dot(n, sun), 0.0) * 0.65;
    let diff_fill = max(dot(n, fill), 0.0) * 0.15;

    let light = ambient + diff_sun + diff_fill;
    return vec4<f32>(in.color.rgb * light, in.color.a);
}
