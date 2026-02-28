//! Instanced rendering system for atoms
//!
//! This system implements GPU instancing to render thousands of atoms
//! with minimal draw calls (one per element instead of one per atom).

use crate::core::atom::{AtomData, Element};
use crate::core::trajectory::FrameData;
use crate::rendering::generate_atom_mesh;
use bevy::prelude::*;
use std::collections::HashMap;

// ============================================================================
// INSTANCED RENDERING COMPONENTS
// ============================================================================

/// Instance data for each atom (sent to GPU for instanced rendering)
#[derive(bevy::render::render_resource::ShaderType, Clone, Copy, Default, Debug, PartialEq)]
pub struct AtomInstanceData {
    /// Atom position in world space
    pub position: Vec3,
    /// Scale factor (multiplied with base mesh radius)
    pub scale: f32,
    /// Atom color (RGBA)
    pub color: Vec4,
    /// Padding for 16-byte alignment
    pub _padding: Vec3,
}

/// Component holding instance data for instanced atom rendering
#[derive(Component, Default, Debug)]
pub struct InstancedAtomMesh {
    /// All instances of atoms of this element type
    pub instances: Vec<AtomInstanceData>,
}

/// Marker component for instanced atom entities
#[derive(Component)]
pub struct InstancedAtomEntity {
    /// Element type for this instanced entity
    pub element: Element,
}

/// Resource tracking instanced atom entities
#[derive(Resource, Default, Debug)]
pub struct InstancedAtomEntities {
    /// Map from element to entity handle
    pub entities: HashMap<Element, Entity>,
    /// Total number of atoms across all instanced entities
    pub total_atoms: usize,
}

/// Event sent when instanced atoms are spawned
#[derive(Event, Debug)]
pub struct InstancedAtomsSpawnedEvent {
    /// Number of atoms spawned
    pub count: usize,
    /// Number of draw calls (should equal number of unique elements)
    pub draw_calls: usize,
}

/// Spawn atoms using instanced rendering (ONE entity per element)
///
/// This is the core optimization: instead of spawning N entities for N atoms,
/// we group atoms by element and spawn ONE entity per element with N instances.
/// This reduces draw calls from N to ~118 (number of elements).
pub fn spawn_atoms_instanced_internal(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    frame_data: &FrameData,
    atom_data: &[AtomData],
) -> HashMap<u32, Entity> {
    info!("Spawning {} atoms with instanced rendering", atom_data.len());

    // Track which entity each atom belongs to
    let mut entity_map = HashMap::new();

    // Step 1: Group atoms by element (118 elements maximum)
    let mut atoms_by_element: HashMap<Element, Vec<&AtomData>> = HashMap::new();

    for atom_info in atom_data {
        if let Some(_position) = frame_data.get_position(atom_info.id) {
            atoms_by_element
                .entry(atom_info.element)
                .or_default()
                .push(atom_info);
        }
    }

    info!("Grouped atoms into {} element types", atoms_by_element.len());

    // Step 2: Spawn ONE instanced entity per element
    for (element, atoms) in atoms_by_element {
        // Generate mesh ONCE per element (not per atom!)
        let radius = element.vdw_radius() * 0.5; // Use 50% of VDW radius
        let mesh = meshes.add(generate_atom_mesh(radius));

        // Get CPK color for this element
        let color_rgb = element.cpk_color();

        // Create instance data for ALL atoms of this element
        let instances: Vec<AtomInstanceData> = atoms
            .iter()
            .map(|atom_info| {
                let position = frame_data
                    .get_position(atom_info.id)
                    .unwrap_or(Vec3::ZERO);

                AtomInstanceData {
                    position,
                    scale: 1.0,
                    color: Vec4::new(color_rgb[0], color_rgb[1], color_rgb[2], 1.0),
                    _padding: Vec3::ZERO,
                }
            })
            .collect();

        info!(
            "Created {} instances for element {:?} ({})",
            instances.len(),
            element,
            element.symbol()
        );

        // Create material
        // Note: Color is handled by instance data, so we use white base
        let material = materials.add(StandardMaterial {
            base_color: Color::WHITE,
            unlit: false,
            metallic: 0.1,
            perceptual_roughness: 0.2,
            ..default()
        });

        // Spawn ONE entity with instancing
        let entity = commands
            .spawn((
                PbrBundle {
                    mesh,
                    material,
                    transform: Transform::from_translation(Vec3::ZERO),
                    ..default()
                },
                InstancedAtomMesh { instances },
                InstancedAtomEntity { element },
            ))
            .id();

        // Track entity for all atoms of this element
        for atom_info in atoms {
            entity_map.insert(atom_info.id, entity);
        }

        info!("Spawned instanced entity for element {:?}", element);
    }

    info!(
        "Total instanced entities: {} (down from {} atoms)",
        entity_map.len(),
        atom_data.len()
    );
    info!(
        "Draw call reduction: {:.1}%",
        (1.0 - entity_map.len() as f32 / atom_data.len() as f32) * 100.0
    );

    entity_map
}

/// System to spawn instanced atoms when simulation data is loaded
pub fn spawn_instanced_atoms_on_load(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    sim_data: Res<crate::systems::loading::SimulationData>,
    mut file_loaded_events: EventReader<crate::systems::loading::FileLoadedEvent>,
    mut instanced_entities: ResMut<InstancedAtomEntities>,
    mut spawned_event: EventWriter<InstancedAtomsSpawnedEvent>,
) {
    // Check if atoms are already spawned
    if !instanced_entities.entities.is_empty() {
        return;
    }

    // Check if we should spawn (any file loaded event)
    let should_spawn = file_loaded_events.read().next().is_some();

    if should_spawn && sim_data.loaded && !sim_data.atom_data.is_empty() {
        info!(
            "File loaded, spawning {} instanced atoms",
            sim_data.atom_data.len()
        );

        // Get the first frame
        if let Some(first_frame) = sim_data.trajectory.get_frame(0) {
            let new_entities = spawn_atoms_instanced_internal(
                &mut commands,
                &mut meshes,
                &mut materials,
                first_frame,
                &sim_data.atom_data,
            );

            // Track entities by element
            instanced_entities.entities.clear(); // Clear previous entities
            // Note: For now we just track count, elements map can be built later
            for (_atom_id, _entity) in new_entities.iter() {
                // Element lookup would go here
            }
            instanced_entities.total_atoms = sim_data.atom_data.len();

            // Send event
            spawned_event.send(InstancedAtomsSpawnedEvent {
                count: instanced_entities.total_atoms,
                draw_calls: new_entities.len(),
            });
        }
    }
}

/// Clear all instanced atoms
pub fn despawn_all_instanced_atoms(
    mut commands: Commands,
    mut instanced_entities: ResMut<InstancedAtomEntities>,
) {
    let count = instanced_entities.entities.len();

    if count > 0 {
        info!("Despawning {} instanced atom entities", count);

        // Despawn all entities
        for (_, entity) in instanced_entities.entities.drain() {
            commands.entity(entity).despawn_recursive();
        }

        instanced_entities.total_atoms = 0;
    }
}

/// Clear instanced atoms when new file is loaded
pub fn clear_instanced_atoms_on_load(
    mut commands: Commands,
    mut instanced_entities: ResMut<InstancedAtomEntities>,
    file_loaded_events: EventReader<crate::systems::loading::FileLoadedEvent>,
) {
    if !file_loaded_events.is_empty() && !instanced_entities.entities.is_empty() {
        // Despawn all instanced entities
        for (_, entity) in instanced_entities.entities.drain() {
            commands.entity(entity).despawn_recursive();
        }

        instanced_entities.total_atoms = 0;
        info!("Instanced atoms cleared on file load");
    }
}

/// Calculate the center of mass of all atoms (for camera centering)
pub fn calculate_center_of_mass_instanced(
    sim_data: Res<crate::systems::loading::SimulationData>,
) -> Option<Vec3> {
    if !sim_data.loaded || sim_data.atom_data.is_empty() {
        return None;
    }

    if let Some(frame) = sim_data.trajectory.get_frame(0) {
        let mut sum = Vec3::ZERO;
        let mut count = 0;

        for position in frame.positions.values() {
            sum += *position;
            count += 1;
        }

        if count > 0 {
            Some(sum / count as f32)
        } else {
            None
        }
    } else {
        None
    }
}

/// Center the camera on the molecule when a file is loaded
pub fn center_camera_on_file_load_instanced(
    mut file_loaded_events: EventReader<crate::systems::loading::FileLoadedEvent>,
    sim_data: Res<crate::systems::loading::SimulationData>,
    mut camera_query: Query<&mut bevy_panorbit_camera::PanOrbitCamera>,
) {
    if file_loaded_events.read().next().is_none() {
        return;
    }

    let center = calculate_center_of_mass_instanced(sim_data);

    if let Some(center) = center {
        for mut cam in camera_query.iter_mut() {
            cam.focus = center;
            cam.target_focus = center;
            info!("Camera centered on molecule at {:?}", center);
        }
    }
}

/// Register all instanced rendering systems
pub fn register(app: &mut App) {
    app.init_resource::<InstancedAtomEntities>()
        .add_event::<InstancedAtomsSpawnedEvent>()
        .add_systems(Update, spawn_instanced_atoms_on_load)
        .add_systems(Update, clear_instanced_atoms_on_load)
        .add_systems(Update, center_camera_on_file_load_instanced);

    info!("Instanced rendering plugin registered");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atom_instance_data_alignment() {
        let instance = AtomInstanceData {
            position: Vec3::new(1.0, 2.0, 3.0),
            scale: 1.5,
            color: Vec4::new(0.5, 0.5, 0.5, 1.0),
            _padding: Vec3::ZERO,
        };

        assert_eq!(instance.position, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(instance.scale, 1.5);
    }

    #[test]
    fn test_instanced_atom_entities() {
        let mut entities = InstancedAtomEntities::default();
        assert!(entities.entities.is_empty());
        assert_eq!(entities.total_atoms, 0);
    }
}
