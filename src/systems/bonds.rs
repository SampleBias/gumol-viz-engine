//! Bond detection and rendering system
//!
//! This system detects bonds between atoms based on distance and
//! spawns bond entities with cylindrical meshes.

use crate::core::atom::Element;
use crate::core::bond::{Bond, BondData, BondOrder, BondType};
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
    /// Check if two atoms should be bonded based on element, distance, and residue
    pub fn should_bond(
        &self,
        element_a: Element,
        element_b: Element,
        residue_a: u32,
        residue_b: u32,
        distance: f32,
    ) -> bool {
        if distance < self.min_bond_distance || distance > self.max_bond_distance {
            return false;
        }

        let vdw_sum = element_a.vdw_radius() + element_b.vdw_radius();
        if distance > vdw_sum * self.distance_multiplier {
            return false;
        }

        if self.same_residue_only && residue_a != residue_b {
            return false;
        }

        true
    }

    /// Determine bond order based on element types and distance
    pub fn determine_bond_order(
        &self,
        element_a: Element,
        element_b: Element,
        distance: f32,
    ) -> BondOrder {
        let expected_length = crate::core::bond::BondLengths::get_length(element_a, element_b);

        if distance < expected_length * 0.9 {
            BondOrder::Triple
        } else if distance < expected_length * 0.95 {
            BondOrder::Double
        } else {
            BondOrder::Single
        }
    }

    /// Determine bond type based on element types
    pub fn determine_bond_type(&self, element_a: Element, element_b: Element) -> BondType {
        match (element_a.symbol(), element_b.symbol()) {
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
    pub count: usize,
}

/// Event sent when bonds are despawned
#[derive(Event, Debug)]
pub struct BondsDespawnedEvent;

/// Lightweight atom snapshot for bond detection (avoids borrow conflicts)
#[derive(Clone, Debug)]
struct AtomSnapshot {
    id: u32,
    entity: Entity,
    element: Element,
    position: Vec3,
    residue_id: u32,
}

/// Detect bonds between atoms. Only runs when bond_entities is empty and atoms exist.
pub fn spawn_bonds(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    atom_entities: Res<crate::systems::spawning::AtomEntities>,
    mut bond_entities: ResMut<BondEntities>,
    atom_query: Query<(&crate::core::atom::Atom, &Transform)>,
    config: Res<BondDetectionConfig>,
    mut spawned_events: EventWriter<BondsSpawnedEvent>,
) {
    if !config.enabled || atom_entities.entities.is_empty() || !bond_entities.entities.is_empty() {
        return;
    }

    let mut snapshots = Vec::with_capacity(atom_entities.entities.len());
    for (&atom_id, &entity) in atom_entities.entities.iter() {
        if let Ok((atom, transform)) = atom_query.get(entity) {
            snapshots.push(AtomSnapshot {
                id: atom_id,
                entity,
                element: atom.element,
                position: transform.translation,
                residue_id: atom.residue_id,
            });
        }
    }

    let mut bonds: Vec<(BondData, Entity, Entity)> = Vec::new();

    for (i, a) in snapshots.iter().enumerate() {
        for b in snapshots.iter().skip(i + 1) {
            let distance = a.position.distance(b.position);

            if config.should_bond(a.element, b.element, a.residue_id, b.residue_id, distance) {
                let bond_order = config.determine_bond_order(a.element, b.element, distance);
                let bond_type = config.determine_bond_type(a.element, b.element);

                let bond_data = BondData::new(a.id, b.id, bond_type, bond_order, distance);
                bonds.push((bond_data, a.entity, b.entity));
            }
        }
    }

    if bonds.is_empty() {
        return;
    }

    info!("Spawning {} bonds...", bonds.len());

    let bond_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.6, 0.6, 0.6),
        metallic: 0.2,
        perceptual_roughness: 0.4,
        ..default()
    });

    for (bond_data, entity_a, entity_b) in bonds {
        let pos_a = match snapshots.iter().find(|s| s.id == bond_data.atom_a_id) {
            Some(s) => s.position,
            None => continue,
        };
        let pos_b = match snapshots.iter().find(|s| s.id == bond_data.atom_b_id) {
            Some(s) => s.position,
            None => continue,
        };

        let bond_vector = pos_b - pos_a;
        let bond_length = bond_vector.length();
        let bond_midpoint = pos_a + bond_vector * 0.5;

        let bond_radius = 0.1;
        let bond_mesh = meshes.add(rendering::generate_bond_mesh(bond_length, bond_radius));

        let rotation = compute_bond_rotation(bond_vector, bond_length);

        let bond_entity = commands
            .spawn((
                PbrBundle {
                    mesh: bond_mesh,
                    material: bond_material.clone(),
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
                    atom_a_id: bond_data.atom_a_id,
                    atom_b_id: bond_data.atom_b_id,
                    bond_type: bond_data.bond_type,
                    order: bond_data.order,
                    length: bond_length,
                },
            ))
            .id();

        let bond_key = if bond_data.atom_a_id < bond_data.atom_b_id {
            (bond_data.atom_a_id, bond_data.atom_b_id)
        } else {
            (bond_data.atom_b_id, bond_data.atom_a_id)
        };
        bond_entities.entities.insert(bond_key, bond_entity);
    }

    info!("Spawned {} bond entities", bond_entities.entities.len());
    spawned_events.send(BondsSpawnedEvent {
        count: bond_entities.entities.len(),
    });
}

/// Update bond positions when atoms move (reads from Transform, not Atom.position)
pub fn update_bond_positions(
    atom_entities: Res<crate::systems::spawning::AtomEntities>,
    mut param_set: ParamSet<(
        Query<(&Bond, &mut Transform)>,
        Query<&Transform, With<crate::systems::spawning::SpawnedAtom>>,
    )>,
) {
    // Step 1: collect atom positions from their Transforms
    let mut atom_positions: HashMap<u32, Vec3> = HashMap::new();
    {
        let atom_transforms = param_set.p1();
        for (&atom_id, &entity) in atom_entities.entities.iter() {
            if let Ok(transform) = atom_transforms.get(entity) {
                atom_positions.insert(atom_id, transform.translation);
            }
        }
    }

    // Step 2: update bond transforms using the collected positions
    {
        let mut bond_query = param_set.p0();
        for (bond, mut transform) in bond_query.iter_mut() {
            let pos_a = match atom_positions.get(&bond.atom_a_id) {
                Some(pos) => *pos,
                None => continue,
            };
            let pos_b = match atom_positions.get(&bond.atom_b_id) {
                Some(pos) => *pos,
                None => continue,
            };

            let bond_vector = pos_b - pos_a;
            let bond_length = bond_vector.length();

            transform.translation = pos_a + bond_vector * 0.5;
            transform.rotation = compute_bond_rotation(bond_vector, bond_length);
        }
    }
}

/// Compute rotation quaternion to align a Y-axis cylinder with a bond vector
fn compute_bond_rotation(bond_vector: Vec3, bond_length: f32) -> Quat {
    if bond_length < 0.0001 {
        return Quat::IDENTITY;
    }

    let up = Vec3::Y;
    let direction = bond_vector.normalize();
    let axis = up.cross(direction);
    let axis_length = axis.length();

    if axis_length > 0.0001 {
        let angle = axis_length.atan2(up.dot(direction));
        Quat::from_axis_angle(axis.normalize(), angle)
    } else if up.dot(direction) < 0.0 {
        Quat::from_rotation_x(std::f32::consts::PI)
    } else {
        Quat::IDENTITY
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

        for (_, entity) in bond_entities.entities.drain() {
            commands.entity(entity).despawn_recursive();
        }

        despawned_event.send(BondsDespawnedEvent);
    }
}

/// Clear bonds when new file is loaded
pub fn clear_bonds_on_load(
    mut commands: Commands,
    mut bond_entities: ResMut<BondEntities>,
    file_loaded_events: EventReader<crate::systems::loading::FileLoadedEvent>,
    mut despawned_event: EventWriter<BondsDespawnedEvent>,
) {
    if !file_loaded_events.is_empty() && !bond_entities.entities.is_empty() {
        for (_, entity) in bond_entities.entities.drain() {
            commands.entity(entity).despawn_recursive();
        }

        despawned_event.send(BondsDespawnedEvent);
        info!("Bonds cleared on file load");
    }
}

/// Register bond resources and events. Systems are registered centrally in systems::register.
pub fn register(app: &mut App) {
    app.init_resource::<BondEntities>()
        .init_resource::<BondDetectionConfig>()
        .add_event::<BondsSpawnedEvent>()
        .add_event::<BondsDespawnedEvent>();

    info!("Bond resources registered");
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
    fn test_should_bond_within_range() {
        let config = BondDetectionConfig::default();
        let bonded = config.should_bond(Element::C, Element::C, 0, 0, 1.54);
        assert!(bonded, "C-C at 1.54 A should be bonded");
    }

    #[test]
    fn test_should_bond_too_far() {
        let config = BondDetectionConfig::default();
        let bonded = config.should_bond(Element::C, Element::C, 0, 0, 5.0);
        assert!(!bonded, "C-C at 5.0 A should NOT be bonded");
    }

    #[test]
    fn test_should_bond_same_residue_only() {
        let config = BondDetectionConfig {
            same_residue_only: true,
            ..Default::default()
        };
        let bonded = config.should_bond(Element::C, Element::N, 0, 1, 1.47);
        assert!(!bonded, "Different residues should not bond when same_residue_only");
    }

    #[test]
    fn test_bond_type_disulfide() {
        let config = BondDetectionConfig::default();
        assert_eq!(
            config.determine_bond_type(Element::S, Element::S),
            BondType::Disulfide
        );
    }

    #[test]
    fn test_bond_entities() {
        let mut entities = BondEntities::default();
        assert!(entities.entities.is_empty());

        entities.entities.insert((0, 1), Entity::PLACEHOLDER);
        assert_eq!(entities.entities.len(), 1);
    }

    #[test]
    fn test_compute_bond_rotation_identity() {
        let rot = compute_bond_rotation(Vec3::Y, 1.0);
        let diff = rot.angle_between(Quat::IDENTITY);
        assert!(diff < 0.001, "Y-aligned bond should give identity rotation");
    }

    #[test]
    fn test_compute_bond_rotation_flipped() {
        let rot = compute_bond_rotation(-Vec3::Y, 1.0);
        let diff = rot.angle_between(Quat::from_rotation_x(std::f32::consts::PI));
        assert!(diff < 0.001, "Negative Y should give PI rotation");
    }
}
