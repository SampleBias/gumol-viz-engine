//! PDB file format parser
//!
//! The Protein Data Bank (PDB) format is a standard for representing
//! 3D structures of biological macromolecules.

use crate::core::atom::{AtomData, Element};
use crate::core::bond::{BondData, BondType};
use crate::core::trajectory::{FrameData, Trajectory, TrajectoryMetadata};
use crate::io::{IOError, IOResult};
use bevy::prelude::*;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};

/// PDB format parser
pub struct PDBParser;

impl PDBParser {
    /// Parse a PDB file and return trajectory data
    pub fn parse_file(path: &Path) -> IOResult<Trajectory> {
        Ok(Self::parse_file_with_atoms(path)?.0)
    }

    /// Parse a PDB file returning trajectory, atom metadata, and CONECT bonds
    pub fn parse_file_with_atoms(
        path: &Path,
    ) -> IOResult<(Trajectory, Vec<AtomData>, Vec<BondData>)> {
        let file =
            File::open(path).map_err(|_e| IOError::FileNotFound(path.display().to_string()))?;
        let reader = BufReader::new(file);
        Self::parse_reader(reader, path.to_path_buf())
    }

    /// Parse PDB format from a reader
    pub fn parse_reader<R: Read>(
        reader: R,
        file_path: PathBuf,
    ) -> IOResult<(Trajectory, Vec<AtomData>, Vec<BondData>)> {
        let reader = BufReader::new(reader);
        let lines: Vec<String> = reader.lines().collect::<Result<_, _>>()?;
        Self::parse_lines(&lines, file_path)
    }

    /// Parse PDB format from string content
    pub fn parse_string(
        content: &str,
        file_path: PathBuf,
    ) -> IOResult<(Trajectory, Vec<AtomData>, Vec<BondData>)> {
        let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        Self::parse_lines(&lines, file_path)
    }

    fn parse_lines(
        lines: &[String],
        file_path: PathBuf,
    ) -> IOResult<(Trajectory, Vec<AtomData>, Vec<BondData>)> {
        let mut atom_data = Vec::new();
        let mut bond_data = Vec::new();
        let mut frames = Vec::new();
        let mut current_frame = FrameData::new(0, 0.0);
        let mut frame_index = 0;
        // Atom metadata is only collected from the first model to avoid duplicates.
        let mut in_first_model = true;

        let mut metadata = TrajectoryMetadata::default();

        for (line_num, line) in lines.iter().enumerate() {
            if line.len() < 6 {
                continue;
            }

            let record_name = line[0..6].trim();

            match record_name {
                "HEADER" => Self::parse_header(line, &mut metadata),
                "TITLE" => Self::parse_title(line, &mut metadata),
                "CRYST1" => Self::parse_cryst1(line, &mut current_frame),
                "ATOM" | "HETATM" => {
                    if let Some(atom) = Self::parse_atom(line, line_num)? {
                        current_frame.set_position(atom.id, atom.position);
                        if in_first_model {
                            atom_data.push(atom);
                        }
                    }
                }
                "CONECT" => {
                    if let Some(bonds) = Self::parse_conect(line, line_num)? {
                        bond_data.extend(bonds);
                    }
                }
                "MODEL" => {
                    // Start of new frame
                    if frame_index > 0 && !current_frame.positions.is_empty() {
                        in_first_model = false;
                        frames.push(current_frame);
                    }
                    current_frame = FrameData::new(frame_index, frame_index as f32);
                    frame_index += 1;
                }
                "ENDMDL" => {
                    // End of frame
                    frames.push(current_frame.clone());
                    in_first_model = false;
                    current_frame = FrameData::new(frame_index, frame_index as f32);
                }
                "END" | "TER" => {
                    // End of record/terminator - do nothing
                }
                _ => {
                    // Other record types (REMARK, SEQRES, etc.) can be ignored for now
                }
            }
        }

        // Add the last frame if it has data
        if !current_frame.positions.is_empty()
            && (frames.is_empty() || current_frame.index != frames.last().unwrap().index)
        {
            frames.push(current_frame);
        }

        // If no frames were found, create one from the ATOM records
        if frames.is_empty() && !atom_data.is_empty() {
            let mut frame = FrameData::new(0, 0.0);
            for atom in atom_data.iter() {
                frame.set_position(atom.id, atom.position);
            }
            frames.push(frame);
        }

        // Create trajectory
        let num_atoms = atom_data.len();
        let mut trajectory = Trajectory::new(file_path, num_atoms, 1.0);
        trajectory.metadata = metadata;
        for frame in frames {
            trajectory.add_frame(frame);
        }

        Ok((trajectory, atom_data, bond_data))
    }

    /// Parse HEADER record
    fn parse_header(line: &str, metadata: &mut TrajectoryMetadata) {
        if line.len() > 50 {
            metadata.classification = line[10..50].trim().to_string();
        }
    }

    /// Parse TITLE record
    fn parse_title(line: &str, metadata: &mut TrajectoryMetadata) {
        if line.len() > 10 {
            metadata.title.push_str(line[10..].trim());
            metadata.title.push(' ');
        }
    }

    /// Parse CRYST1 record (unit cell dimensions)
    fn parse_cryst1(line: &str, frame: &mut FrameData) {
        if line.len() >= 54 {
            let a = line[6..15].trim().parse::<f32>().ok();
            let b = line[15..24].trim().parse::<f32>().ok();
            let c = line[24..33].trim().parse::<f32>().ok();

            if let (Some(a), Some(b), Some(c)) = (a, b, c) {
                frame.box_size = Some([a, b, c]);
            }
        }
    }

    /// Parse ATOM or HETATM record
    fn parse_atom(line: &str, line_num: usize) -> IOResult<Option<AtomData>> {
        if line.len() < 54 {
            return Err(IOError::ParseError {
                line: line_num,
                message: "ATOM record too short".to_string(),
            });
        }

        let serial = line[6..11]
            .trim()
            .parse::<u32>()
            .map_err(|_| IOError::ParseError {
                line: line_num,
                message: format!("Invalid serial number: {}", &line[6..11]),
            })?;

        let name = line[12..16].trim().to_string();
        let residue_name = line[17..20].trim().to_string();
        let chain_id = if line.len() > 21 {
            line[21..22].trim().to_string()
        } else {
            String::new()
        };
        let residue_seq = line[22..26]
            .trim()
            .parse::<u32>()
            .map_err(|_| IOError::ParseError {
                line: line_num,
                message: format!("Invalid residue sequence: {}", &line[22..26]),
            })?;

        let x = line[30..38]
            .trim()
            .parse::<f32>()
            .map_err(|_| IOError::ParseError {
                line: line_num,
                message: format!("Invalid X coordinate: {}", &line[30..38]),
            })?;
        let y = line[38..46]
            .trim()
            .parse::<f32>()
            .map_err(|_| IOError::ParseError {
                line: line_num,
                message: format!("Invalid Y coordinate: {}", &line[38..46]),
            })?;
        let z = line[46..54]
            .trim()
            .parse::<f32>()
            .map_err(|_| IOError::ParseError {
                line: line_num,
                message: format!("Invalid Z coordinate: {}", &line[46..54]),
            })?;

        // Parse occupancy and temperature factor
        let occupancy = if line.len() >= 60 {
            line[54..60].trim().parse::<f32>().ok()
        } else {
            None
        };

        let temp_factor = if line.len() >= 66 {
            line[60..66].trim().parse::<f32>().ok()
        } else {
            None
        };

        // Element: prefer PDB columns 77–78, then protein-aware atom name inference.
        let element = Self::parse_element(line, &name);

        Ok(Some(AtomData {
            id: serial,
            element,
            residue_id: residue_seq,
            residue_name,
            chain_id,
            name,
            charge: 0.0,
            mass: element.mass(),
            position: Vec3::new(x, y, z),
            occupancy: occupancy.unwrap_or(1.0),
            b_factor: temp_factor.unwrap_or(0.0),
        }))
    }

    /// Parse element from PDB record (columns 77–78) with atom-name fallback.
    fn parse_element(line: &str, atom_name: &str) -> Element {
        if line.len() >= 78 {
            let symbol = line[76..78].trim();
            if !symbol.is_empty() {
                if let Ok(element) = Element::from_symbol(symbol) {
                    return element;
                }
            }
        }
        Element::from_atom_name(atom_name)
    }

    /// Extract static atom metadata and bonds from a topology file (single frame).
    pub fn parse_topology(path: &Path) -> IOResult<(Vec<AtomData>, Vec<BondData>)> {
        let (_, atom_data, bond_data) = Self::parse_file_with_atoms(path)?;
        Ok(crate::io::topology::normalize_topology(
            atom_data, bond_data,
        ))
    }

    /// Parse CONECT record (bonds)
    fn parse_conect(line: &str, line_num: usize) -> IOResult<Option<Vec<BondData>>> {
        let parts: Vec<&str> = line.split_whitespace().collect();

        if parts.len() < 3 {
            return Ok(None);
        }

        let atom_a = parts[1].parse::<u32>().map_err(|_| IOError::ParseError {
            line: line_num,
            message: format!("Invalid atom ID: {}", parts[1]),
        })?;

        let mut bonds = Vec::new();

        // CONECT can have multiple bonded atoms
        for part in parts.iter().skip(2) {
            if let Ok(atom_b) = part.parse::<u32>() {
                bonds.push(BondData::new(
                    atom_a,
                    atom_b,
                    BondType::Covalent,
                    crate::core::bond::BondOrder::Single,
                    crate::core::bond::BondLengths::get_length(
                        crate::core::atom::Element::C, // Placeholder
                        crate::core::atom::Element::C, // Placeholder
                    ),
                ));
            }
        }

        Ok(Some(bonds))
    }
}

/// Write trajectory to PDB format
pub struct PDBWriter;

impl PDBWriter {
    /// Write a single frame to PDB format
    pub fn write_frame<W: std::io::Write>(
        writer: &mut W,
        frame: &FrameData,
        atom_data: &[AtomData],
    ) -> IOResult<()> {
        for atom in atom_data {
            if let Some(pos) = frame.get_position(atom.id) {
                // Write ATOM or HETATM record
                let record_type = if Self::is_standard_residue(&atom.residue_name) {
                    "ATOM  "
                } else {
                    "HETATM"
                };

                writeln!(
                    writer,
                    "{:<6}{:>5} {:<4}{:1}{:>3} {:1}{:>4}    {:>8.3}{:>8.3}{:>8.3}{:>6.2}{:>6.2}          {:>2}",
                    record_type,
                    atom.id,
                    atom.name,
                    "", // altLoc
                    atom.residue_name,
                    atom.chain_id,
                    atom.residue_id,
                    pos.x, pos.y, pos.z,
                    atom.occupancy,
                    atom.b_factor,
                    atom.element.symbol()
                )?;
            }
        }

        writeln!(writer, "END")?;
        Ok(())
    }

    /// Check if a residue is a standard amino acid
    fn is_standard_residue(name: &str) -> bool {
        matches!(
            name.to_uppercase().as_str(),
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
        )
    }
}

/// Register PDB parsing systems with Bevy
pub fn register(_app: &mut App) {
    info!("PDB parser registered");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_pdb() {
        let pdb_content = r#"HEADER    EXAMPLE STRUCTURE                       01-JAN-24   XXXX              
TITLE     Example structure
CRYST1   10.000   10.000   10.000  90.00  90.00  90.00 P 1           1          
ATOM      1  N   ALA A   1       0.000   0.000   0.000  1.00 20.00           N  
ATOM      2  CA  ALA A   1       1.000   0.000   0.000  1.00 20.00           C  
ATOM      3  C   ALA A   1       2.000   0.000   0.000  1.00 20.00           C  
ATOM      4  O   ALA A   1       2.500   1.000   0.000  1.00 20.00           O  
END
"#;

        let result = PDBParser::parse_string(pdb_content, PathBuf::from("test.pdb"));

        assert!(result.is_ok());
        let (trajectory, atom_data, _bonds) = result.unwrap();
        assert_eq!(trajectory.num_frames(), 1);
        assert_eq!(trajectory.num_atoms, 4);
        assert_eq!(atom_data.len(), 4);

        // Elements must be parsed correctly (not defaulted to carbon).
        assert_eq!(atom_data[0].element, Element::N);
        assert_eq!(atom_data[3].element, Element::O);
        assert_eq!(atom_data[0].residue_name, "ALA");

        // Frame positions are keyed by serial number and match atom data.
        let frame = trajectory.get_frame(0).unwrap();
        for atom in &atom_data {
            assert_eq!(frame.get_position(atom.id), Some(atom.position));
        }
    }

    #[test]
    fn test_parse_multi_model_pdb() {
        let pdb_content = r#"MODEL        1
ATOM      1  N   ALA A   1       0.000   0.000   0.000  1.00 20.00           N  
ATOM      2  CA  ALA A   1       1.000   0.000   0.000  1.00 20.00           C  
ENDMDL
MODEL        2
ATOM      1  N   ALA A   1       0.100   0.000   0.000  1.00 20.00           N  
ATOM      2  CA  ALA A   1       1.100   0.000   0.000  1.00 20.00           C  
ENDMDL
END
"#;

        let result = PDBParser::parse_string(pdb_content, PathBuf::from("test.pdb"));

        assert!(result.is_ok());
        let (trajectory, atom_data, _bonds) = result.unwrap();
        assert_eq!(trajectory.num_frames(), 2);
        // Atom metadata comes from the first model only (no duplicates).
        assert_eq!(atom_data.len(), 2);

        // Both frames carry positions keyed by serial number.
        assert_eq!(
            trajectory.get_frame(0).unwrap().get_position(1),
            Some(Vec3::new(0.0, 0.0, 0.0))
        );
        assert_eq!(
            trajectory.get_frame(1).unwrap().get_position(1),
            Some(Vec3::new(0.1, 0.0, 0.0))
        );
    }
}
