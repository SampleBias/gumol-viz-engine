//! Atom ID ↔ instanced GPU buffer mapping and position lookups.

use crate::core::atom::Element;
use crate::rendering::instanced::{InstancedAtomEntity, InstancedAtomMesh};
use bevy::prelude::*;
use std::collections::HashMap;

/// Maps atom IDs to their location inside per-element instance buffers.
#[derive(Resource, Default, Debug, Clone)]
pub struct InstancedAtomIndex {
    /// atom_id → (element, index within that element's instance vec)
    pub atom_to_instance: HashMap<u32, (Element, u32)>,
    /// element → ordered atom IDs (instance index → atom_id)
    pub element_atom_ids: HashMap<Element, Vec<u32>>,
}

impl InstancedAtomIndex {
    /// Build index from grouped atom spawn order.
    pub fn build(atoms_by_element: &HashMap<Element, Vec<u32>>) -> Self {
        let mut atom_to_instance = HashMap::new();
        let mut element_atom_ids = HashMap::new();

        for (element, atom_ids) in atoms_by_element {
            let mut ids = Vec::with_capacity(atom_ids.len());
            for (idx, &atom_id) in atom_ids.iter().enumerate() {
                atom_to_instance.insert(atom_id, (*element, idx as u32));
                ids.push(atom_id);
            }
            element_atom_ids.insert(*element, ids);
        }

        Self {
            atom_to_instance,
            element_atom_ids,
        }
    }

    pub fn clear(&mut self) {
        self.atom_to_instance.clear();
        self.element_atom_ids.clear();
    }

    pub fn atom_id_at(&self, element: Element, instance_idx: u32) -> Option<u32> {
        self.element_atom_ids
            .get(&element)
            .and_then(|ids| ids.get(instance_idx as usize).copied())
    }

    /// Collect current world positions for all indexed atoms.
    pub fn collect_positions(
        &self,
        instanced: &Query<(&InstancedAtomEntity, &InstancedAtomMesh)>,
    ) -> HashMap<u32, Vec3> {
        let mut positions = HashMap::with_capacity(self.atom_to_instance.len());

        for (element, atom_ids) in &self.element_atom_ids {
            let mesh = instanced
                .iter()
                .find(|(e, _)| e.element == *element)
                .map(|(_, m)| m);

            let Some(mesh) = mesh else {
                continue;
            };

            for (idx, &atom_id) in atom_ids.iter().enumerate() {
                if let Some(instance) = mesh.instances.get(idx) {
                    positions.insert(atom_id, instance.position);
                }
            }
        }

        positions
    }

    /// Look up a single atom position.
    pub fn get_position(
        &self,
        atom_id: u32,
        instanced: &Query<(&InstancedAtomEntity, &InstancedAtomMesh)>,
    ) -> Option<Vec3> {
        let (element, idx) = self.atom_to_instance.get(&atom_id)?;
        for (entity_info, mesh) in instanced.iter() {
            if entity_info.element == *element {
                return mesh.instances.get(*idx as usize).map(|i| i.position);
            }
        }
        None
    }
}
