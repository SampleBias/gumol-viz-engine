//! mmCIF file format parser
//!
//! The mmCIF (macromolecular Crystallographic Information File) format is an
//! alternative to PDB format that can store larger structures with more metadata.
//! It uses a hierarchical key-value structure rather than fixed-width columns.

use crate::core::atom::{Atom, AtomData, Element};
use crate::core::trajectory::{FrameData, Trajectory, TrajectoryMetadata};
use crate::io::{FileFormat, IOError, IOResult};
use bevy::prelude::*;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};

/// mmCIF format parser
pub struct MmcifParser;

impl MmcifParser {
    /// Parse an mmCIF file and return trajectory data
    pub fn parse_file(path: &Path) -> IOResult<Trajectory> {
        let file = File::open(path).map_err(|e| IOError::FileNotFound(path.display().to_string()))?;
        let reader = BufReader::new(file);
        Self::parse_reader(reader, path.to_path_buf())
    }

    /// Parse mmCIF format from a reader
    pub fn parse_reader<R: Read>(reader: R, file_path: PathBuf) -> IOResult<Trajectory> {
        let reader = BufReader::new(reader);
        let lines: Vec<String> = reader.lines().collect::<Result<_, _>>()?;

        Self::parse_lines(&lines, file_path)
    }

    /// Parse mmCIF format from string content
    pub fn parse_string(content: &str, file_path: PathBuf) -> IOResult<Trajectory> {
        let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        Self::parse_lines(&lines, file_path)
    }

    /// Parse mmCIF format from a vector of lines
    fn parse_lines(lines: &[String], file_path: PathBuf) -> IOResult<Trajectory> {
        if lines.is_empty() {
            return Err(IOError::ParseError {
                line: 0,
                message: "Empty mmCIF file".to_string(),
            });
        }

        // Parse data into categories and columns
        let mut data = MmcifData::default();

        let mut line_iter = lines.iter().enumerate();
        let mut current_category = String::new();
        let mut current_columns: Vec<String> = Vec::new();
        let mut in_loop = false;

        while let Some((line_num, line)) = line_iter.next() {
            let line = line.trim();

            // Skip comments and empty lines
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Data block start
            if line.starts_with("data_") {
                data.id = line[5..].to_string();
                continue;
            }

            // Loop start (multi-record data)
            if line.starts_with("loop_") {
                in_loop = true;
                current_category.clear();
                current_columns.clear();
                continue;
            }

            // Category/column definition
            if in_loop {
                if line.starts_with('_') {
                    // This is a column definition
                    // Format: _category.column
                    if let Some(dot_pos) = line.find('.') {
                        let category = &line[1..dot_pos];
                        let column = &line[dot_pos + 1..];

                        if current_category.is_empty() {
                            current_category = category.to_string();
                        } else if current_category != category {
                            // New category, start a new block
                            if !current_columns.is_empty() {
                                data.categories.insert(
                                    current_category.clone(),
                                    current_columns.clone(),
                                );
                            }
                            current_category = category.to_string();
                            current_columns.clear();
                        }

                        current_columns.push(column.to_string());
                    }
                    continue;
                }

                // Check if this is a data line (not another _ or loop_)
                if !line.starts_with('_') && !line.starts_with("loop_") && !line.starts_with("data_") {
                    // This is data for the current loop
                    let values: Vec<String> = line
                        .split_whitespace()
                        .map(|s| s.to_string())
                        .collect();

                    if current_category != "atom_site" {
                        // We only care about atom_site for now
                        continue;
                    }

                    // Store in atom_site data
                    let mut record: HashMap<String, String> = HashMap::new();
                    for (i, value) in values.iter().enumerate() {
                        if i < current_columns.len() {
                            record.insert(current_columns[i].clone(), value.clone());
                        }
                    }

                    data.atom_site.push(record);
                    continue;
                }

                // End of loop
                in_loop = false;
                if !current_columns.is_empty() && !current_category.is_empty() {
                    data.categories.insert(current_category.clone(), current_columns.clone());
                }
                continue;
            }

            // Single-value record (key-value pairs outside loops)
            if line.starts_with('_') {
                // Format: _category.column  value
                let parts: Vec<&str> = line.splitn(2, char::is_whitespace).collect();
                if parts.len() >= 2 {
                    let key = parts[0][1..].to_string(); // Remove leading _
                    let value = parts[1].to_string();
                    data.metadata.insert(key, value);
                }
                continue;
            }
        }

        // End of file - save any remaining loop data
        if !current_columns.is_empty() && !current_category.is_empty() {
            data.categories.insert(current_category.clone(), current_columns);
        }

        // Extract metadata
        let mut metadata = TrajectoryMetadata::default();
        if let Some(title) = data.metadata.get("struct.title") {
            metadata.title = title.clone();
        }
        if let Some(classification) = data.metadata.get("struct_keywords.pdbx_keywords") {
            metadata.classification = classification.clone();
        }

        // Check if we have atom data
        if data.atom_site.is_empty() {
            // Return empty trajectory if no atoms
            return Ok(Trajectory::new(file_path, 0, 1.0));
        }

        // Count atoms and create frame
        let num_atoms = data.atom_site.len();
        let mut frame = FrameData::new(0, 0.0);

        // Parse atoms
        for (atom_id, record) in data.atom_site.iter().enumerate() {
            // Get position
            let x = record
                .get("Cartn_x")
                .and_then(|s| s.parse::<f32>().ok())
                .unwrap_or(0.0);
            let y = record
                .get("Cartn_y")
                .and_then(|s| s.parse::<f32>().ok())
                .unwrap_or(0.0);
            let z = record
                .get("Cartn_z")
                .and_then(|s| s.parse::<f32>().ok())
                .unwrap_or(0.0);

            let position = Vec3::new(x, y, z);
            frame.set_position(atom_id as u32, position);

            // Note: Atom data would be stored separately in a full implementation
            // For now, we're just storing positions in the trajectory
        }

        // Create trajectory
        let mut trajectory = Trajectory::new(file_path, num_atoms, 1.0);
        trajectory.metadata = metadata;
        trajectory.add_frame(frame);

        Ok(trajectory)
    }

    /// Parse atom data from mmCIF record
    fn parse_atom_data(record: &HashMap<String, String>, atom_id: u32) -> Option<AtomData> {
        // Get atom name
        let atom_name = record.get("label_atom_id")?.clone();

        // Get residue name
        let residue_name = record
            .get("label_comp_id")
            .unwrap_or(&"UNK".to_string())
            .clone();

        // Get residue number
        let residue_id = record
            .get("label_seq_id")
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(0);

        // Get chain ID
        let chain_id = record
            .get("auth_asym_id")
            .unwrap_or(&"A".to_string())
            .clone();

        // Determine element from atom name
        let element = Self::element_from_atom_name(&atom_name);

        Some(AtomData::new(
            atom_id,
            element,
            residue_id as u32,
            residue_name,
            chain_id,
            atom_name,
        ))
    }

    /// Determine element from atom name (similar to PDB)
    fn element_from_atom_name(atom_name: &str) -> Element {
        // Remove leading numbers and non-letters
        let name = atom_name.trim_start_matches(|c: char| c.is_digit(10));
        let name = name.trim();

        // Try 2-character element first
        if name.len() >= 2 {
            let two_char = &name[..2];
            if let Ok(elem) = Element::from_symbol(two_char) {
                return elem;
            }
        }

        // Try 1-character element
        if name.len() >= 1 {
            let one_char = &name[..1];
            if let Ok(elem) = Element::from_symbol(one_char) {
                return elem;
            }
        }

        warn!("Unknown element for atom name: {}, using Unknown", atom_name);
        Element::Unknown
    }
}

/// mmCIF data structure
#[derive(Debug, Default)]
struct MmcifData {
    id: String,
    metadata: HashMap<String, String>,
    categories: HashMap<String, Vec<String>>,
    atom_site: Vec<HashMap<String, String>>,
}

/// Write trajectory to mmCIF format
pub struct MmcifWriter;

impl MmcifWriter {
    /// Write a trajectory to an mmCIF file (only first frame)
    pub fn write_trajectory(path: &Path, trajectory: &Trajectory) -> IOResult<()> {
        let file = File::create(path)?;
        let mut writer = std::io::BufWriter::new(file);

        // Write data block
        writeln!(writer, "data_{}", file_path_to_id(path))?;

        // Write metadata
        writeln!(writer, "#")?;
        writeln!(writer, "_entry.id {}", file_path_to_id(path))?;
        writeln!(
            writer,
            "_struct.title {}",
            trajectory.metadata.title
        )?;

        // Write atom_site loop header
        writeln!(writer, "#")?;
        writeln!(writer, "loop_")?;
        writeln!(writer, "_atom_site.group_PDB")?;
        writeln!(writer, "_atom_site.id")?;
        writeln!(writer, "_atom_site.type_symbol")?;
        writeln!(writer, "_atom_site.label_atom_id")?;
        writeln!(writer, "_atom_site.label_alt_id")?;
        writeln!(writer, "_atom_site.label_comp_id")?;
        writeln!(writer, "_atom_site.label_asym_id")?;
        writeln!(writer, "_atom_site.label_entity_id")?;
        writeln!(writer, "_atom_site.label_seq_id")?;
        writeln!(writer, "_atom_site.Cartn_x")?;
        writeln!(writer, "_atom_site.Cartn_y")?;
        writeln!(writer, "_atom_site.Cartn_z")?;
        writeln!(writer, "_atom_site.occupancy")?;
        writeln!(writer, "_atom_site.B_iso_or_equiv")?;
        writeln!(writer, "_atom_site.pdbx_formal_charge")?;
        writeln!(writer, "_atom_site.auth_asym_id")?;
        writeln!(writer, "_atom_site.auth_seq_id")?;
        writeln!(writer, "_atom_site.pdbx_PDB_ins_code")?;

        // Write atoms
        if let Some(frame) = trajectory.frames.first() {
            for atom_id in 0..trajectory.num_atoms as u32 {
                if let Some(pos) = frame.get_position(atom_id) {
                    // Note: This is a simplified writer - in reality you'd need atom data
                    writeln!(
                        writer,
                        "ATOM {:6} X     X     . A   1 ?   ?   {:.3} {:.3} {:.3} 1.00 20.00 ? ? ? ?",
                        atom_id + 1, pos.x, pos.y, pos.z
                    )?;
                }
            }
        }

        Ok(())
    }
}

/// Helper to convert file path to mmCIF ID
fn file_path_to_id(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string()
}

/// Register mmCIF parsing systems with Bevy
pub fn register(app: &mut App) {
    info!("mmCIF parser registered");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_mmcif() {
        let mmcif_content = r#"data_test
#
_entry.id test
_struct.title Test structure
#
loop_
_atom_site.group_PDB
_atom_site.id
_atom_site.type_symbol
_atom_site.label_atom_id
_atom_site.label_alt_id
_atom_site.label_comp_id
_atom_site.label_asym_id
_atom_site.label_entity_id
_atom_site.label_seq_id
_atom_site.Cartn_x
_atom_site.Cartn_y
_atom_site.Cartn_z
ATOM 1  O  O  .  HOH  A  1  . 0.000 0.000 0.000
ATOM 2  H  H1 .  HOH  A  1  . 0.757 0.000 0.000
ATOM 3  H  H2 .  HOH  A  1  . -0.757 0.000 0.000
"#;

        let result = MmcifParser::parse_string(mmcif_content, PathBuf::from("test.cif"));

        assert!(result.is_ok());
        let trajectory = result.unwrap();
        assert_eq!(trajectory.num_frames(), 1);
        assert_eq!(trajectory.num_atoms, 3);
    }

    #[test]
    fn test_element_from_atom_name() {
        // Test common element patterns
        assert_eq!(MmcifParser::element_from_atom_name("C"), Element::C);
        assert_eq!(MmcifParser::element_from_atom_name("CA"), Element::C);
        assert_eq!(MmcifParser::element_from_atom_name("CB"), Element::C);
        assert_eq!(MmcifParser::element_from_atom_name("N"), Element::N);
        assert_eq!(MmcifParser::element_from_atom_name("O"), Element::O);
        assert_eq!(MmcifParser::element_from_atom_name("H"), Element::H);
        assert_eq!(MmcifParser::element_from_atom_name("S"), Element::S);
    }

    #[test]
    fn test_file_path_to_id() {
        let path = Path::new("/path/to/structure.cif");
        let id = file_path_to_id(path);
        assert_eq!(id, "structure");
    }
}
