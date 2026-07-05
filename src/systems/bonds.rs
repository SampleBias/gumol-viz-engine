//! Bond detection and rendering system
//!
//! Detects bonds from file topology or distance heuristics and renders
//! cylinder meshes synced to instanced atom positions.

use crate::core::atom::Element;
use crate::core::bond::{Bond, BondData, BondOrder, BondType};
use crate::core::visualization::VisualizationConfig;
use crate::rendering;
use crate::rendering::atom_index::InstancedAtomIndex;
use crate::rendering::instanced::{InstancedAtomEntity, InstancedAtomMesh};
use bevy::prelude::*;
use std::collections::HashMap;

/// Maximum atoms for O(N²) distance-based bond detection without spatial index.
const MAX_NAIVE_BOND_ATOMS: usize = 5_000;

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

#[derive(Event, Debug)]
pub struct BondsSpawnedEvent {
    pub count: usize,
}

#[derive(Event, Debug)]
pub struct BondsDespawnedEvent;

fn bond_key(a: u32, b: u32) -> (u32, u32) {
    if a < b {
        (a, b)
    } else {
        (b, a)
    }
}

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

fn detect_bonds_from_distance(
    sim_data: &crate::systems::loading::SimulationData,
    positions: &HashMap<u32, Vec3>,
    config: &BondDetectionConfig,
) -> Vec<BondData> {
    if sim_data.atom_data.len() > MAX_NAIVE_BOND_ATOMS {
        warn!(
            "Skipping distance bond detection for {} atoms (limit {})",
            sim_data.atom_data.len(),
            MAX_NAIVE_BOND_ATOMS
        );
        return Vec::new();
    }

    let mut bonds = Vec::new();
    let atoms = &sim_data.atom_data;

    for i in 0..atoms.len() {
        let a = &atoms[i];
        let Some(pos_a) = positions.get(&a.id) else {
            continue;
        };

        for b in atoms.iter().skip(i + 1) {
            let Some(pos_b) = positions.get(&b.id) else {
                continue;
            };

            let distance = pos_a.distance(*pos_b);
            if config.should_bond(a.element, b.element, a.residue_id, b.residue_id, distance) {
                bonds.push(BondData::new(
                    a.id,
                    b.id,
                    config.determine_bond_type(a.element, b.element),
                    config.determine_bond_order(a.element, b.element, distance),
                    distance,
                ));
            }
        }
    }

    bonds
}

fn dedupe_bonds(bonds: Vec<BondData>) -> Vec<BondData> {
    let mut seen = HashMap::new();
    for bond in bonds {
        seen.entry(bond_key(bond.atom_a_id, bond.atom_b_id)).or_insert(bond);
    }
    seen.into_values().collect()
}

/// Resolve the bond list from file topology or distance detection.
pub fn resolve_bond_list(
    sim_data: &crate::systems::loading::SimulationData,
    positions: &HashMap<u32, Vec3>,
    config: &BondDetectionConfig,
) -> Vec<BondData> {
    let bonds = if !sim_data.bond_data.is_empty() {
        sim_data.bond_data.clone()
    } else if config.enabled {
        detect_bonds_from_distance(sim_data, positions, config)
    } else {
        Vec::new()
    };
    dedupe_bonds(bonds)
}

/// Spawn bond cylinders after instanced atoms are ready.
pub fn spawn_bonds(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    sim_data: Res<crate::systems::loading::SimulationData>,
    viz_config: Res<VisualizationConfig>,
    index: Res<InstancedAtomIndex>,
    instanced: Query<(&InstancedAtomEntity, &InstancedAtomMesh)>,
    mut bond_entities: ResMut<BondEntities>,
    config: Res<BondDetectionConfig>,
    spawned_events: EventReader<crate::rendering::instanced::InstancedAtomsSpawnedEvent>,
    mut bond_spawned: EventWriter<BondsSpawnedEvent>,
) {
    if spawned_events.is_empty() || !bond_entities.entities.is_empty() {
        return;
    }

    if !config.enabled || !sim_data.loaded || index.atom_to_instance.is_empty() {
        return;
    }

    let positions = index.collect_positions(&instanced);

    let bonds = resolve_bond_list(&sim_data, &positions, &config);

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

    let base_radius = 0.1;

    for bond_data in bonds {
        let Some(pos_a) = positions.get(&bond_data.atom_a_id) else {
            continue;
        };
        let Some(pos_b) = positions.get(&bond_data.atom_b_id) else {
            continue;
        };

        let bond_vector = *pos_b - *pos_a;
        let bond_length = bond_vector.length();
        if bond_length < config.min_bond_distance {
            continue;
        }

        let bond_midpoint = *pos_a + bond_vector * 0.5;
        let bond_mesh = meshes.add(rendering::generate_bond_mesh(bond_length, base_radius));
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
                    visibility: if viz_config.show_bonds {
                        Visibility::Visible
                    } else {
                        Visibility::Hidden
                    },
                    ..default()
                },
                Bond {
                    atom_a: Entity::PLACEHOLDER,
                    atom_b: Entity::PLACEHOLDER,
                    atom_a_id: bond_data.atom_a_id,
                    atom_b_id: bond_data.atom_b_id,
                    bond_type: bond_data.bond_type,
                    order: bond_data.order,
                    length: bond_length,
                },
            ))
            .id();

        bond_entities
            .entities
            .insert(bond_key(bond_data.atom_a_id, bond_data.atom_b_id), bond_entity);
    }

    info!("Spawned {} bond entities", bond_entities.entities.len());
    bond_spawned.send(BondsSpawnedEvent {
        count: bond_entities.entities.len(),
    });
}

/// Update bond transforms from instanced atom positions.
pub fn update_bond_positions(
    index: Res<InstancedAtomIndex>,
    instanced: Query<(&InstancedAtomEntity, &InstancedAtomMesh)>,
    mut bond_query: Query<(&Bond, &mut Transform)>,
) {
    if index.atom_to_instance.is_empty() {
        return;
    }

    let positions = index.collect_positions(&instanced);

    for (bond, mut transform) in bond_query.iter_mut() {
        let Some(pos_a) = positions.get(&bond.atom_a_id) else {
            continue;
        };
        let Some(pos_b) = positions.get(&bond.atom_b_id) else {
            continue;
        };

        let bond_vector = *pos_b - *pos_a;
        let bond_length = bond_vector.length();

        transform.translation = *pos_a + bond_vector * 0.5;
        transform.rotation = compute_bond_rotation(bond_vector, bond_length);
    }
}

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
    fn test_compute_bond_rotation_identity() {
        let rot = compute_bond_rotation(Vec3::Y, 1.0);
        let diff = rot.angle_between(Quat::IDENTITY);
        assert!(diff < 0.001, "Y-aligned bond should give identity rotation");
    }
}
