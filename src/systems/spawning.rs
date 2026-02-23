//! Atom spawning system
//!
//! This system creates Bevy entities for atoms and manages their lifecycle.

use crate::core::atom::Atom;
use crate::core::trajectory::FrameData;
use crate::rendering;
use bevy::prelude::*;
use std::collections::HashMap;

/// Marker component for atom entities spawned by this system
#[derive(Component)]
pub struct SpawnedAtom {
    /// Atom ID from the trajectory
    pub atom_id: u32,
}

/// Resource tracking atom entities
#[derive(Resource, Default, Debug)]
pub struct AtomEntities {
    /// Map from atom ID to entity
    pub entities: HashMap<u32, Entity>,
}

/// Event sent when atoms are spawned
#[derive(Event, Debug)]
pub struct AtomsSpawnedEvent {
    /// Number of atoms spawned
    pub count: usize,
}

/// Event sent when atoms are despawned
#[derive(Event, Debug)]
pub struct AtomsDespawnedEvent;

/// Internal function to spawn atoms from frame data
fn spawn_atoms_from_frame_internal(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    frame_data: &FrameData,
    atom_data: &[crate::core::atom::AtomData],
) -> HashMap<u32, Entity> {
    info!("Spawning {} atoms", atom_data.len());

    let mut entity_map = HashMap::new();

    for atom_info in atom_data {
        if let Some(position) = frame_data.get_position(atom_info.id) {
            // Generate mesh for this atom type
            let radius = atom_info.element.vdw_radius() * 0.5; // Use 50% of VDW radius for visibility
            let mesh = meshes.add(rendering::generate_atom_mesh(radius));

            // Create material with CPK color
            let color = atom_info.element.cpk_color();
            let material = materials.add(StandardMaterial {
                base_color: Color::srgb(color[0], color[1], color[2]),
                metallic: 0.1,
                perceptual_roughness: 0.2,
                ..default()
            });

            // Spawn the atom entity
            let entity = commands
                .spawn((
                    PbrBundle {
                        mesh,
                        material,
                        transform: Transform::from_translation(position),
                        ..default()
                    },
                    SpawnedAtom {
                        atom_id: atom_info.id,
                    },
                    Atom {
                        id: atom_info.id,
                        element: atom_info.element,
                        position,
                        residue_id: atom_info.residue_id,
                        residue_name: atom_info.residue_name.clone(),
                        chain_id: atom_info.chain_id.clone(),
                        b_factor: atom_info.b_factor,
                        occupancy: atom_info.occupancy,
                        name: atom_info.name.clone(),
                    },
                ))
                .id();

            // Track the entity
            entity_map.insert(atom_info.id, entity);
        }
    }

    info!("Spawned {} atom entities", entity_map.len());
    entity_map
}

/// Clear all spawned atoms
pub fn despawn_all_atoms(
    mut commands: Commands,
    mut atom_entities: ResMut<AtomEntities>,
    mut despawned_event: EventWriter<AtomsDespawnedEvent>,
) {
    let count = atom_entities.entities.len();

    if count > 0 {
        info!("Despawning {} atoms", count);

        // Despawn all entities
        for (_, entity) in atom_entities.entities.drain() {
            commands.entity(entity).despawn_recursive();
        }

        despawned_event.send(AtomsDespawnedEvent);
    }
}

/// System to spawn atoms when simulation data is loaded
pub fn spawn_atoms_on_load(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    sim_data: Res<crate::systems::loading::SimulationData>,
    mut file_loaded_events: EventReader<crate::systems::loading::FileLoadedEvent>,
    mut atom_entities: ResMut<AtomEntities>,
    mut spawned_event: EventWriter<AtomsSpawnedEvent>,
) {
    // Check if atoms are already spawned
    if !atom_entities.entities.is_empty() {
        return;
    }

    // Check if we should spawn (any file loaded event)
    let should_spawn = file_loaded_events.read().next().is_some();

    if should_spawn && sim_data.loaded && !sim_data.atom_data.is_empty() {
        info!("File loaded, spawning {} atoms", sim_data.atom_data.len());

        // Get the first frame
        if let Some(first_frame) = sim_data.trajectory.get_frame(0) {
            let new_entities = spawn_atoms_from_frame_internal(
                &mut commands,
                &mut meshes,
                &mut materials,
                first_frame,
                &sim_data.atom_data,
            );

            // Store entities in the resource
            atom_entities.entities = new_entities;

            // Send event
            spawned_event.send(AtomsSpawnedEvent {
                count: atom_entities.entities.len(),
            });
        }
    }
}

/// Update atom positions from the current frame
pub fn update_atom_positions(
    sim_data: Res<crate::systems::loading::SimulationData>,
    timeline: Res<crate::core::trajectory::TimelineState>,
    mut atom_query: Query<(&SpawnedAtom, &mut Transform)>,
) {
    if !sim_data.loaded {
        return;
    }

    // Get the current frame
    if let Some(frame) = sim_data.trajectory.get_frame(timeline.current_frame) {
        for (spawned_atom, mut transform) in atom_query.iter_mut() {
            if let Some(position) = frame.get_position(spawned_atom.atom_id) {
                transform.translation = position;
            }
        }
    }
}

/// Calculate the center of mass of all atoms
pub fn calculate_center_of_mass(atom_query: Query<&Transform, (With<SpawnedAtom>, Without<Camera>)>) -> Option<Vec3> {
    let mut sum = Vec3::ZERO;
    let mut count = 0;

    for transform in atom_query.iter() {
        sum += transform.translation;
        count += 1;
    }

    if count > 0 {
        Some(sum / count as f32)
    } else {
        None
    }
}

/// Center the camera on the molecule
pub fn center_camera_on_molecule(
    mut camera_query: Query<&mut Transform, With<Camera>>,
    atom_query: Query<&Transform, (With<SpawnedAtom>, Without<Camera>)>,
) {
    let center = if let Some(center) = calculate_center_of_mass(atom_query) {
        center
    } else {
        return;
    };

    // Move camera to look at center
    for mut transform in camera_query.iter_mut() {
        transform.look_at(center, Vec3::Y);
    }

    info!("Camera centered on molecule at {:?}", center);
}

/// Register all spawning systems
pub fn register(app: &mut App) {
    app.init_resource::<AtomEntities>()
        .add_event::<AtomsSpawnedEvent>()
        .add_event::<AtomsDespawnedEvent>()
        .add_systems(Update, spawn_atoms_on_load)
        .add_systems(Update, update_atom_positions)
        .add_systems(PostUpdate, center_camera_on_molecule);

    info!("Spawning systems registered");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spawned_atom_component() {
        let atom = SpawnedAtom { atom_id: 42 };
        assert_eq!(atom.atom_id, 42);
    }

    #[test]
    fn test_atom_entities() {
        let mut entities = AtomEntities::default();
        assert!(entities.entities.is_empty());

        entities.entities.insert(1, Entity::PLACEHOLDER);
        assert_eq!(entities.entities.len(), 1);
    }
}
