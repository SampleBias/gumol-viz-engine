//! Distance and angle measurement tools
//!
//! Computes measurements from selected atoms via instanced position data.

use crate::interaction::selection::SelectionState;
use crate::rendering::atom_index::InstancedAtomIndex;
use crate::rendering::instanced::{InstancedAtomEntity, InstancedAtomMesh};
use bevy::prelude::*;

/// Resource holding current measurement results for display
#[derive(Resource, Default, Debug, Clone)]
pub struct MeasurementState {
    /// Distance between 2 selected atoms (Å), if applicable
    pub distance: Option<f32>,
    /// Angle at middle of 3 selected atoms (degrees), if applicable
    pub angle: Option<f32>,
    /// Dihedral angle for 4 selected atoms (degrees), if applicable
    pub dihedral: Option<f32>,
}

/// Compute measurements from selected atom IDs and instanced positions.
pub fn compute_measurements(
    selection: Res<SelectionState>,
    index: Res<InstancedAtomIndex>,
    instanced: Query<(&InstancedAtomEntity, &InstancedAtomMesh)>,
    mut measurements: ResMut<MeasurementState>,
) {
    measurements.distance = None;
    measurements.angle = None;
    measurements.dihedral = None;

    let atom_ids = selection.atom_ids();
    if atom_ids.is_empty() || index.atom_to_instance.is_empty() {
        return;
    }

    let positions: Vec<Vec3> = atom_ids
        .iter()
        .filter_map(|id| index.get_position(*id, &instanced))
        .collect();

    match positions.len() {
        2 => {
            measurements.distance = Some(positions[0].distance(positions[1]));
        }
        3 => {
            let v01 = positions[0] - positions[1];
            let v21 = positions[2] - positions[1];
            measurements.angle = Some(v01.angle_between(v21).to_degrees());
        }
        4.. => {
            let b1 = positions[1] - positions[0];
            let b2 = positions[2] - positions[1];
            let b3 = positions[3] - positions[2];

            let n1 = b1.cross(b2);
            let n2 = b2.cross(b3);

            let n1_len = n1.length();
            let n2_len = n2.length();

            if n1_len > 1e-6 && n2_len > 1e-6 {
                let n1 = n1 / n1_len;
                let n2 = n2 / n2_len;
                let b2_norm = b2.normalize();
                let m = n1.cross(b2_norm);
                let x = n1.dot(n2);
                let y = m.dot(n2);
                measurements.dihedral = Some(y.atan2(x).to_degrees());
            }
        }
        _ => {}
    }
}

pub fn register(app: &mut App) {
    app.init_resource::<MeasurementState>()
        .add_systems(Update, compute_measurements);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_distance_between_two_points() {
        let a = Vec3::new(0.0, 0.0, 0.0);
        let b = Vec3::new(3.0, 4.0, 0.0);
        assert!((a.distance(b) - 5.0).abs() < 1e-5);
    }

    #[test]
    fn test_angle_at_middle_atom() {
        let a = Vec3::new(1.0, 0.0, 0.0);
        let b = Vec3::ZERO;
        let c = Vec3::new(0.0, 1.0, 0.0);
        let angle = (a - b).angle_between(c - b).to_degrees();
        assert!((angle - 90.0).abs() < 1e-3);
    }

    #[test]
    fn test_dihedral_for_staggered_chain() {
        let p0 = Vec3::new(0.0, 0.0, 0.0);
        let p1 = Vec3::new(1.0, 0.0, 0.0);
        let p2 = Vec3::new(1.0, 1.0, 0.0);
        let p3 = Vec3::new(0.0, 1.0, 0.5);

        let b1 = p1 - p0;
        let b2 = p2 - p1;
        let b3 = p3 - p2;
        let n1 = b1.cross(b2);
        let n2 = b2.cross(b3);
        let n1_len = n1.length();
        let n2_len = n2.length();
        assert!(n1_len > 1e-6 && n2_len > 1e-6);

        let n1 = n1 / n1_len;
        let n2 = n2 / n2_len;
        let b2_norm = b2.normalize();
        let m = n1.cross(b2_norm);
        let dihedral = m.dot(n2).atan2(n1.dot(n2)).to_degrees();

        assert!(
            dihedral.is_finite() && dihedral.abs() > 1.0,
            "expected finite non-zero dihedral, got {dihedral}"
        );
    }
}
