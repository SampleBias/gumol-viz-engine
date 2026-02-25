//! Mesh geometry generation for 3D export
//!
//! Generates sphere and cylinder vertices/indices for OBJ and glTF export.
//! Matches the rendering module's geometry for consistency.

use bevy::prelude::*;

/// Sphere resolution for export (lower than rendering for smaller files)
const SPHERE_LATITUDES: usize = 12;
const SPHERE_LONGITUDES: usize = 24;
const CYLINDER_SEGMENTS: usize = 12;

/// Generate sphere vertices and indices centered at origin.
/// Returns (vertices, indices) - vertices in local space.
pub fn generate_sphere_mesh(radius: f32) -> (Vec<[f32; 3]>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for i in 0..=SPHERE_LATITUDES {
        let theta = (std::f32::consts::PI * i as f32) / SPHERE_LATITUDES as f32;
        let sin_theta = theta.sin();
        let cos_theta = theta.cos();

        for j in 0..=SPHERE_LONGITUDES {
            let phi = (2.0 * std::f32::consts::PI * j as f32) / SPHERE_LONGITUDES as f32;
            let sin_phi = phi.sin();
            let cos_phi = phi.cos();

            let x = radius * sin_theta * cos_phi;
            let y = radius * cos_theta;
            let z = radius * sin_theta * sin_phi;

            vertices.push([x, y, z]);
        }
    }

    for i in 0..SPHERE_LATITUDES {
        for j in 0..SPHERE_LONGITUDES {
            let first = (i * (SPHERE_LONGITUDES + 1) + j) as u32;
            let second = first + (SPHERE_LONGITUDES + 1) as u32;

            indices.extend_from_slice(&[first, second, first + 1]);
            indices.extend_from_slice(&[second, second + 1, first + 1]);
        }
    }

    (vertices, indices)
}

/// Generate cylinder vertices and indices along Y axis, centered at origin.
/// Length is along Y, radius is the cylinder radius.
pub fn generate_cylinder_mesh(length: f32, radius: f32) -> (Vec<[f32; 3]>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let half_length = length / 2.0;

    for i in 0..=CYLINDER_SEGMENTS {
        let theta = (2.0 * std::f32::consts::PI * i as f32) / CYLINDER_SEGMENTS as f32;
        let cos_theta = theta.cos();
        let sin_theta = theta.sin();

        vertices.push([radius * cos_theta, half_length, radius * sin_theta]);
        vertices.push([radius * cos_theta, -half_length, radius * sin_theta]);
    }

    for i in 0..CYLINDER_SEGMENTS {
        let i1 = i * 2;
        let i2 = (i + 1) * 2;

        indices.extend_from_slice(&[i1 as u32, i2 as u32, (i1 + 1) as u32]);
        indices.extend_from_slice(&[i2 as u32, (i2 + 1) as u32, (i1 + 1) as u32]);
    }

    let top_center = vertices.len() as u32;
    vertices.push([0.0, half_length, 0.0]);
    for i in 0..CYLINDER_SEGMENTS {
        indices.extend_from_slice(&[
            top_center,
            ((i + 1) * 2) as u32,
            (i * 2) as u32,
        ]);
    }

    let bottom_center = vertices.len() as u32;
    vertices.push([0.0, -half_length, 0.0]);
    for i in 0..CYLINDER_SEGMENTS {
        indices.extend_from_slice(&[
            bottom_center,
            (i * 2 + 1) as u32,
            ((i + 1) * 2 + 1) as u32,
        ]);
    }

    (vertices, indices)
}

/// Transform a vertex by position and rotation
pub fn transform_vertex(vertex: [f32; 3], translation: Vec3, rotation: Quat) -> [f32; 3] {
    let v = Vec3::from(vertex);
    let transformed = rotation * v + translation;
    [transformed.x, transformed.y, transformed.z]
}
