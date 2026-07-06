//! Topology helpers for pairing coordinate trajectories (DCD) with structure files.

use crate::core::atom::AtomData;
use crate::core::bond::BondData;
use std::collections::HashMap;

/// Renumber atoms to sequential 0..N-1 (DCD frame order) and remap bond endpoints.
pub fn normalize_topology(atom_data: Vec<AtomData>, bond_data: Vec<BondData>) -> (Vec<AtomData>, Vec<BondData>) {
    let id_map: HashMap<u32, u32> = atom_data
        .iter()
        .enumerate()
        .map(|(i, atom)| (atom.id, i as u32))
        .collect();

    let normalized_atoms: Vec<AtomData> = atom_data
        .into_iter()
        .enumerate()
        .map(|(i, mut atom)| {
            atom.id = i as u32;
            atom
        })
        .collect();

    let normalized_bonds: Vec<BondData> = bond_data
        .into_iter()
        .filter_map(|mut bond| {
            let atom_a = *id_map.get(&bond.atom_a_id)?;
            let atom_b = *id_map.get(&bond.atom_b_id)?;
            bond.atom_a_id = atom_a;
            bond.atom_b_id = atom_b;
            Some(bond)
        })
        .collect();

    (normalized_atoms, normalized_bonds)
}

/// Verify topology atom count matches a trajectory with only coordinates.
pub fn validate_atom_count(topology_len: usize, trajectory_atoms: usize) -> Result<(), String> {
    if topology_len != trajectory_atoms {
        Err(format!(
            "Topology atom count ({topology_len}) does not match trajectory ({trajectory_atoms})"
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::atom::Element;
    use crate::core::bond::{BondOrder, BondType, BondData};

    #[test]
    fn test_normalize_topology_remaps_bonds() {
        let atoms = vec![
            AtomData::new(10, Element::N, 1, "ALA".into(), "A".into(), "N".into()),
            AtomData::new(20, Element::C, 1, "ALA".into(), "A".into(), "CA".into()),
        ];
        let bonds = vec![BondData::new(
            10,
            20,
            BondType::Covalent,
            BondOrder::Single,
            1.5,
        )];

        let (norm_atoms, norm_bonds) = normalize_topology(atoms, bonds);
        assert_eq!(norm_atoms[0].id, 0);
        assert_eq!(norm_atoms[1].id, 1);
        assert_eq!(norm_bonds[0].atom_a_id, 0);
        assert_eq!(norm_bonds[0].atom_b_id, 1);
    }
}
