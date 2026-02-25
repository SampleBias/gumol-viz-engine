//! Distance and angle measurement tools
//!
//! Computes measurements from selected atoms:
//! - 2 atoms: distance (Å)
//! - 3 atoms: angle at middle atom (degrees)
//! - 4 atoms: dihedral angle (degrees)

use crate::core::atom::Atom;
use crate::interaction::selection::SelectionState;
use crate::systems::spawning::SpawnedAtom;
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

/// Compute measurements from selected atoms
pub fn compute_measurements(
    selection: Res<SelectionState>,
    atom_query: Query<(&Atom, &Transform), With<SpawnedAtom>>,
    mut measurements: ResMut<MeasurementState>,
) {
    measurements.distance = None;
    measurements.angle = None;
    measurements.dihedral = None;

    let entities: Vec<Entity> = selection.entities().to_vec();
    if entities.is_empty() {
        return;
    }

    // Get positions of selected atoms in order
    let positions: Vec<Vec3> = entities
        .iter()
        .filter_map(|e| atom_query.get(*e).ok())
        .map(|(_, t)| t.translation)
        .collect();

    match positions.len() {
        2 => {
            let d = positions[0].distance(positions[1]);
            measurements.distance = Some(d);
        }
        3 => {
            // Angle at position 1 (middle atom): angle between v1->v0 and v1->v2
            let v01 = positions[0] - positions[1];
            let v21 = positions[2] - positions[1];
            let angle_rad = v01.angle_between(v21);
            measurements.angle = Some(angle_rad.to_degrees());
        }
        4.. => {
            // Dihedral: angle between planes (0,1,2) and (1,2,3)
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
                let dihedral_rad = y.atan2(x);
                measurements.dihedral = Some(dihedral_rad.to_degrees());
            }
        }
        _ => {}
    }
}

/// Register measurement systems
pub fn register(app: &mut App) {
    app.init_resource::<MeasurementState>()
        .add_systems(Update, compute_measurements);
}
