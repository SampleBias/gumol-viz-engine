// GPU compute: linear interpolation between two trajectory frames.
// out[i] = mix(pos_a[i], pos_b[i], alpha)

struct InterpolationUniforms {
    alpha: f32,
    num_atoms: u32,
    _padding: vec2<f32>,
}

@group(0) @binding(0) var<storage, read> positions_a: array<vec3<f32>>;
@group(0) @binding(1) var<storage, read> positions_b: array<vec3<f32>>;
@group(0) @binding(2) var<storage, read_write> positions_out: array<vec3<f32>>;
@group(0) @binding(3) var<uniform> uniforms: InterpolationUniforms;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let atom_idx = global_id.x;
    if (atom_idx >= uniforms.num_atoms) {
        return;
    }

    let pos_a = positions_a[atom_idx];
    let pos_b = positions_b[atom_idx];
    positions_out[atom_idx] = mix(pos_a, pos_b, uniforms.alpha);
}
