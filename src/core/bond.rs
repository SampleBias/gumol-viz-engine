//! Bond component and data structures

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Bond component attached to Bevy entities
#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component)]
pub struct Bond {
    /// First atom entity
    pub atom_a: Entity,
    /// Second atom entity
    pub atom_b: Entity,
    /// Bond type
    pub bond_type: BondType,
    /// Bond order (1 = single, 2 = double, 3 = triple)
    pub order: u8,
    /// Bond length (in Angstroms)
    pub length: f32,
}

impl Default for Bond {
    fn default() -> Self {
        Self {
            atom_a: Entity::PLACEHOLDER,
            atom_b: Entity::PLACEHOLDER,
            bond_type: BondType::Unknown,
            order: 1,
            length: 0.0,
        }
    }
}

/// Static bond data (loaded once)
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
#[reflect(Debug, Serialize, Deserialize)]
pub struct BondData {
    /// First atom ID
    pub atom_a_id: u32,
    /// Second atom ID
    pub atom_b_id: u32,
    /// Bond type
    pub bond_type: BondType,
    /// Bond order
    pub order: u8,
}

impl BondData {
    /// Create a new bond data structure
    pub fn new(atom_a_id: u32, atom_b_id: u32, bond_type: BondType, order: u8) -> Self {
        Self {
            atom_a_id,
            atom_b_id,
            bond_type,
            order,
        }
    }
}

/// Bond type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
#[reflect(Debug, PartialEq, Hash)]
pub enum BondType {
    /// Covalent bond
    Covalent,
    /// Hydrogen bond
    Hydrogen,
    /// Ionic bond
    Ionic,
    /// Van der Waals interaction
    VanDerWaals,
    /// Pi bond
    Pi,
    /// Metal coordinate bond
    MetalCoord,
    /// Disulfide bond
    Disulfide,
    /// Peptide bond
    Peptide,
    /// Unknown bond type
    Unknown,
}

impl Default for BondType {
    fn default() -> Self {
        BondType::Unknown
    }
}

/// Bond length lookup table for element pairs (in Angstroms)
pub struct BondLengths;

impl BondLengths {
    /// Get the typical bond length between two elements
    pub fn get_length(element_a: crate::core::atom::Element, element_b: crate::core::atom::Element) -> f32 {
        // Sort elements to ensure consistent lookup
        let (e1, e2) = if (element_a as u32) < (element_b as u32) {
            (element_a, element_b)
        } else {
            (element_b, element_a)
        };

        match (e1, e2) {
            // Single bonds
            (crate::core::atom::Element::H, crate::core::atom::Element::H) => 0.74,
            (crate::core::atom::Element::H, crate::core::atom::Element::C) => 1.09,
            (crate::core::atom::Element::H, crate::core::atom::Element::N) => 1.01,
            (crate::core::atom::Element::H, crate::core::atom::Element::O) => 0.96,
            (crate::core::atom::Element::C, crate::core::atom::Element::C) => 1.54,
            (crate::core::atom::Element::C, crate::core::atom::Element::N) => 1.47,
            (crate::core::atom::Element::C, crate::core::atom::Element::O) => 1.43,
            (crate::core::atom::Element::C, crate::core::atom::Element::S) => 1.82,
            (crate::core::atom::Element::N, crate::core::atom::Element::N) => 1.45,
            (crate::core::atom::Element::N, crate::core::atom::Element::O) => 1.36,
            (crate::core::atom::Element::O, crate::core::atom::Element::O) => 1.48,
            (crate::core::atom::Element::S, crate::core::atom::Element::S) => 2.05,
            // Metal bonds
            (crate::core::atom::Element::Fe, crate::core::atom::Element::S) => 2.30,
            (crate::core::atom::Element::Zn, crate::core::atom::Element::S) => 2.34,
            // Default to sum of van der Waals radii / 2
            _ => (element_a.vdw_radius() + element_b.vdw_radius()) / 2.0 * 0.75,
        }
    }

    /// Get the covalent radius for an element
    pub fn covalent_radius(element: crate::core::atom::Element) -> f32 {
        match element {
            crate::core::atom::Element::H => 0.31,
            crate::core::atom::Element::C => 0.76,
            crate::core::atom::Element::N => 0.71,
            crate::core::atom::Element::O => 0.66,
            crate::core::atom::Element::F => 0.57,
            crate::core::atom::Element::P => 1.07,
            crate::core::atom::Element::S => 1.05,
            crate::core::atom::Element::Cl => 1.02,
            crate::core::atom::Element::Br => 1.20,
            crate::core::atom::Element::I => 1.39,
            _ => 1.0,
        }
    }
}

/// Detect bonds between atoms based on distance
pub fn detect_bonds(
    atoms: &HashMap<u32, Vec3>,
    atom_data: &HashMap<u32, crate::core::atom::AtomData>,
    tolerance: f32,
) -> Vec<BondData> {
    let mut bonds = Vec::new();
    let atom_ids: Vec<u32> = atoms.keys().copied().collect();

    for i in 0..atom_ids.len() {
        for j in (i + 1)..atom_ids.len() {
            let id_a = atom_ids[i];
            let id_b = atom_ids[j];

            if let (Some(pos_a), Some(pos_b)) = (atoms.get(&id_a), atoms.get(&id_b)) {
                let distance = pos_a.distance(*pos_b);

                if let (Some(atom_a), Some(atom_b)) = (atom_data.get(&id_a), atom_data.get(&id_b)) {
                    let expected_length = BondLengths::get_length(atom_a.element, atom_b.element);
                    let max_distance = expected_length * (1.0 + tolerance);

                    if distance <= max_distance {
                        // Determine bond order based on distance
                        let order = if distance < expected_length * 0.9 {
                            3 // triple bond (very short)
                        } else if distance < expected_length * 0.95 {
                            2 // double bond
                        } else {
                            1 // single bond
                        };

                        bonds.push(BondData::new(id_a, id_b, BondType::Covalent, order));
                    }
                }
            }
        }
    }

    bonds
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::atom::{Element, AtomData};

    #[test]
    fn test_bond_length_lookup() {
        let length = BondLengths::get_length(Element::C, Element::H);
        assert!((length - 1.09).abs() < 0.01);

        let length = BondLengths::get_length(Element::O, Element::H);
        assert!((length - 0.96).abs() < 0.01);
    }

    #[test]
    fn test_detect_bonds() {
        let mut atoms = HashMap::new();
        atoms.insert(0, glam::Vec3::new(0.0, 0.0, 0.0));
        atoms.insert(1, glam::Vec3::new(1.09, 0.0, 0.0));

        let mut atom_data = HashMap::new();
        atom_data.insert(0, AtomData::new(0, Element::C, 0, "MET".into(), "A".into(), "CA".into()));
        atom_data.insert(1, AtomData::new(1, Element::H, 0, "MET".into(), "A".into(), "H1".into()));

        let bonds = detect_bonds(&atoms, &atom_data, 0.2);
        assert_eq!(bonds.len(), 1);
        assert_eq!(bonds[0].atom_a_id, 0);
        assert_eq!(bonds[0].atom_b_id, 1);
    }
}
