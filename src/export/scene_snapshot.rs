//! Centralized scene data collection for export formats.

use crate::core::bond::Bond;
use crate::rendering::atom_index::InstancedAtomIndex;
use crate::rendering::instanced::{InstancedAtomEntity, InstancedAtomMesh};
use bevy::prelude::*;

/// Snapshot of atom positions and radii for export.
#[derive(Debug, Clone)]
pub struct AtomSnapshot {
    pub position: Vec3,
    pub radius: f32,
    pub color: [f32; 3],
}

/// Snapshot of a bond cylinder for export.
#[derive(Debug, Clone)]
pub struct BondSnapshot {
    pub translation: Vec3,
    pub rotation: Quat,
    pub length: f32,
    pub radius: f32,
}

/// Complete exportable scene state.
#[derive(Debug, Clone, Default)]
pub struct SceneSnapshot {
    pub atoms: Vec<AtomSnapshot>,
    pub bonds: Vec<BondSnapshot>,
}

/// Build export snapshot from instanced rendering + bond entities.
pub fn capture_scene(
    index: &InstancedAtomIndex,
    instanced: &Query<(&InstancedAtomEntity, &InstancedAtomMesh)>,
    atom_data: &[crate::core::atom::AtomData],
    bond_query: &Query<(&Transform, &Bond)>,
    bond_entities: &crate::systems::bonds::BondEntities,
    viz_config: &crate::core::visualization::VisualizationConfig,
) -> SceneSnapshot {
    let mut snapshot = SceneSnapshot::default();
    let mode_scale = viz_config.render_mode.atom_scale() * viz_config.atom_scale;

    let atom_lookup: std::collections::HashMap<u32, &crate::core::atom::AtomData> =
        atom_data.iter().map(|a| (a.id, a)).collect();

    for (element, atom_ids) in &index.element_atom_ids {
        let Some((_, mesh)) = instanced.iter().find(|(e, _)| e.element == *element) else {
            continue;
        };

        for (idx, &atom_id) in atom_ids.iter().enumerate() {
            let Some(instance) = mesh.instances.get(idx) else {
                continue;
            };

            if instance.scale <= 0.001 {
                continue;
            }

            let vdw = atom_lookup
                .get(&atom_id)
                .map(|a| a.element.vdw_radius())
                .unwrap_or(element.vdw_radius());

            let radius = vdw * 0.5 * instance.scale * mode_scale;
            let color = atom_lookup
                .get(&atom_id)
                .map(|a| a.element.cpk_color())
                .unwrap_or(element.cpk_color());

            snapshot.atoms.push(AtomSnapshot {
                position: instance.position,
                radius,
                color,
            });
        }
    }

    for (_, entity) in bond_entities.entities.iter() {
        if let Ok((transform, bond)) = bond_query.get(*entity) {
            snapshot.bonds.push(BondSnapshot {
                translation: transform.translation,
                rotation: transform.rotation,
                length: bond.length,
                radius: 0.1 * viz_config.bond_scale,
            });
        }
    }

    snapshot
}
