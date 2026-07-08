//! Rubber-band box selection in screen space.
//!
//! Middle-mouse drag selects all atoms whose projected positions fall inside
//! the rectangle. Hold Shift to add to the current selection.

use crate::interaction::pick_proxy::PickProxyEntities;
use crate::interaction::selection::{
    AtomDeselectedEvent, AtomSelectedEvent, Selected, SelectionState,
};
use crate::rendering::atom_index::InstancedAtomIndex;
use crate::rendering::instanced::{InstancedAtomEntity, InstancedAtomMesh};
use crate::systems::loading::SimulationData;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

/// Screen-space rectangle (window coordinates, origin top-left).
#[derive(Debug, Clone, Copy)]
pub struct ScreenRect {
    pub min: Vec2,
    pub max: Vec2,
}

impl ScreenRect {
    pub fn from_corners(a: Vec2, b: Vec2) -> Self {
        Self {
            min: a.min(b),
            max: a.max(b),
        }
    }

    pub fn contains(&self, point: Vec2) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
    }

    /// Ignore accidental clicks (tiny movement).
    pub fn is_degenerate(&self) -> bool {
        (self.max - self.min).length_squared() < 4.0
    }
}

/// Active rubber-band drag state (read by UI overlay).
#[derive(Resource, Default, Debug)]
pub struct BoxSelectionState {
    pub dragging: bool,
    pub start: Vec2,
    pub current: Vec2,
}

/// Project a world position to viewport/window coordinates (logical pixels, origin top-left).
pub fn world_to_window(
    camera: &Camera,
    camera_transform: &GlobalTransform,
    world: Vec3,
) -> Option<Vec2> {
    camera.world_to_viewport(camera_transform, world)
}

/// Return atom IDs whose screen projection lies inside `rect`.
pub fn atoms_in_screen_rect(
    rect: ScreenRect,
    index: &InstancedAtomIndex,
    instanced: &Query<(&InstancedAtomEntity, &InstancedAtomMesh)>,
    camera: &Camera,
    camera_transform: &GlobalTransform,
) -> Vec<u32> {
    if index.atom_to_instance.is_empty() {
        return Vec::new();
    }

    let positions = index.collect_positions(instanced);
    let mut hits = Vec::new();

    for (&atom_id, &world) in &positions {
        if let Some(screen) = world_to_window(camera, camera_transform, world) {
            if rect.contains(screen) {
                hits.push(atom_id);
            }
        }
    }

    hits.sort_unstable();
    hits
}

fn apply_box_hits(
    commands: &mut Commands,
    selection: &mut SelectionState,
    pick_entities: &PickProxyEntities,
    hit_ids: &[u32],
    add_to_selection: bool,
    selected_events: &mut EventWriter<AtomSelectedEvent>,
    deselected_events: &mut EventWriter<AtomDeselectedEvent>,
) {
    if !add_to_selection {
        for entity in selection.entities().to_vec() {
            commands.entity(entity).remove::<Selected>();
            deselected_events.send(AtomDeselectedEvent { entity });
        }
        selection.clear();
    }

    for &atom_id in hit_ids {
        if selection.selected_atom_ids.contains(&atom_id) {
            continue;
        }

        if let Some(&entity) = pick_entities.entities.get(&atom_id) {
            commands.entity(entity).insert(Selected);
            selection.add(entity, atom_id);
            selected_events.send(AtomSelectedEvent { entity });
        } else {
            selection.selected_atom_ids.push(atom_id);
        }
    }
}

/// Middle-mouse drag box selection.
#[allow(clippy::too_many_arguments)]
pub fn handle_box_selection(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut box_state: ResMut<BoxSelectionState>,
    mut selection: ResMut<SelectionState>,
    mut selected_events: EventWriter<AtomSelectedEvent>,
    mut deselected_events: EventWriter<AtomDeselectedEvent>,
    sim_data: Res<SimulationData>,
    index: Res<InstancedAtomIndex>,
    instanced: Query<(&InstancedAtomEntity, &InstancedAtomMesh)>,
    pick_entities: Res<PickProxyEntities>,
    window: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
) {
    if !sim_data.loaded {
        return;
    }

    let Ok(window) = window.get_single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };

    if mouse.just_pressed(MouseButton::Middle) {
        box_state.dragging = true;
        box_state.start = cursor;
        box_state.current = cursor;
        return;
    }

    if !box_state.dragging {
        return;
    }

    if mouse.pressed(MouseButton::Middle) {
        box_state.current = cursor;
    }

    if mouse.just_released(MouseButton::Middle) {
        box_state.dragging = false;
        let rect = ScreenRect::from_corners(box_state.start, box_state.current);

        if rect.is_degenerate() {
            return;
        }

        let Ok((camera, camera_transform)) = camera_q.get_single() else {
            return;
        };

        let hit_ids = atoms_in_screen_rect(
            rect,
            &index,
            &instanced,
            camera,
            camera_transform,
        );

        if hit_ids.is_empty() {
            return;
        }

        let add_to_selection =
            keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);

        apply_box_hits(
            &mut commands,
            &mut selection,
            &pick_entities,
            &hit_ids,
            add_to_selection,
            &mut selected_events,
            &mut deselected_events,
        );

        info!(
            "Box selection: {} atoms (total selected: {})",
            hit_ids.len(),
            selection.len()
        );
    }
}

/// Ctrl+A — select all atoms (warn above 10K).
pub fn handle_select_all(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut selection: ResMut<SelectionState>,
    mut selected_events: EventWriter<AtomSelectedEvent>,
    mut deselected_events: EventWriter<AtomDeselectedEvent>,
    sim_data: Res<SimulationData>,
    pick_entities: Res<PickProxyEntities>,
) {
    let ctrl = keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight);
    if !ctrl || !keyboard.just_pressed(KeyCode::KeyA) || !sim_data.loaded {
        return;
    }

    let count = sim_data.atom_data.len();
    if count > 10_000 {
        warn!("Selecting all {count} atoms — highlight updates may be slower");
    }

    for entity in selection.entities().to_vec() {
        commands.entity(entity).remove::<Selected>();
        deselected_events.send(AtomDeselectedEvent { entity });
    }
    selection.clear();

    for atom in &sim_data.atom_data {
        if let Some(&entity) = pick_entities.entities.get(&atom.id) {
            commands.entity(entity).insert(Selected);
            selection.add(entity, atom.id);
            selected_events.send(AtomSelectedEvent { entity });
        } else if !selection.selected_atom_ids.contains(&atom.id) {
            selection.selected_atom_ids.push(atom.id);
        }
    }

    info!("Selected all {} atoms", selection.len());
}

pub fn register(app: &mut App) {
    app.init_resource::<BoxSelectionState>()
        .add_systems(Update, (handle_box_selection, handle_select_all));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_screen_rect_contains() {
        let rect = ScreenRect::from_corners(Vec2::new(10.0, 20.0), Vec2::new(100.0, 80.0));
        assert!(rect.contains(Vec2::new(50.0, 50.0)));
        assert!(!rect.contains(Vec2::new(5.0, 50.0)));
    }

    #[test]
    fn test_screen_rect_degenerate() {
        let rect = ScreenRect::from_corners(Vec2::new(10.0, 10.0), Vec2::new(10.5, 10.5));
        assert!(rect.is_degenerate());
        let rect = ScreenRect::from_corners(Vec2::new(0.0, 0.0), Vec2::new(20.0, 20.0));
        assert!(!rect.is_degenerate());
    }
}
