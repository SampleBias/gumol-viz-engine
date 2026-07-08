//! Atom selection system
//!
//! This system handles atom selection via raycasting and manages
//! selection state for interaction with atoms.

use crate::interaction::pick_proxy::PickProxy;
use bevy::prelude::*;
use bevy_mod_picking::prelude::*;

/// Resource tracking the current selection state
#[derive(Resource, Default, Debug, Clone)]
pub struct SelectionState {
    /// Pick-proxy entities currently selected
    pub selected_entities: Vec<Entity>,
    /// Atom IDs currently selected (stable across instanced rendering)
    pub selected_atom_ids: Vec<u32>,
    /// Last selected entity (for single-select operations)
    pub last_selected: Option<Entity>,
    /// Selection mode (single, multiple, box)
    pub mode: SelectionMode,
}

impl SelectionState {
    /// Create a new empty selection state
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if an entity is selected
    pub fn is_selected(&self, entity: Entity) -> bool {
        self.selected_entities.contains(&entity)
    }

    /// Get number of selected atoms
    pub fn len(&self) -> usize {
        self.selected_atom_ids.len()
    }

    /// Check if selection is empty
    pub fn is_empty(&self) -> bool {
        self.selected_atom_ids.is_empty()
    }

    /// Clear all selections
    pub fn clear(&mut self) {
        self.selected_entities.clear();
        self.selected_atom_ids.clear();
        self.last_selected = None;
    }

    /// Add an entity and atom ID to selection
    pub fn add(&mut self, entity: Entity, atom_id: u32) {
        if !self.selected_atom_ids.contains(&atom_id) {
            self.selected_entities.push(entity);
            self.selected_atom_ids.push(atom_id);
            self.last_selected = Some(entity);
        }
    }

    /// Remove an entity from selection
    pub fn remove(&mut self, entity: Entity, atom_id: u32) {
        self.selected_entities.retain(|&e| e != entity);
        self.selected_atom_ids.retain(|&id| id != atom_id);
        if self.last_selected == Some(entity) {
            self.last_selected = self.selected_entities.last().copied();
        }
    }

    /// Toggle selection of an entity
    pub fn toggle(&mut self, entity: Entity, atom_id: u32) {
        if self.selected_atom_ids.contains(&atom_id) {
            self.remove(entity, atom_id);
        } else {
            self.add(entity, atom_id);
        }
    }

    /// Replace selection with a single entity
    pub fn set(&mut self, entity: Entity, atom_id: u32) {
        self.selected_entities.clear();
        self.selected_atom_ids.clear();
        self.selected_entities.push(entity);
        self.selected_atom_ids.push(atom_id);
        self.last_selected = Some(entity);
    }

    pub fn atom_ids(&self) -> &[u32] {
        &self.selected_atom_ids
    }

    /// Get all selected entities
    pub fn entities(&self) -> &[Entity] {
        &self.selected_entities
    }
}

/// Selection mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SelectionMode {
    /// Single selection (replace current selection)
    #[default]
    Single,
    /// Multiple selection (add to selection)
    Multiple,
    /// Box selection (select atoms in region)
    Box,
}

/// Marker component for selected atoms
#[derive(Component)]
pub struct Selected;

/// Event sent when an atom is selected
#[derive(Event, Debug)]
pub struct AtomSelectedEvent {
    pub entity: Entity,
}

/// Event sent when an atom is deselected
#[derive(Event, Debug)]
pub struct AtomDeselectedEvent {
    pub entity: Entity,
}

/// Event sent when selection is cleared
#[derive(Event, Debug)]
pub struct SelectionClearedEvent;

/// Handle atom selection via clicking
#[allow(clippy::too_many_arguments)]
pub fn handle_atom_selection(
    mut commands: Commands,
    mut selection: ResMut<SelectionState>,
    mut selected_events: EventWriter<AtomSelectedEvent>,
    mut deselected_events: EventWriter<AtomDeselectedEvent>,
    _mouse: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut click_events: EventReader<Pointer<Click>>,
    atom_query: Query<(Entity, Option<&Selected>, &PickProxy)>,
) {
    // Process click events from bevy_mod_picking
    for event in click_events.read() {
        let entity = event.target;

        // Check if this is a pick proxy atom
        if let Ok((_, _, proxy)) = atom_query.get(entity) {
            let atom_id = proxy.atom_id;
            let is_shift_held =
                keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);
            let is_ctrl_held =
                keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight);

            // Determine selection action based on modifiers
            let action = if is_shift_held || is_ctrl_held {
                SelectionAction::Toggle
            } else {
                SelectionAction::Set
            };

            let is_selected_id = selection.selected_atom_ids.contains(&atom_id);

            match action {
                SelectionAction::Set => {
                    for selected_entity in selection.entities().to_vec() {
                        commands.entity(selected_entity).remove::<Selected>();
                        deselected_events.send(AtomDeselectedEvent {
                            entity: selected_entity,
                        });
                    }
                    selection.clear();

                    if !is_selected_id {
                        commands.entity(entity).insert(Selected);
                        selection.set(entity, atom_id);
                        selected_events.send(AtomSelectedEvent { entity });
                    }
                }
                SelectionAction::Toggle => {
                    if is_selected_id {
                        commands.entity(entity).remove::<Selected>();
                        selection.remove(entity, atom_id);
                        deselected_events.send(AtomDeselectedEvent { entity });
                    } else {
                        commands.entity(entity).insert(Selected);
                        selection.add(entity, atom_id);
                        selected_events.send(AtomSelectedEvent { entity });
                    }
                }
            }

            info!("Atom {} selected (total: {})", atom_id, selection.len());
        }
    }

    // Handle deselect all (Escape key)
    if keyboard.just_pressed(KeyCode::Escape) && !selection.is_empty() {
        for selected_entity in selection.entities().to_vec() {
            commands.entity(selected_entity).remove::<Selected>();
            deselected_events.send(AtomDeselectedEvent {
                entity: selected_entity,
            });
        }

        selection.clear();
        commands.trigger(SelectionClearedEvent);
        info!("Selection cleared");
    }
}

/// Selection action type
enum SelectionAction {
    /// Replace current selection with clicked atom
    Set,
    /// Toggle selection of clicked atom
    Toggle,
}

/// Instanced rendering handles highlight colors; keep Selected marker in sync.
pub fn sync_selection_markers(
    mut commands: Commands,
    selection: Res<SelectionState>,
    pick_query: Query<(Entity, Option<&Selected>, &PickProxy)>,
) {
    if !selection.is_changed() {
        return;
    }

    let selected_entities: std::collections::HashSet<Entity> =
        selection.selected_entities.iter().copied().collect();

    for (entity, is_selected, _) in pick_query.iter() {
        let should_select = selected_entities.contains(&entity);
        if should_select && is_selected.is_none() {
            commands.entity(entity).insert(Selected);
        } else if !should_select && is_selected.is_some() {
            commands.entity(entity).remove::<Selected>();
        }
    }
}

/// Clear selection when a new file is loaded
pub fn clear_selection_on_load(
    mut commands: Commands,
    mut selection: ResMut<SelectionState>,
    file_loaded_events: EventReader<crate::systems::loading::FileLoadedEvent>,
    mut cleared_event: EventWriter<SelectionClearedEvent>,
) {
    if !file_loaded_events.is_empty() && !selection.is_empty() {
        // Deselect all atoms
        for selected_entity in selection.entities().to_vec() {
            commands.entity(selected_entity).remove::<Selected>();
        }

        selection.clear();
        cleared_event.send(SelectionClearedEvent);
        info!("Selection cleared on file load");
    }
}

/// Register all selection systems
pub fn register(app: &mut App) {
    app.init_resource::<SelectionState>()
        .add_event::<AtomSelectedEvent>()
        .add_event::<AtomDeselectedEvent>()
        .add_event::<SelectionClearedEvent>()
        .add_systems(Update, handle_atom_selection)
        .add_systems(Update, sync_selection_markers)
        .add_systems(Update, clear_selection_on_load);

    info!("Atom selection systems registered");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selection_state() {
        let mut selection = SelectionState::new();
        assert!(selection.is_empty());
        assert_eq!(selection.len(), 0);

        let entity = Entity::PLACEHOLDER;
        selection.add(entity, 1);
        assert_eq!(selection.len(), 1);
        assert!(selection.selected_atom_ids.contains(&1));

        selection.remove(entity, 1);
        assert!(selection.is_empty());

        selection.set(entity, 2);
        assert!(selection.selected_atom_ids.contains(&2));

        selection.clear();
        assert!(selection.is_empty());
    }

    #[test]
    fn test_selection_toggle() {
        let mut selection = SelectionState::new();
        let entity = Entity::PLACEHOLDER;

        selection.toggle(entity, 3);
        assert!(selection.selected_atom_ids.contains(&3));

        selection.toggle(entity, 3);
        assert!(!selection.selected_atom_ids.contains(&3));
    }
}
