//! Atom selection system
//!
//! This system handles atom selection via raycasting and manages
//! selection state for interaction with atoms.

use bevy::prelude::*;
use bevy_mod_picking::prelude::*;

/// Resource tracking the current selection state
#[derive(Resource, Default, Debug, Clone)]
pub struct SelectionState {
    /// List of selected atom entities
    pub selected_entities: Vec<Entity>,
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

    /// Get number of selected entities
    pub fn len(&self) -> usize {
        self.selected_entities.len()
    }

    /// Check if selection is empty
    pub fn is_empty(&self) -> bool {
        self.selected_entities.is_empty()
    }

    /// Clear all selections
    pub fn clear(&mut self) {
        self.selected_entities.clear();
        self.last_selected = None;
    }

    /// Add an entity to selection
    pub fn add(&mut self, entity: Entity) {
        if !self.selected_entities.contains(&entity) {
            self.selected_entities.push(entity);
            self.last_selected = Some(entity);
        }
    }

    /// Remove an entity from selection
    pub fn remove(&mut self, entity: Entity) {
        self.selected_entities.retain(|&e| e != entity);
        if self.last_selected == Some(entity) {
            self.last_selected = self.selected_entities.last().copied();
        }
    }

    /// Toggle selection of an entity
    pub fn toggle(&mut self, entity: Entity) {
        if self.is_selected(entity) {
            self.remove(entity);
        } else {
            self.add(entity);
        }
    }

    /// Replace selection with a single entity
    pub fn set(&mut self, entity: Entity) {
        self.selected_entities.clear();
        self.selected_entities.push(entity);
        self.last_selected = Some(entity);
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
pub fn handle_atom_selection(
    mut commands: Commands,
    mut selection: ResMut<SelectionState>,
    mut selected_events: EventWriter<AtomSelectedEvent>,
    mut deselected_events: EventWriter<AtomDeselectedEvent>,
    _mouse: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut click_events: EventReader<Pointer<Click>>,
    atom_query: Query<
        (Entity, Option<&Selected>),
        With<crate::systems::spawning::SpawnedAtom>,
    >,
) {
    // Process click events from bevy_mod_picking
    for event in click_events.read() {
        let entity = event.target;

        // Check if this is an atom
        if atom_query.get(entity).is_ok() {
            let is_shift_held = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);
            let is_ctrl_held = keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight);

            // Determine selection action based on modifiers
            let action = if is_shift_held || is_ctrl_held {
                SelectionAction::Toggle
            } else {
                SelectionAction::Set
            };

            // Get current selection state
            let is_selected = selection.is_selected(entity);

            match action {
                SelectionAction::Set => {
                    // Clear existing selection and select this atom
                    // Deselect all currently selected atoms
                    for selected_entity in selection.entities().iter().copied().collect::<Vec<_>>() {
                        if selected_entity != entity {
                            commands.entity(selected_entity).remove::<Selected>();
                            deselected_events.send(AtomDeselectedEvent { entity: selected_entity });
                        }
                    }

                    // Select or deselect clicked atom
                    if is_selected {
                        commands.entity(entity).remove::<Selected>();
                        selection.remove(entity);
                        deselected_events.send(AtomDeselectedEvent { entity });
                    } else {
                        commands.entity(entity).insert(Selected);
                        selection.set(entity);
                        selected_events.send(AtomSelectedEvent { entity });
                    }
                }
                SelectionAction::Toggle => {
                    // Toggle selection of clicked atom
                    if is_selected {
                        commands.entity(entity).remove::<Selected>();
                        selection.remove(entity);
                        deselected_events.send(AtomDeselectedEvent { entity });
                    } else {
                        commands.entity(entity).insert(Selected);
                        selection.add(entity);
                        selected_events.send(AtomSelectedEvent { entity });
                    }
                }
            }

            info!("Atom {:?} selected (total: {})", entity, selection.len());
        }
    }

    // Handle deselect all (Escape key)
    if keyboard.just_pressed(KeyCode::Escape) && !selection.is_empty() {
        for selected_entity in selection.entities().iter().copied().collect::<Vec<_>>() {
            commands.entity(selected_entity).remove::<Selected>();
            deselected_events.send(AtomDeselectedEvent { entity: selected_entity });
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

/// Update atom highlighting based on selection state
pub fn update_selection_highlight(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    selection: Res<SelectionState>,
    mut atom_query: Query<
        (
            Entity,
            Option<&Selected>,
            &mut Handle<StandardMaterial>,
            &crate::core::atom::Atom,
        ),
        With<crate::systems::spawning::SpawnedAtom>,
    >,
) {
    // Highlight color for selected atoms (yellow with emissive glow)
    let highlight_color = Color::srgb(1.0, 1.0, 0.0);

    for (entity, is_selected, mut material_handle, atom) in atom_query.iter_mut() {
        let should_be_selected = selection.is_selected(entity);

        // Ensure Selected component matches selection state
        if should_be_selected && is_selected.is_none() {
            commands.entity(entity).insert(Selected);
        } else if !should_be_selected && is_selected.is_some() {
            commands.entity(entity).remove::<Selected>();
        }

        // Update material based on selection state
        if let Some(material) = materials.get(material_handle.id()) {
            let current_color = material.base_color;
            let is_highlighted = current_color == highlight_color;
            let should_highlight = selection.is_selected(entity);

            // Only update material if highlight state changed
            if should_highlight != is_highlighted {
                if should_highlight {
                    // Create highlight material (yellow with emissive glow)
                    let highlight_material = materials.add(StandardMaterial {
                        base_color: highlight_color,
                        emissive: LinearRgba::rgb(0.5, 0.5, 0.0),
                        metallic: 0.3,
                        perceptual_roughness: 0.2,
                        ..default()
                    });
                    *material_handle = highlight_material;
                } else {
                    // Restore original CPK color
                    let cpk_color = atom.element.cpk_color();
                    let original_material = materials.add(StandardMaterial {
                        base_color: Color::srgb(cpk_color[0], cpk_color[1], cpk_color[2]),
                        metallic: 0.1,
                        perceptual_roughness: 0.2,
                        ..default()
                    });
                    *material_handle = original_material;
                }
            }
        }
    }
}

/// Selection box visualization (for box selection mode)
#[derive(Component)]
pub struct SelectionBox;

/// Draw selection box when in box selection mode
pub fn draw_selection_box(
    _commands: Commands,
    _selection: Res<SelectionState>,
    _mouse: Res<ButtonInput<MouseButton>>,
    _keyboard: Res<ButtonInput<KeyCode>>,
    _windows: Query<&Window>,
    _camera_q: Query<(&Camera, &GlobalTransform)>,
) {
    // Box selection is not yet implemented
    // This would require mouse drag detection and raycasting to a plane
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
        for selected_entity in selection.entities().iter().copied().collect::<Vec<_>>() {
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
        .add_systems(Update, update_selection_highlight)
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
        selection.add(entity);
        assert_eq!(selection.len(), 1);
        assert!(selection.is_selected(entity));

        selection.remove(entity);
        assert!(selection.is_empty());

        selection.set(entity);
        assert!(selection.is_selected(entity));

        selection.clear();
        assert!(selection.is_empty());
    }

    #[test]
    fn test_selection_toggle() {
        let mut selection = SelectionState::new();
        let entity = Entity::PLACEHOLDER;

        selection.toggle(entity);
        assert!(selection.is_selected(entity));

        selection.toggle(entity);
        assert!(!selection.is_selected(entity));
    }
}
