//! DSSP secondary structure assignment (Kabsch-Sander / DSSP 4-like).
//!
//! Wraps [`pdbrust`] hydrogen-bond analysis to assign per-residue secondary
//! structure from backbone N, CA, C, and O coordinates.

use crate::core::atom::AtomData;
use crate::core::molecule::SecondaryStructure;
use bevy::prelude::*;
use pdbrust::records::Atom as PdbAtom;
use pdbrust::PdbStructure;
use std::collections::HashMap;

/// Result of a DSSP assignment run.
#[derive(Debug, Clone)]
pub struct DsspResult {
    /// Per-residue assignments keyed by `(chain_id, residue_id)`.
    pub assignments: HashMap<(String, u32), SecondaryStructure>,
    /// Whether full DSSP ran or a fallback was used upstream.
    pub used_dssp: bool,
    /// Non-fatal warnings from the assignment engine.
    pub warnings: Vec<String>,
}

impl DsspResult {
    fn empty_with_warnings(warnings: Vec<String>) -> Self {
        Self {
            assignments: HashMap::new(),
            used_dssp: false,
            warnings,
        }
    }
}

/// Assign secondary structure using the DSSP algorithm.
///
/// Requires backbone atoms (N, CA, C, O) for standard amino-acid residues.
/// Returns an empty assignment map when the input lacks sufficient backbone
/// coordinates (callers should fall back to a heuristic in that case).
pub fn assign_dssp(atom_data: &[AtomData], positions: &HashMap<u32, Vec3>) -> DsspResult {
    let pdb_atoms = atoms_for_dssp(atom_data, positions);
    if pdb_atoms.is_empty() {
        return DsspResult::empty_with_warnings(
            vec!["No backbone atoms available for DSSP".into()],
        );
    }

    let mut structure = PdbStructure::new();
    structure.atoms = pdb_atoms;

    let assignment = structure.assign_secondary_structure();
    let warnings = assignment.warnings.clone();

    if assignment.residue_assignments.is_empty() {
        return DsspResult::empty_with_warnings(warnings);
    }

    let ca_only = warnings.iter().any(|w| w.contains("CA-only"));
    if ca_only {
        return DsspResult::empty_with_warnings(warnings);
    }

    let mut assignments = HashMap::with_capacity(assignment.residue_assignments.len());
    for res in &assignment.residue_assignments {
        if res.residue_seq < 0 {
            continue;
        }
        let key = (res.chain_id.clone(), res.residue_seq as u32);
        assignments.insert(key, map_dssp_ss(res.ss));
    }

    DsspResult {
        assignments,
        used_dssp: true,
        warnings,
    }
}

/// Map pdbrust DSSP codes to engine [`SecondaryStructure`].
pub fn map_dssp_ss(ss: pdbrust::dssp::SecondaryStructure) -> SecondaryStructure {
    use pdbrust::dssp::SecondaryStructure as DsspSs;
    match ss {
        DsspSs::AlphaHelix => SecondaryStructure::AlphaHelix,
        DsspSs::Helix310 => SecondaryStructure::ThreeTenHelix,
        DsspSs::PiHelix => SecondaryStructure::PiHelix,
        DsspSs::ExtendedStrand => SecondaryStructure::BetaStrand,
        DsspSs::BetaBridge => SecondaryStructure::BetaStrand,
        DsspSs::Turn => SecondaryStructure::Turn,
        DsspSs::KappaHelix | DsspSs::Bend | DsspSs::Coil => SecondaryStructure::Coil,
    }
}

fn atoms_for_dssp(atom_data: &[AtomData], positions: &HashMap<u32, Vec3>) -> Vec<PdbAtom> {
    atom_data
        .iter()
        .filter(|atom| is_standard_amino_acid(&atom.residue_name))
        .filter(|atom| is_backbone_atom_name(&atom.name))
        .filter_map(|atom| atom_data_to_pdb_atom(atom, positions))
        .collect()
}

fn atom_data_to_pdb_atom(atom: &AtomData, positions: &HashMap<u32, Vec3>) -> Option<PdbAtom> {
    let pos = positions.get(&atom.id).copied().unwrap_or(atom.position);
    if !pos.is_finite() {
        return None;
    }

    Some(PdbAtom::new(
        atom.id as i32,
        normalize_atom_name(&atom.name),
        None,
        atom.residue_name.clone(),
        atom.chain_id.clone(),
        atom.residue_id as i32,
        f64::from(pos.x),
        f64::from(pos.y),
        f64::from(pos.z),
        f64::from(atom.occupancy),
        f64::from(atom.b_factor),
        atom.element.symbol().to_string(),
        None,
    ))
}

fn normalize_atom_name(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return "CA".to_string();
    }
    trimmed.to_string()
}

fn is_backbone_atom_name(name: &str) -> bool {
    matches!(
        name.trim(),
        "N" | "CA" | "C" | "O" | "OXT" | "H" | "HN" | "H1" | "H2" | "H3"
    )
}

fn is_standard_amino_acid(residue_name: &str) -> bool {
    matches!(
        residue_name.trim().to_ascii_uppercase().as_str(),
        "ALA"
            | "ARG"
            | "ASN"
            | "ASP"
            | "CYS"
            | "GLN"
            | "GLU"
            | "GLY"
            | "HIS"
            | "ILE"
            | "LEU"
            | "LYS"
            | "MET"
            | "PHE"
            | "PRO"
            | "SER"
            | "THR"
            | "TRP"
            | "TYR"
            | "VAL"
            | "SEC"
            | "PYL"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::atom::{AtomData, Element};
    use pdbrust::parse_pdb_file;

    fn load_1crn_atoms() -> (Vec<AtomData>, HashMap<u32, Vec3>) {
        let structure = parse_pdb_file("tests/fixtures/1CRN.pdb").expect("parse 1CRN");
        let mut atom_data = Vec::new();
        let mut positions = HashMap::new();

        for atom in &structure.atoms {
            if atom.is_hetatm {
                continue;
            }
            let id = atom.serial as u32;
            let element = Element::from_symbol(atom.element.trim()).unwrap_or(Element::C);
            let mut data = AtomData::new(
                id,
                element,
                atom.residue_seq as u32,
                atom.residue_name.clone(),
                atom.chain_id.clone(),
                atom.name.clone(),
            );
            data.position = Vec3::new(atom.x as f32, atom.y as f32, atom.z as f32);
            data.b_factor = atom.temp_factor as f32;
            data.occupancy = atom.occupancy as f32;
            positions.insert(id, data.position);
            atom_data.push(data);
        }

        (atom_data, positions)
    }

    #[test]
    fn test_dssp_1crn_matches_pdbrust() {
        let (atom_data, positions) = load_1crn_atoms();
        let result = assign_dssp(&atom_data, &positions);

        assert!(result.used_dssp, "expected DSSP: {:?}", result.warnings);
        assert_eq!(result.assignments.len(), 46);

        let structure = parse_pdb_file("tests/fixtures/1CRN.pdb").expect("parse 1CRN");
        let direct = structure.assign_secondary_structure();

        for res in &direct.residue_assignments {
            let key = (res.chain_id.clone(), res.residue_seq as u32);
            let expected = map_dssp_ss(res.ss);
            assert_eq!(
                result.assignments.get(&key),
                Some(&expected),
                "residue {}:{}",
                res.chain_id,
                res.residue_seq
            );
        }

        assert!(
            direct.helix_fraction > 0.4,
            "crambin should be mostly helical"
        );
    }

    #[test]
    fn test_dssp_ca_only_falls_back() {
        let mut atoms = Vec::new();
        let mut positions = HashMap::new();
        for i in 0..30 {
            let id = i as u32;
            let pos = Vec3::new(i as f32 * 3.8, 0.0, 0.0);
            let atom = AtomData::new(id, Element::C, i + 1, "ALA".into(), "A".into(), "CA".into());
            positions.insert(id, pos);
            atoms.push(atom);
        }

        let result = assign_dssp(&atoms, &positions);
        assert!(!result.used_dssp);
        assert!(result.assignments.is_empty());
    }

    #[test]
    fn test_map_dssp_codes() {
        use pdbrust::dssp::SecondaryStructure as DsspSs;
        assert_eq!(
            map_dssp_ss(DsspSs::AlphaHelix),
            SecondaryStructure::AlphaHelix
        );
        assert_eq!(
            map_dssp_ss(DsspSs::ExtendedStrand),
            SecondaryStructure::BetaStrand
        );
        assert_eq!(map_dssp_ss(DsspSs::Turn), SecondaryStructure::Turn);
    }
}
