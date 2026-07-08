//! Protein backbone extraction and secondary-structure assignment.
//!
//! Uses DSSP (Kabsch-Sander hydrogen-bond model via [`pdbrust`]) when backbone
//! atoms (N, CA, C, O) are available; falls back to a distance-based heuristic
//! for CA-only or incomplete structures.

use crate::analysis::dssp;
use crate::core::atom::AtomData;
use crate::core::molecule::SecondaryStructure;
use bevy::prelude::*;
use std::collections::HashMap;
use tracing::debug;

/// Minimum CA atoms required before cartoon modes are offered in the UI.
pub const MIN_CARTOON_RESIDUES: usize = 20;

/// One residue along a protein backbone (CA trace).
#[derive(Debug, Clone)]
pub struct BackboneResidue {
    pub residue_id: u32,
    pub ca_atom_id: u32,
    pub chain_id: String,
    pub position: Vec3,
    pub secondary_structure: SecondaryStructure,
}

/// Extracted protein backbone used by ribbon rendering.
#[derive(Resource, Default, Debug, Clone)]
pub struct ProteinBackbone {
    pub residues: Vec<BackboneResidue>,
    pub ca_count: usize,
    pub cartoon_available: bool,
}

impl ProteinBackbone {
    pub fn clear(&mut self) {
        self.residues.clear();
        self.ca_count = 0;
        self.cartoon_available = false;
    }
}

/// Extract CA backbone atoms and assign secondary structure (DSSP or heuristic).
pub fn build_protein_backbone(
    atom_data: &[AtomData],
    positions: &HashMap<u32, Vec3>,
) -> ProteinBackbone {
    let mut ca_atoms: Vec<&AtomData> = atom_data
        .iter()
        .filter(|a| a.name.eq_ignore_ascii_case("CA"))
        .filter(|a| positions.contains_key(&a.id))
        .collect();

    ca_atoms.sort_by(|a, b| {
        a.chain_id
            .cmp(&b.chain_id)
            .then(a.residue_id.cmp(&b.residue_id))
    });

    let ca_count = ca_atoms.len();
    let mut residues: Vec<BackboneResidue> = ca_atoms
        .into_iter()
        .map(|a| BackboneResidue {
            residue_id: a.residue_id,
            ca_atom_id: a.id,
            chain_id: a.chain_id.clone(),
            position: positions[&a.id],
            secondary_structure: SecondaryStructure::Coil,
        })
        .collect();

    apply_secondary_structure(atom_data, positions, &mut residues);

    ProteinBackbone {
        ca_count,
        cartoon_available: ca_count >= MIN_CARTOON_RESIDUES,
        residues,
    }
}

/// Assign secondary structure via DSSP, falling back to a distance heuristic.
fn apply_secondary_structure(
    atom_data: &[AtomData],
    positions: &HashMap<u32, Vec3>,
    residues: &mut [BackboneResidue],
) {
    let dssp_result = dssp::assign_dssp(atom_data, positions);
    if dssp_result.used_dssp {
        for residue in residues.iter_mut() {
            let key = (residue.chain_id.clone(), residue.residue_id);
            if let Some(ss) = dssp_result.assignments.get(&key) {
                residue.secondary_structure = *ss;
            }
        }
        debug!(
            "DSSP assigned secondary structure to {} residues",
            dssp_result.assignments.len()
        );
        for warning in &dssp_result.warnings {
            debug!("DSSP: {warning}");
        }
        return;
    }

    debug!(
        "DSSP unavailable ({:?}); using distance heuristic",
        dssp_result.warnings
    );
    assign_secondary_structure_heuristic(residues);
}

/// Distance-based secondary structure assignment (helix / sheet / coil).
pub fn assign_secondary_structure_heuristic(residues: &mut [BackboneResidue]) {
    let n = residues.len();
    if n < 4 {
        return;
    }

    let mut ss = vec![SecondaryStructure::Coil; n];

    for i in 0..n {
        if i + 4 < n && same_chain(&residues[i], &residues[i + 4]) {
            let d13 = residues[i].position.distance(residues[i + 3].position);
            let d14 = residues[i].position.distance(residues[i + 4].position);
            if d13 < 6.0 && d14 < 7.0 {
                for ss_j in ss.iter_mut().skip(i).take(4) {
                    *ss_j = SecondaryStructure::AlphaHelix;
                }
            }
        }
    }

    for i in 0..n {
        if ss[i] != SecondaryStructure::Coil {
            continue;
        }
        if i + 2 < n && same_chain(&residues[i], &residues[i + 2]) {
            let d12 = residues[i].position.distance(residues[i + 1].position);
            let d13 = residues[i].position.distance(residues[i + 2].position);
            if (3.5..=4.2).contains(&d12) && (6.0..=7.5).contains(&d13) {
                ss[i] = SecondaryStructure::BetaStrand;
                ss[i + 1] = SecondaryStructure::BetaStrand;
                ss[i + 2] = SecondaryStructure::BetaStrand;
            }
        }
    }

    for (residue, structure) in residues.iter_mut().zip(ss) {
        residue.secondary_structure = structure;
    }
}

fn same_chain(a: &BackboneResidue, b: &BackboneResidue) -> bool {
    a.chain_id == b.chain_id
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::atom::{AtomData, Element};

    fn ca(id: u32, residue_id: u32, pos: Vec3) -> AtomData {
        let mut atom = AtomData::new(
            id,
            Element::C,
            residue_id,
            "ALA".into(),
            "A".into(),
            "CA".into(),
        );
        atom.position = pos;
        atom
    }

    #[test]
    fn test_build_backbone_counts_ca() {
        let atoms = vec![
            ca(0, 1, Vec3::ZERO),
            ca(1, 2, Vec3::X * 3.8),
            ca(2, 3, Vec3::X * 7.6),
        ];
        let positions: HashMap<u32, Vec3> = atoms.iter().map(|a| (a.id, a.position)).collect();
        let backbone = build_protein_backbone(&atoms, &positions);
        assert_eq!(backbone.ca_count, 3);
        assert!(!backbone.cartoon_available);
    }

    #[test]
    fn test_cartoon_available_threshold() {
        let mut atoms = Vec::new();
        let mut positions = HashMap::new();
        for i in 0..MIN_CARTOON_RESIDUES {
            let id = i as u32;
            let pos = Vec3::new(i as f32 * 3.8, 0.0, 0.0);
            atoms.push(ca(id, i as u32 + 1, pos));
            positions.insert(id, pos);
        }
        let backbone = build_protein_backbone(&atoms, &positions);
        assert!(backbone.cartoon_available);
    }
}
