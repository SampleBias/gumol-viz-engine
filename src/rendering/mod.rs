//! Rendering systems and mesh generation

pub mod instanced;

use bevy::{
    prelude::*,
    render::{
        mesh::{Indices, PrimitiveTopology},
        render_resource::ShaderType,
    },
};

const RENDER_ASSET_USAGES: bevy::render::render_asset::RenderAssetUsages =
    bevy::render::render_asset::RenderAssetUsages::RENDER_WORLD;

// ============================================================================
// INSTANCED RENDERING COMPONENTS
// ============================================================================

/// Instance data for each atom (sent to GPU for instanced rendering)
#[derive(ShaderType, Clone, Copy, Default, Debug, PartialEq)]
pub struct AtomInstanceData {
    /// Atom position in world space
    pub position: Vec3,
    /// Scale factor (multiplied with base mesh radius)
    pub scale: f32,
    /// Atom color (RGBA)
    pub color: Vec4,
    /// Padding for 16-byte alignment
    pub _padding: Vec3,
}

/// Component holding instance data for instanced atom rendering
#[derive(Component, Default, Debug)]
pub struct InstancedAtomMesh {
    /// All instances of atoms of this element type
    pub instances: Vec<AtomInstanceData>,
}

// ============================================================================
// MESH GENERATION
// ============================================================================

/// Register all rendering systems
pub fn register(_app: &mut App) {
    info!("Rendering module registered");
}

// Placeholder for atom mesh generation
pub fn generate_atom_mesh(radius: f32) -> Mesh {
    // Create a sphere manually for now
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RENDER_ASSET_USAGES);
    // Simple UV sphere implementation
    let latitudes = 16;
    let longitudes = 32;
    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();

    for i in 0..=latitudes {
        let theta = (std::f32::consts::PI * i as f32) / latitudes as f32;
        let sin_theta = theta.sin();
        let cos_theta = theta.cos();

        for j in 0..=longitudes {
            let phi = (2.0 * std::f32::consts::PI * j as f32) / longitudes as f32;
            let sin_phi = phi.sin();
            let cos_phi = phi.cos();

            let x = radius * sin_theta * cos_phi;
            let y = radius * cos_theta;
            let z = radius * sin_theta * sin_phi;

            vertices.push([x, y, z]);
            let normal = Vec3::new(x, y, z).normalize();
            normals.push([normal.x, normal.y, normal.z]);
            uvs.push([j as f32 / longitudes as f32, i as f32 / latitudes as f32]);
        }
    }

    for i in 0..latitudes {
        for j in 0..longitudes {
            let first = i * (longitudes + 1) + j;
            let second = first + longitudes + 1;

            indices.push(first as u32);
            indices.push(second as u32);
            indices.push(first + 1 as u32);

            indices.push(second as u32);
            indices.push(second + 1 as u32);
            indices.push(first + 1 as u32);
        }
    }

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));

    mesh
}

// Placeholder for bond mesh generation
pub fn generate_bond_mesh(length: f32, radius: f32) -> Mesh {
    // Create a cylinder manually for now
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RENDER_ASSET_USAGES);
    let segments = 16;
    let half_length = length / 2.0;

    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();

    // Top and bottom circles
    for i in 0..=segments {
        let theta = (2.0 * std::f32::consts::PI * i as f32) / segments as f32;
        let cos_theta = theta.cos();
        let sin_theta = theta.sin();

        // Top circle
        vertices.push([radius * cos_theta, half_length, radius * sin_theta]);
        normals.push([cos_theta, 0.0, sin_theta]);

        // Bottom circle
        vertices.push([radius * cos_theta, -half_length, radius * sin_theta]);
        normals.push([cos_theta, 0.0, sin_theta]);
    }

    // Side faces
    for i in 0..segments {
        let i1 = i * 2;
        let i2 = (i + 1) * 2;

        indices.push(i1 as u32);
        indices.push(i2 as u32);
        indices.push(i1 + 1 as u32);

        indices.push(i2 as u32);
        indices.push(i2 + 1 as u32);
        indices.push(i1 + 1 as u32);
    }

    // Top cap
    let top_center = vertices.len();
    vertices.push([0.0, half_length, 0.0]);
    normals.push([0.0, 1.0, 0.0]);

    for i in 0..segments {
        indices.push(top_center as u32);
        indices.push(((i + 1) * 2) as u32);
        indices.push((i * 2) as u32);
    }

    // Bottom cap
    let bottom_center = vertices.len();
    vertices.push([0.0, -half_length, 0.0]);
    normals.push([0.0, -1.0, 0.0]);

    for i in 0..segments {
        indices.push(bottom_center as u32);
        indices.push((i * 2 + 1) as u32);
        indices.push(((i + 1) * 2 + 1) as u32);
    }

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_indices(Indices::U32(indices));

    mesh
}
