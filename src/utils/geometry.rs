//! Geometry utility functions for molecular visualization

use bevy::prelude::*;

/// Calculate distance between two points
pub fn distance(pos_a: Vec3, pos_b: Vec3) -> f32 {
    (pos_a - pos_b).length()
}

/// Calculate angle between three points (in radians)
pub fn angle(pos_a: Vec3, pos_b: Vec3, pos_c: Vec3) -> f32 {
    let v1 = (pos_a - pos_b).normalize();
    let v2 = (pos_c - pos_b).normalize();
    v1.dot(v2).acos()
}

/// Calculate dihedral angle between four points (in radians)
pub fn dihedral(pos_a: Vec3, pos_b: Vec3, pos_c: Vec3, pos_d: Vec3) -> f32 {
    let b0 = pos_a - pos_b;
    let b1 = pos_c - pos_b;
    let b2 = pos_d - pos_c;

    let b1_x_b2 = b1.cross(b2);
    let b0_x_b1 = b0.cross(b1);

    let x = b0_x_b1.dot(b1_x_b2) / (b0_x_b1.length() * b1_x_b2.length());
    let y = b0.dot(b1_x_b2) / (b0_x_b1.length() * b1_x_b2.length());

    y.atan2(x)
}

/// Generate a sphere mesh for atoms
pub fn create_sphere_mesh(radius: f32, _resolution: u32) -> Mesh {
    // Placeholder: will be implemented properly in rendering module
    crate::rendering::generate_atom_mesh(radius)
}

/// Generate a cylinder mesh for bonds
pub fn create_cylinder_mesh(radius: f32, height: f32) -> Mesh {
    // Placeholder: will be implemented properly in rendering module
    crate::rendering::generate_bond_mesh(height, radius)
}
