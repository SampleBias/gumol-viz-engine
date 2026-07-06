//! Camera control systems

use crate::interaction::selection::SelectionState;
use crate::rendering::atom_index::InstancedAtomIndex;
use crate::rendering::instanced::{InstancedAtomEntity, InstancedAtomMesh};
use crate::systems::loading::SimulationData;
use bevy::prelude::*;
use bevy_panorbit_camera::PanOrbitCamera;

pub fn register(app: &mut App) {
    app.add_systems(Update, (
        focus_on_molecule_key,
        focus_on_selection_key,
    ));
    info!("Camera module registered");
}

/// F — center camera on entire molecule
fn focus_on_molecule_key(
    keyboard: Res<ButtonInput<KeyCode>>,
    sim_data: Res<SimulationData>,
    mut camera_query: Query<&mut PanOrbitCamera, With<Camera3d>>,
) {
    if !keyboard.just_pressed(KeyCode::KeyF) {
        return;
    }
    let shift = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);
    if shift {
        return;
    }

    if !sim_data.loaded {
        return;
    }

    let Some(frame) = sim_data.get_frame(0) else {
        return;
    };

    let mut sum = Vec3::ZERO;
    let mut count = 0;
    for pos in frame.positions.values() {
        sum += *pos;
        count += 1;
    }

    if count == 0 {
        return;
    }

    let center = sum / count as f32;
    for mut cam in camera_query.iter_mut() {
        cam.focus = center;
        cam.target_focus = center;
    }
}

/// Shift+F — center camera on current selection
fn focus_on_selection_key(
    keyboard: Res<ButtonInput<KeyCode>>,
    selection: Res<SelectionState>,
    index: Res<InstancedAtomIndex>,
    instanced: Query<(&InstancedAtomEntity, &InstancedAtomMesh)>,
    mut camera_query: Query<&mut PanOrbitCamera, With<Camera3d>>,
) {
    let shift = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);
    if !keyboard.just_pressed(KeyCode::KeyF) || !shift || selection.is_empty() {
        return;
    }

    let mut sum = Vec3::ZERO;
    let mut count = 0;
    for &atom_id in selection.atom_ids() {
        if let Some(pos) = index.get_position(atom_id, &instanced) {
            sum += pos;
            count += 1;
        }
    }

    if count == 0 {
        return;
    }

    let center = sum / count as f32;
    for mut cam in camera_query.iter_mut() {
        cam.focus = center;
        cam.target_focus = center;
    }
}
