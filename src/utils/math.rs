//! Math utility functions for molecular dynamics

use bevy::prelude::*;
use nalgebra::{Vector3, Matrix3};
use std::f32::consts::PI;

/// Apply periodic boundary conditions to a position
pub fn apply_pbc(mut pos: Vec3, box_size: Vec3) -> Vec3 {
    pos.x = pos.x.rem_euclid(box_size.x);
    pos.y = pos.y.rem_euclid(box_size.y);
    pos.z = pos.z.rem_euclid(box_size.z);
    pos
}

/// Calculate minimum image distance between two positions with PBC
pub fn minimum_image(pos_a: Vec3, pos_b: Vec3, box_size: Vec3) -> Vec3 {
    let mut delta = pos_b - pos_a;
    delta.x -= box_size.x * (delta.x / box_size.x).round();
    delta.y -= box_size.y * (delta.y / box_size.y).round();
    delta.z -= box_size.z * (delta.z / box_size.z).round();
    delta
}

/// Convert degrees to radians
pub fn deg_to_rad(degrees: f32) -> f32 {
    degrees * (PI / 180.0)
}

/// Convert radians to degrees
pub fn rad_to_deg(radians: f32) -> f32 {
    radians * (180.0 / PI)
}

/// Lerp (linear interpolation) between two values
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Smooth step interpolation
pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Calculate RMSD (Root Mean Square Deviation) between two sets of coordinates
pub fn calculate_rmsd(coords_a: &[Vec3], coords_b: &[Vec3]) -> f32 {
    if coords_a.len() != coords_b.len() || coords_a.is_empty() {
        return 0.0;
    }

    let sum_sq: f32 = coords_a
        .iter()
        .zip(coords_b.iter())
        .map(|(a, b)| (*a - *b).length_squared())
        .sum();

    (sum_sq / coords_a.len() as f32).sqrt()
}

/// Calculate center of mass for a set of positions
pub fn center_of_mass(positions: &[Vec3], masses: &[f32]) -> Vec3 {
    if positions.is_empty() || masses.is_empty() {
        return Vec3::ZERO;
    }

    let total_mass: f32 = masses.iter().sum();
    let center = positions
        .iter()
        .zip(masses.iter())
        .map(|(pos, mass)| *pos * *mass)
        .sum::<Vec3>()
        / total_mass;

    center
}

/// Calculate Kabsch rotation matrix to align two structures
pub fn kabsch_rotation(coords_a: &[Vec3], coords_b: &[Vec3]) -> Option<Matrix3<f32>> {
    if coords_a.len() != coords_b.len() || coords_a.len() < 3 {
        return None;
    }

    // Calculate centers of mass
    let masses = vec![1.0; coords_a.len()];
    let center_a = center_of_mass(coords_a, &masses);
    let center_b = center_of_mass(coords_b, &masses);

    // Center the coordinates
    let centered_a: Vec<Vector3<f32>> = coords_a.iter().map(|p| {
        let diff = *p - center_a;
        Vector3::new(diff.x, diff.y, diff.z)
    }).collect();
    let centered_b: Vec<Vector3<f32>> = coords_b.iter().map(|p| {
        let diff = *p - center_b;
        Vector3::new(diff.x, diff.y, diff.z)
    }).collect();

    // Compute covariance matrix
    let mut cov = Matrix3::zeros();
    for (a, b) in centered_a.iter().zip(centered_b.iter()) {
        cov += a * b.transpose();
    }

    // Compute SVD
    let svd = cov.svd(true, true);
    let u = svd.u?;
    let v_t = svd.v_t?;

    // Compute rotation matrix
    let rotation = v_t.transpose() * u.transpose();

    // Ensure proper rotation (determinant = 1)
    let det = rotation.determinant();
    if det < 0.0 {
        let mut v_t_corrected = v_t;
        v_t_corrected.set_row(2, &(-v_t_corrected.row(2)));
        return Some(v_t_corrected.transpose() * u.transpose());
    }

    Some(rotation)
}

/// Interpolate between two positions
pub fn interpolate_position(pos_a: Vec3, pos_b: Vec3, t: f32) -> Vec3 {
    Vec3::lerp(pos_a, pos_b, t)
}

/// Interpolate between multiple frames using cubic spline
pub fn spline_interpolation(frames: &[Vec<Vec3>], frame_index: usize, t: f32) -> Vec<Vec3> {
    if frames.is_empty() {
        return Vec::new();
    }

    if frame_index >= frames.len() - 1 {
        return frames.last().unwrap().clone();
    }

    let pos_prev = if frame_index > 0 { frames[frame_index - 1].clone() } else { frames[frame_index].clone() };
    let pos_curr = frames[frame_index].clone();
    let pos_next = frames[frame_index + 1].clone();
    let pos_next_next = if frame_index + 2 < frames.len() { frames[frame_index + 2].clone() } else { frames[frame_index + 1].clone() };

    pos_curr
        .iter()
        .zip(pos_next.iter())
        .enumerate()
        .map(|(i, (a, b))| {
            let pa = if i < pos_prev.len() { pos_prev[i] } else { *a };
            let pb = if i < pos_next_next.len() { pos_next_next[i] } else { *b };

            let t2 = t * t;
            let t3 = t2 * t;

            // Catmull-Rom spline
            0.5 * ((2.0 * *a)
                 + (-pa + *b) * t
                 + (2.0 * pa - 5.0 * *a + 4.0 * *b - pb) * t2
                 + (-pa + 3.0 * *a - 3.0 * *b + pb) * t3)
        })
        .collect()
}
