//! Molecule component and data structures

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Molecule component attached to Bevy entities
#[derive(Component, Clone, Debug, Reflect, Default)]
#[reflect(Component)]
pub struct Molecule {
    /// Molecule name or identifier
    pub name: String,
    /// Atoms belonging to this molecule
    pub atoms: Vec<Entity>,
    /// Molecule type (protein, ligand, water, etc.)
    pub molecule_type: MoleculeType,
    /// Chain identifier (if applicable)
    pub chain_id: String,
}

/// Static molecule data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoleculeData {
    /// Molecule name or identifier
    pub name: String,
    /// Atom IDs belonging to this molecule
    pub atom_ids: Vec<u32>,
    /// Molecule type
    pub molecule_type: MoleculeType,
    /// Chain identifier (if applicable)
    pub chain_id: String,
    /// Sequence (for proteins/polymers)
    pub sequence: String,
}

impl MoleculeData {
    /// Create a new molecule data structure
    pub fn new(name: String, molecule_type: MoleculeType, chain_id: String) -> Self {
        Self {
            name,
            atom_ids: Vec::new(),
            molecule_type,
            chain_id,
            sequence: String::new(),
        }
    }

    /// Add an atom to this molecule
    pub fn add_atom(&mut self, atom_id: u32) {
        self.atom_ids.push(atom_id);
    }
}

/// Molecule type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
#[reflect(Debug, PartialEq, Hash)]
pub enum MoleculeType {
    /// Protein
    Protein,
    /// Nucleic acid (DNA/RNA)
    NucleicAcid,
    /// Ligand
    Ligand,
    /// Water molecule
    Water,
    /// Ion
    Ion,
    /// Lipid
    Lipid,
    /// Carbohydrate
    Carbohydrate,
    /// Small molecule
    SmallMolecule,
    /// Polymer
    Polymer,
    /// Unknown molecule type
    Unknown,
}

impl Default for MoleculeType {
    fn default() -> Self {
        MoleculeType::Unknown
    }
}

impl MoleculeType {
    /// Determine molecule type from name
    pub fn from_name(name: &str) -> Self {
        let name_upper = name.to_uppercase();

        if name_upper.contains("HOH") || name_upper.contains("WAT") || name_upper.contains("TIP3") {
            MoleculeType::Water
        } else if name_upper.contains("NA") || name_upper.contains("CL") || name_upper.contains("K") {
            MoleculeType::Ion
        } else if name_upper.contains("DNA") || name_upper.contains("RNA") {
            MoleculeType::NucleicAcid
        } else if name_upper.contains("ALA") || name_upper.contains("GLY") || name_upper.contains("VAL") {
            MoleculeType::Protein
        } else if name_upper.contains("LIG") || name_upper.contains("INH") || name_upper.contains("ACT") {
            MoleculeType::Ligand
        } else {
            MoleculeType::Unknown
        }
    }

    /// Check if this is a protein-like molecule
    pub fn is_protein(&self) -> bool {
        matches!(self, MoleculeType::Protein | MoleculeType::SmallMolecule)
    }

    /// Check if this is a nucleic acid
    pub fn is_nucleic_acid(&self) -> bool {
        matches!(self, MoleculeType::NucleicAcid)
    }

    /// Check if this is a solvent molecule
    pub fn is_solvent(&self) -> bool {
        matches!(self, MoleculeType::Water | MoleculeType::Ion)
    }
}

/// Secondary structure classification for proteins
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
#[reflect(Debug, PartialEq, Hash)]
pub enum SecondaryStructure {
    /// Alpha helix
    AlphaHelix,
    /// 3-10 helix
    ThreeTenHelix,
    /// Pi helix
    PiHelix,
    /// Beta strand
    BetaStrand,
    /// Beta sheet
    BetaSheet,
    /// Turn
    Turn,
    /// Coil (no regular structure)
    Coil,
    /// Unknown structure
    Unknown,
}

impl Default for SecondaryStructure {
    fn default() -> Self {
        SecondaryStructure::Unknown
    }
}

/// Amino acid types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AminoAcid {
    Alanine,    // ALA
    Arginine,   // ARG
    Asparagine, // ASN
    AsparticAcid, // ASP
    Cysteine,   // CYS
    GlutamicAcid, // GLU
    Glutamine,  // GLN
    Glycine,    // GLY
    Histidine,  // HIS
    Isoleucine, // ILE
    Leucine,    // LEU
    Lysine,     // LYS
    Methionine, // MET
    Phenylalanine, // PHE
    Proline,    // PRO
    Serine,     // SER
    Threonine,  // THR
    Tryptophan, // TRP
    Tyrosine,   // TYR
    Valine,     // VAL
    Unknown,
}

impl AminoAcid {
    /// Get the 3-letter code for this amino acid
    pub fn code3(&self) -> &'static str {
        match self {
            AminoAcid::Alanine => "ALA",
            AminoAcid::Arginine => "ARG",
            AminoAcid::Asparagine => "ASN",
            AminoAcid::AsparticAcid => "ASP",
            AminoAcid::Cysteine => "CYS",
            AminoAcid::GlutamicAcid => "GLU",
            AminoAcid::Glutamine => "GLN",
            AminoAcid::Glycine => "GLY",
            AminoAcid::Histidine => "HIS",
            AminoAcid::Isoleucine => "ILE",
            AminoAcid::Leucine => "LEU",
            AminoAcid::Lysine => "LYS",
            AminoAcid::Methionine => "MET",
            AminoAcid::Phenylalanine => "PHE",
            AminoAcid::Proline => "PRO",
            AminoAcid::Serine => "SER",
            AminoAcid::Threonine => "THR",
            AminoAcid::Tryptophan => "TRP",
            AminoAcid::Tyrosine => "TYR",
            AminoAcid::Valine => "VAL",
            AminoAcid::Unknown => "UNK",
        }
    }

    /// Get the 1-letter code for this amino acid
    pub fn code1(&self) -> char {
        match self {
            AminoAcid::Alanine => 'A',
            AminoAcid::Arginine => 'R',
            AminoAcid::Asparagine => 'N',
            AminoAcid::AsparticAcid => 'D',
            AminoAcid::Cysteine => 'C',
            AminoAcid::GlutamicAcid => 'E',
            AminoAcid::Glutamine => 'Q',
            AminoAcid::Glycine => 'G',
            AminoAcid::Histidine => 'H',
            AminoAcid::Isoleucine => 'I',
            AminoAcid::Leucine => 'L',
            AminoAcid::Lysine => 'K',
            AminoAcid::Methionine => 'M',
            AminoAcid::Phenylalanine => 'F',
            AminoAcid::Proline => 'P',
            AminoAcid::Serine => 'S',
            AminoAcid::Threonine => 'T',
            AminoAcid::Tryptophan => 'W',
            AminoAcid::Tyrosine => 'Y',
            AminoAcid::Valine => 'V',
            AminoAcid::Unknown => 'X',
        }
    }

    /// Parse an amino acid from its 3-letter code
    pub fn from_code3(code: &str) -> Self {
        match code.to_uppercase().as_str() {
            "ALA" => AminoAcid::Alanine,
            "ARG" => AminoAcid::Arginine,
            "ASN" => AminoAcid::Asparagine,
            "ASP" => AminoAcid::AsparticAcid,
            "CYS" => AminoAcid::Cysteine,
            "GLU" => AminoAcid::GlutamicAcid,
            "GLN" => AminoAcid::Glutamine,
            "GLY" => AminoAcid::Glycine,
            "HIS" => AminoAcid::Histidine,
            "ILE" => AminoAcid::Isoleucine,
            "LEU" => AminoAcid::Leucine,
            "LYS" => AminoAcid::Lysine,
            "MET" => AminoAcid::Methionine,
            "PHE" => AminoAcid::Phenylalanine,
            "PRO" => AminoAcid::Proline,
            "SER" => AminoAcid::Serine,
            "THR" => AminoAcid::Threonine,
            "TRP" => AminoAcid::Tryptophan,
            "TYR" => AminoAcid::Tyrosine,
            "VAL" => AminoAcid::Valine,
            _ => AminoAcid::Unknown,
        }
    }

    /// Check if this is a hydrophobic amino acid
    pub fn is_hydrophobic(&self) -> bool {
        matches!(
            self,
            AminoAcid::Alanine
                | AminoAcid::Valine
                | AminoAcid::Leucine
                | AminoAcid::Isoleucine
                | AminoAcid::Methionine
                | AminoAcid::Phenylalanine
                | AminoAcid::Tryptophan
                | AminoAcid::Proline
        )
    }

    /// Check if this is a polar amino acid
    pub fn is_polar(&self) -> bool {
        matches!(
            self,
            AminoAcid::Asparagine
                | AminoAcid::Glutamine
                | AminoAcid::Serine
                | AminoAcid::Threonine
                | AminoAcid::Tyrosine
                | AminoAcid::Cysteine
        )
    }

    /// Check if this is a charged amino acid
    pub fn is_charged(&self) -> bool {
        matches!(
            self,
            AminoAcid::Arginine
                | AminoAcid::Lysine
                | AminoAcid::AsparticAcid
                | AminoAcid::GlutamicAcid
                | AminoAcid::Histidine
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_molecule_type_from_name() {
        assert_eq!(MoleculeType::from_name("HOH"), MoleculeType::Water);
        assert_eq!(MoleculeType::from_name("WAT"), MoleculeType::Water);
        assert_eq!(MoleculeType::from_name("NA"), MoleculeType::Ion);
        assert_eq!(MoleculeType::from_name("ALA"), MoleculeType::Protein);
        assert_eq!(MoleculeType::from_name("LIG"), MoleculeType::Ligand);
    }

    #[test]
    fn test_amino_acid_codes() {
        assert_eq!(AminoAcid::Alanine.code3(), "ALA");
        assert_eq!(AminoAcid::Alanine.code1(), 'A');
        assert_eq!(AminoAcid::from_code3("GLY"), AminoAcid::Glycine);
    }

    #[test]
    fn test_amino_acid_classification() {
        assert!(AminoAcid::Alanine.is_hydrophobic());
        assert!(AminoAcid::Serine.is_polar());
        assert!(AminoAcid::Lysine.is_charged());
    }
}
