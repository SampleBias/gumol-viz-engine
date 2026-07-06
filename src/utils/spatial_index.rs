//! R-tree spatial index for O(log N) neighbor queries (bond detection).

use crate::core::atom::{AtomData, Element};
use bevy::prelude::*;
use rstar::{RTree, RTreeObject, AABB};
use std::collections::HashMap;

/// One atom entry in the spatial index.
#[derive(Clone, Copy, Debug)]
pub struct IndexedAtom {
    pub atom_id: u32,
    pub element: Element,
    pub residue_id: u32,
    pub position: [f32; 3],
}

impl RTreeObject for IndexedAtom {
    type Envelope = AABB<[f32; 3]>;

    fn envelope(&self) -> Self::Envelope {
        AABB::from_point(self.position)
    }
}

/// Spatial index rebuilt when atom positions change.
#[derive(Resource, Default, Debug)]
pub struct AtomSpatialIndex {
    tree: Option<RTree<IndexedAtom>>,
    pub atom_count: usize,
}

impl AtomSpatialIndex {
    pub fn build(atom_data: &[AtomData], positions: &HashMap<u32, Vec3>) -> Self {
        let entries: Vec<IndexedAtom> = atom_data
            .iter()
            .filter_map(|a| {
                let pos = positions.get(&a.id)?;
                Some(IndexedAtom {
                    atom_id: a.id,
                    element: a.element,
                    residue_id: a.residue_id,
                    position: [pos.x, pos.y, pos.z],
                })
            })
            .collect();

        let count = entries.len();
        let tree = if entries.is_empty() {
            None
        } else {
            Some(RTree::bulk_load(entries))
        };

        Self {
            tree,
            atom_count: count,
        }
    }

    pub fn clear(&mut self) {
        self.tree = None;
        self.atom_count = 0;
    }

    /// Find all atom IDs within `radius` of `center`.
    pub fn neighbors_within(&self, center: Vec3, radius: f32) -> Vec<u32> {
        let Some(tree) = &self.tree else {
            return Vec::new();
        };

        let envelope = AABB::from_corners(
            [center.x - radius, center.y - radius, center.z - radius],
            [center.x + radius, center.y + radius, center.z + radius],
        );

        tree.locate_in_envelope_intersecting(&envelope)
            .filter(|a| {
                let dx = a.position[0] - center.x;
                let dy = a.position[1] - center.y;
                let dz = a.position[2] - center.z;
                (dx * dx + dy * dy + dz * dz).sqrt() <= radius
            })
            .map(|a| a.atom_id)
            .collect()
    }

    pub fn is_built(&self) -> bool {
        self.tree.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_atoms(n: usize) -> (Vec<AtomData>, HashMap<u32, Vec3>) {
        let mut atoms = Vec::new();
        let mut positions = HashMap::new();
        for i in 0..n {
            let id = i as u32;
            atoms.push(AtomData::new(
                id,
                Element::C,
                0,
                "UNK".into(),
                "A".into(),
                format!("C{i}"),
            ));
            positions.insert(id, Vec3::new(i as f32 * 1.5, 0.0, 0.0));
        }
        (atoms, positions)
    }

    #[test]
    fn test_spatial_neighbors() {
        let (atoms, positions) = sample_atoms(10);
        let index = AtomSpatialIndex::build(&atoms, &positions);
        let neighbors = index.neighbors_within(Vec3::ZERO, 2.0);
        assert!(!neighbors.is_empty());
        assert!(neighbors.contains(&0));
    }
}
