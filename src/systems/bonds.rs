//! Bond detection and rendering system
//!
//! This system detects bonds between atoms based on distance and
//! spawns bond entities with cylindrical meshes.

use crate::core::atom::{Atom, Element};
use crate::core::bond::{Bond, BondData, BondType, BondOrder};
use crate::rendering;
use bevy::prelude::*;
use std::collections::HashMap;

/// Resource tracking bond entities
#[derive(Resource, Default, Debug)]
pub struct BondEntities {
    /// Map from bond ID (atom_a_id, atom_b_id) to entity
    pub entities: HashMap<(u32, u32), Entity>,
}

/// Resource containing bond detection configuration
#[derive(Resource, Clone, Debug)]
pub struct BondDetectionConfig {
    /// Enable automatic bond detection
    pub enabled: bool,
    /// Maximum bond distance multiplier (factor of van der Waals radii sum)
    pub distance_multiplier: f32,
    /// Maximum absolute bond distance in Angstroms
    pub max_bond_distance: f32,
    /// Minimum bond distance in Angstroms
    pub min_bond_distance: f32,
    /// Detect bonds only between atoms in same residue
    pub same_residue_only: bool,
}

impl Default for BondDetectionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            distance_multiplier: 1.2,
            max_bond_distance: 3.0,
            min_bond_distance: 0.5,
            same_residue_only: false,
        }
    }
}

impl BondDetectionConfig {
    /// Check if two atoms should be bonded based on distance
    pub fn should_bond(
        &self,
        atom_a: &Atom,
        atom_b: &Atom,
        distance: f32,
    ) -> bool {
        // Check distance bounds
        if distance < self.min_bond_distance || distance > self.max_bond_distance {
            return false;
        }

        // Check van der Waals radii
        let vdw_sum = atom_a.element.vdw_radius() + atom_b.element.vdw_radius();
        if distance > vdw_sum * self.distance_multiplier {
            return false;
        }

        // Check same residue constraint
        if self.same_residue_only {
            if atom_a.residue_id != atom_b.residue_id {
                return false;
            }
        }

        true
    }

    /// Determine bond order based on atom types and distance
    pub fn determine_bond_order(
        &self,
        atom_a: &Atom,
        atom_b: &Atom,
        distance: f32,
    ) -> BondOrder {
        // Simplified bond order detection based on distance
        let expected_length = crate::core::bond::BondLengths::get_length(atom_a.element, atom_b.element);

        if distance < expected_length * 0.9 {
            BondOrder::Triple
        } else if distance < expected_length * 0.95 {
            BondOrder::Double
        } else {
            BondOrder::Single
        }
    }

    /// Determine bond type based on atom types
    pub fn determine_bond_type(&self, atom_a: &Atom, atom_b: &Atom) -> BondType {
        // Simplified bond type detection
        match (atom_a.element.symbol(), atom_b.element.symbol()) {
            ("H", _) | (_, "H") => BondType::Covalent,
            ("C", "C") => BondType::Covalent,
            ("C", "N") | ("N", "C") => BondType::Covalent,
            ("C", "O") | ("O", "C") => BondType::Covalent,
            ("N", "O") | ("O", "N") => BondType::Covalent,
            ("S", "S") => BondType::Disulfide,
            ("Fe", "S") | ("S", "Fe") => BondType::Coordinate,
            ("Mg", "O") | ("O", "Mg") => BondType::Ionic,
            ("Ca", "O") | ("O", "Ca") => BondType::Ionic,
            _ => BondType::Covalent,
        }
    }
}

/// Event sent when bonds are spawned
#[derive(Event, Debug)]
pub struct BondsSpawnedEvent {
    /// Number of bonds spawned
    pub count: usize,
}

/// Event sent when bonds are despawned
#[derive(Event, Debug)]
pub struct BondsDespawnedEvent;

/// Detect bonds between atoms based on distance
pub fn detect_bonds(
    atom_entities: Res<crate::systems::spawning::AtomEntities>,
    config: Res<BondDetectionConfig>,
    atom_query: Query<&Atom>,
) -> Vec<BondData> {
    if !config.enabled {
        return Vec::new();
    }

    let mut bonds = Vec::new();

    // Get all atom IDs
    let atom_ids: Vec<u32> = atom_entities.entities.keys().copied().collect();

    // Detect bonds (O(n^2) naive approach - optimize for production)
    for (i, &atom_id_a) in atom_ids.iter().enumerate() {
        // Get atom A
        let entity_a = match atom_entities.entities.get(&atom_id_a) {
            Some(entity) => *entity,
            None => continue,
        };

        let atom_a = match atom_query.get(entity_a) {
            Ok(atom) => atom,
            Err(_) => continue,
        };

        let pos_a = atom_a.position;

        // Check against all other atoms
        for &atom_id_b in atom_ids.iter().skip(i + 1) {
            // Get atom B
            let entity_b = match atom_entities.entities.get(&atom_id_b) {
                Some(entity) => *entity,
                None => continue,
            };

            let atom_b = match atom_query.get(entity_b) {
                Ok(atom) => atom,
                Err(_) => continue,
            };

            // Calculate distance
            let pos_b = atom_b.position;
            let distance = pos_a.distance(pos_b);

            // Check if should bond
            if config.should_bond(atom_a, atom_b, distance) {
                // Determine bond properties
                let bond_order = config.determine_bond_order(atom_a, atom_b, distance);
                let bond_type = config.determine_bond_type(atom_a, atom_b);

                // Create bond data
                let bond_data = BondData::new(
                    atom_id_a,
                    atom_id_b,
                    bond_type,
                    bond_order,
                    distance,
                );

                bonds.push(bond_data);
            }
        }
    }

    info!("Detected {} bonds", bonds.len());
    bonds
}

/// Spawn bond entities from detected bonds
pub fn spawn_bonds(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    atom_entities: Res<crate::systems::spawning::AtomEntities>,
    mut bond_entities: ResMut<BondEntities>,
    atom_query: Query<&Atom>,
    config: Res<BondDetectionConfig>,
    mut spawned_events: EventWriter<BondsSpawnedEvent>,
) {
    // Only spawn if config is enabled
    if !config.enabled {
        return;
    }

    // Only spawn if atoms exist
    if atom_entities.entities.is_empty() {
        return;
    }

    // Detect bonds
    let bonds = detect_bonds(atom_entities, config, atom_query);

    if bonds.is_empty() {
        info!("No bonds to spawn");
        return;
    }

    info!("Spawning {} bonds...", bonds.len());

    // Spawn bond entities
    for bond_data in bonds {
        let atom_a_id = bond_data.atom_a_id;
        let atom_b_id = bond_data.atom_b_id;

        // Get atom entities
        let entity_a = match atom_entities.entities.get(&atom_a_id) {
            Some(entity) => *entity,
            None => continue,
        };

        let entity_b = match atom_entities.entities.get(&atom_b_id) {
            Some(entity) => *entity,
            None => continue,
        };

        // Get atom positions
        let pos_a = match atom_query.get(entity_a) {
            Ok(atom) => atom.position,
            Err(_) => continue,
        };

        let pos_b = match atom_query.get(entity_b) {
            Ok(atom) => atom.position,
            Err(_) => continue,
        };

        // Calculate bond geometry
        let bond_vector = pos_b - pos_a;
        let bond_length = bond_vector.length();
        let bond_midpoint = pos_a + bond_vector * 0.5;

        // Generate bond mesh
        let bond_radius = 0.1; // Default bond radius
        let bond_mesh = meshes.add(rendering::generate_bond_mesh(bond_length, bond_radius));

        // Create bond material
        let bond_material = materials.add(StandardMaterial {
            base_color: Color::srgb(0.6, 0.6, 0.6), // Gray bonds
            metallic: 0.2,
            perceptual_roughness: 0.4,
            ..default()
        });

        // Calculate transform
        let rotation = if bond_length > 0.0001 {
            // Calculate rotation to align cylinder with bond vector
            let up = Vec3::Y;
            let direction = bond_vector.normalize();
            let axis = up.cross(direction);
            let axis_length = axis.length();
            let angle = if axis_length > 0.0001 {
                axis_length.atan2(up.dot(direction))
            } else if up.dot(direction) < 0.0 {
                std::f32::consts::PI
            } else {
                0.0
            };
            Quat::from_axis_angle(axis.normalize(), angle)
        } else {
            Quat::IDENTITY
        };

        // Spawn bond entity
        let bond_entity = commands
            .spawn((
                PbrBundle {
                    mesh: bond_mesh,
                    material: bond_material,
                    transform: Transform {
                        translation: bond_midpoint,
                        rotation,
                        scale: Vec3::ONE,
                    },
                    ..default()
                },
                Bond {
                    atom_a: entity_a,
                    atom_b: entity_b,
                    atom_a_id: atom_a_id,
                    atom_b_id: atom_b_id,
                    bond_type: bond_data.bond_type,
                    order: bond_data.order,
                    length: bond_length,
                },
            ))
            .id();

        // Track bond entity
        let bond_key = if atom_a_id < atom_b_id {
            (atom_a_id, atom_b_id)
        } else {
            (atom_b_id, atom_a_id)
        };
        bond_entities.entities.insert(bond_key, bond_entity);
    }

    info!("Spawned {} bond entities", bond_entities.entities.len());
    spawned_events.send(BondsSpawnedEvent {
        count: bond_entities.entities.len(),
    });
}

/// Update bond positions when atoms move
pub fn update_bond_positions(
    atom_entities: Res<crate::systems::spawning::AtomEntities>,
    bond_entities: Res<BondEntities>,
    mut bond_query: Query<(&Bond, &mut Transform)>,
    atom_query: Query<&Atom>,
) {
    for (bond, mut transform) in bond_query.iter_mut() {
        // Get atom entities by ID
        let entity_a = match atom_entities.entities.get(&bond.atom_a_id) {
            Some(entity) => *entity,
            None => continue,
        };

        let entity_b = match atom_entities.entities.get(&bond.atom_b_id) {
            Some(entity) => *entity,
            None => continue,
        };

        // Get atom positions
        let pos_a = match atom_query.get(entity_a) {
            Ok(atom) => atom.position,
            Err(_) => continue,
        };

        let pos_b = match atom_query.get(entity_b) {
            Ok(atom) => atom.position,
            Err(_) => continue,
        };

        // Calculate new bond geometry
        let bond_vector = pos_b - pos_a;
        let bond_length = bond_vector.length();
        let bond_midpoint = pos_a + bond_vector * 0.5;

        // Update position
        transform.translation = bond_midpoint;

        // Update rotation
        if bond_length > 0.0001 {
            let up = Vec3::Y;
            let direction = bond_vector.normalize();
            let axis = up.cross(direction);
            let axis_length = axis.length();
            let angle = if axis_length > 0.0001 {
                axis_length.atan2(up.dot(direction))
            } else if up.dot(direction) < 0.0 {
                std::f32::consts::PI
            } else {
                0.0
            };
            transform.rotation = Quat::from_axis_angle(axis.normalize(), angle);
        }
    }
}

/// Clear all bond entities
pub fn despawn_all_bonds(
    mut commands: Commands,
    mut bond_entities: ResMut<BondEntities>,
    mut despawned_event: EventWriter<BondsDespawnedEvent>,
) {
    let count = bond_entities.entities.len();

    if count > 0 {
        info!("Despawning {} bonds", count);

        // Despawn all bond entities
        for (_, entity) in bond_entities.entities.drain() {
            commands.entity(entity).despawn_recursive();
        }

        despawned_event.send(BondsDespawnedEvent);
    }
}

/// Spawn bonds when atoms are loaded
pub fn spawn_bonds_on_load(
    mut commands: Commands,
    mut atoms_spawned_events: EventReader<crate::systems::spawning::AtomsSpawnedEvent>,
    mut bond_entities: ResMut<BondEntities>,
    mut config: ResMut<BondDetectionConfig>,
) {
    // Spawn bonds when atoms are spawned
    if !atoms_spawned_events.is_empty() && bond_entities.entities.is_empty() {
        // Enable bond detection by default
        config.enabled = true;
        info!("Bonds will be detected and spawned");
    }
}

/// Clear bonds when new file is loaded
pub fn clear_bonds_on_load(
    mut commands: Commands,
    mut bond_entities: ResMut<BondEntities>,
    mut file_loaded_events: EventReader<crate::systems::loading::FileLoadedEvent>,
    mut despawned_event: EventWriter<BondsDespawnedEvent>,
) {
    if !file_loaded_events.is_empty() && !bond_entities.entities.is_empty() {
        // Despawn all bonds
        for (_, entity) in bond_entities.entities.drain() {
            commands.entity(entity).despawn_recursive();
        }

        despawned_event.send(BondsDespawnedEvent);
        info!("Bonds cleared on file load");
    }
}

/// Register all bond systems
pub fn register(app: &mut App) {
    app.init_resource::<BondEntities>()
        .init_resource::<BondDetectionConfig>()
        .add_event::<BondsSpawnedEvent>()
        .add_event::<BondsDespawnedEvent>()
        .add_systems(Update, spawn_bonds_on_load)
        .add_systems(Update, spawn_bonds)
        .add_systems(Update, update_bond_positions)
        .add_systems(Update, clear_bonds_on_load);

    info!("Bond detection and rendering systems registered");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bond_detection_config() {
        let config = BondDetectionConfig::default();
        assert!(config.enabled);
        assert!(config.distance_multiplier > 0.0);
        assert!(config.max_bond_distance > config.min_bond_distance);
    }

    #[test]
    fn test_bond_entities() {
        let mut entities = BondEntities::default();
        assert!(entities.entities.is_empty());

        entities.entities.insert((0, 1), Entity::PLACEHOLDER);
        assert_eq!(entities.entities.len(), 1);
    }
}
