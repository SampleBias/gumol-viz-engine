//! GRO file format parser
//!
//! The GRO format is a coordinate file format used by GROMACS:
//! Line 1: Title
//! Line 2: Number of atoms
//! Lines 3+: residue number (5) residue name (5) atom name (5) atom number (5) x y z (8.3 8.3 8.3) vx vy vz (8.4 8.4 8.4)
//! Last line: box vectors (9 8.4)

use crate::core::atom::{AtomData, Element};
use crate::core::trajectory::{FrameData, Trajectory, TrajectoryMetadata};
use crate::io::{IOError, IOResult};
use bevy::prelude::*;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use tracing::warn;

/// Parsed atom data from a GRO line
#[derive(Debug, Clone)]
pub struct ParsedAtom {
    pub residue_id: i32,
    pub residue_name: String,
    pub atom_name: String,
    pub element: Element,
    pub position: Vec3,
    pub velocity: Option<Vec3>,
}

/// GRO format parser
pub struct GroParser;

impl GroParser {
    /// Parse a GRO file and return trajectory data
    pub fn parse_file(path: &Path) -> IOResult<Trajectory> {
        let file = File::open(path).map_err(|_e| IOError::FileNotFound(path.display().to_string()))?;
        let reader = BufReader::new(file);
        Self::parse_reader(reader, path.to_path_buf())
    }

    /// Parse GRO format from a reader
    pub fn parse_reader<R: Read>(reader: R, file_path: PathBuf) -> IOResult<Trajectory> {
        let reader = BufReader::new(reader);
        let lines: Vec<String> = reader.lines().collect::<Result<_, _>>()?;

        Self::parse_lines(&lines, file_path)
    }

    /// Parse GRO format from string content
    pub fn parse_string(content: &str, file_path: PathBuf) -> IOResult<Trajectory> {
        let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        Self::parse_lines(&lines, file_path)
    }

    /// Parse GRO format from a vector of lines
    fn parse_lines(lines: &[String], file_path: PathBuf) -> IOResult<Trajectory> {
        if lines.is_empty() {
            return Err(IOError::ParseError {
                line: 0,
                message: "Empty GRO file".to_string(),
            });
        }

        let mut line_iter = lines.iter().enumerate();

        // Read title line (line 1)
        let title = if let Some((_, line)) = line_iter.next() {
            line.trim().to_string()
        } else {
            return Err(IOError::ParseError {
                line: 1,
                message: "Missing title line".to_string(),
            });
        };

        // Read number of atoms (line 2)
        let num_atoms = if let Some((line_num, line)) = line_iter.next() {
            line.trim()
                .parse::<usize>()
                .map_err(|_| IOError::ParseError {
                    line: line_num + 1,
                    message: format!("Expected number of atoms, got: {}", line),
                })?
        } else {
            return Err(IOError::ParseError {
                line: 2,
                message: "Missing atom count line".to_string(),
            });
        };

        if num_atoms == 0 {
            return Err(IOError::ParseError {
                line: 2,
                message: "Number of atoms cannot be zero".to_string(),
            });
        }

        // Read atom lines
        let mut frame = FrameData::new(0, 0.0);
        let mut atom_data_map = HashMap::new();

        for i in 0..num_atoms {
            if let Some((line_num, line)) = line_iter.next() {
                // GRO format is column-based, not whitespace-delimited
                // resid(5) resname(5) atomname(5) atomnr(5) x(8.3) y(8.3) z(8.3) vx(8.4) vy(8.4) vz(8.4)
                let parsed = Self::parse_atom_line(line, line_num + 1, i)?;

                // Create atom data
                let atom_data = AtomData::new(
                    i as u32,
                    parsed.element,
                    parsed.residue_id as u32,
                    parsed.residue_name,
                    "A".to_string(), // GRO doesn't have chain ID
                    parsed.atom_name,
                );
                atom_data_map.insert(i as u32, atom_data.clone());

                // Set position
                frame.set_position(i as u32, parsed.position);

                // Set velocity if available
                if let Some(velocity) = parsed.velocity {
                    if frame.velocities.is_none() {
                        frame.velocities = Some(HashMap::new());
                    }
                    frame.velocities.as_mut().unwrap().insert(i as u32, velocity);
                }
            } else {
                return Err(IOError::ParseError {
                    line: 0,
                    message: format!("Expected {} atom lines, found {}", num_atoms, i),
                });
            }
        }

        // Read box dimensions (last line) - optional
        let mut box_size = None;
        if let Some((line_num, line)) = line_iter.next() {
            // Box vectors: xx yy zz
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let xx = parts[0]
                    .parse::<f32>()
                    .map_err(|_| IOError::ParseError {
                        line: line_num + 1,
                        message: format!("Invalid box xx: {}", parts[0]),
                    })?;
                let yy = parts[1]
                    .parse::<f32>()
                    .map_err(|_| IOError::ParseError {
                        line: line_num + 1,
                        message: format!("Invalid box yy: {}", parts[1]),
                    })?;
                let zz = parts[2]
                    .parse::<f32>()
                    .map_err(|_| IOError::ParseError {
                        line: line_num + 1,
                        message: format!("Invalid box zz: {}", parts[2]),
                    })?;
                box_size = Some([xx, yy, zz]);
            }
        }

        frame.box_size = box_size;

        // Create trajectory
        let mut metadata = TrajectoryMetadata::default();
        metadata.title = title;
        metadata.software = "GROMACS".to_string();

        let mut trajectory = Trajectory::new(file_path, num_atoms, 1.0);
        trajectory.metadata = metadata;
        trajectory.add_frame(frame);

        Ok(trajectory)
    }

    /// Parse a single atom line from GRO format
    pub fn parse_atom_line(line: &str, line_num: usize, atom_id: usize) -> IOResult<ParsedAtom> {
        let line = line.trim();

        // Minimum required: resid(5) resname(5) atomname(5) atomnr(5) x(8.3) y(8.3) z(8.3)
        // Total minimum width: 5+5+5+5+8+8+8 = 44 characters
        if line.len() < 44 {
            return Err(IOError::ParseError {
                line: line_num,
                message: format!(
                    "Line too short ({} chars), expected at least 44",
                    line.len()
                ),
            });
        }

        // Parse residue number (columns 1-5)
        let residue_id_str = &line[0..5].trim();
        let residue_id = residue_id_str
            .parse::<i32>()
            .unwrap_or_else(|_| atom_id as i32);

        // Parse residue name (columns 6-10)
        let residue_name = line[5..10].trim().to_string();
        let residue_name = if residue_name.is_empty() {
            "UNK".to_string()
        } else {
            residue_name
        };

        // Parse atom name (columns 11-15)
        let atom_name = line[10..15].trim().to_string();
        let atom_name = if atom_name.is_empty() {
            "X".to_string()
        } else {
            atom_name
        };

        // Parse atom number (columns 16-20)
        let _atom_number_str = &line[15..20].trim();

        // Parse coordinates (columns 21-28, 29-36, 37-44)
        let x_str = &line[20..28].trim();
        let y_str = &line[28..36].trim();
        let z_str = &line[36..44].trim();

        let x = x_str
            .parse::<f32>()
            .map_err(|_| IOError::ParseError {
                line: line_num,
                message: format!("Invalid X coordinate: {}", x_str),
            })?;
        let y = y_str
            .parse::<f32>()
            .map_err(|_| IOError::ParseError {
                line: line_num,
                message: format!("Invalid Y coordinate: {}", y_str),
            })?;
        let z = z_str
            .parse::<f32>()
            .map_err(|_| IOError::ParseError {
                line: line_num,
                message: format!("Invalid Z coordinate: {}", z_str),
            })?;

        let position = Vec3::new(x, y, z);

        // Parse velocities if present (columns 45-52, 53-60, 61-68)
        let velocity = if line.len() >= 68 {
            let vx_str = &line[44..52].trim();
            let vy_str = &line[52..60].trim();
            let vz_str = &line[60..68].trim();

            match (
                vx_str.parse::<f32>(),
                vy_str.parse::<f32>(),
                vz_str.parse::<f32>(),
            ) {
                (Ok(vx), Ok(vy), Ok(vz)) => Some(Vec3::new(vx, vy, vz)),
                _ => None,
            }
        } else {
            None
        };

        // Determine element from atom name
        // GROMACS atom names typically start with the element symbol
        let element = Self::element_from_atom_name(&atom_name);

        Ok(ParsedAtom {
            residue_id,
            residue_name,
            atom_name,
            element,
            position,
            velocity,
        })
    }

    /// Determine element from atom name
    pub fn element_from_atom_name(atom_name: &str) -> Element {
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

        // Common GROMACS atom name patterns
        if name.starts_with("OW") || name.starts_with("HW") {
            return Element::O;
        }

        warn!("Unknown element for atom name: {}, using Unknown", atom_name);
        Element::Unknown
    }
}

/// Write trajectory to GRO format
pub struct GroWriter;

impl GroWriter {
    /// Write a trajectory to a GRO file (only first frame)
    pub fn write_trajectory(path: &Path, trajectory: &Trajectory) -> IOResult<()> {
        let file = File::create(path)?;
        let mut writer = std::io::BufWriter::new(file);

        // Write title line
        writeln!(writer, "{}", trajectory.metadata.title)?;

        // Write atom count
        writeln!(writer, "{}", trajectory.num_atoms)?;

        // Write atoms
        if let Some(frame) = trajectory.frames.first() {
            for atom_id in 0..trajectory.num_atoms as u32 {
                if let Some(pos) = frame.get_position(atom_id) {
                    let residue_name = "UNK"; // Would need atom data
                    let atom_name = "X";
                    let residue_id = (atom_id + 1) as i32;

                    // Format: resid(5) resname(5) atomname(5) atomnr(5) x(8.3) y(8.3) z(8.3)
                    writeln!(
                        writer,
                        "{:5}{:5}{:5}{:5}{:8.3}{:8.3}{:8.3}",
                        residue_id, residue_name, atom_name, atom_id + 1, pos.x, pos.y, pos.z
                    )?;
                }
            }

            // Write box dimensions
            if let Some(box_size) = frame.box_size {
                writeln!(
                    writer,
                    " {:8.4}{:8.4}{:8.4}",
                    box_size[0], box_size[1], box_size[2]
                )?;
            } else {
                writeln!(writer, "  0.0000   0.0000   0.0000")?;
            }
        }

        Ok(())
    }
}

/// Register GRO parsing systems with Bevy
pub fn register(_app: &mut App) {
    info!("GRO parser registered");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_gro() {
        let gro_content = r#"Water molecule
3
    1SOL    OW    1   0.126   0.639   0.322
    1SOL   HW1    2   0.187   0.713   0.394
    1SOL   HW2    3   0.145   0.584   0.235
   0.0000   0.0000   0.0000"#;

        let result = GroParser::parse_string(gro_content, PathBuf::from("test.gro"));

        assert!(result.is_ok());
        let trajectory = result.unwrap();
        assert_eq!(trajectory.num_frames(), 1);
        assert_eq!(trajectory.num_atoms, 3);
    }

    #[test]
    fn test_parse_gro_with_velocities() {
        let gro_content = r#"Water with velocities
3
    1SOL    OW    1   0.126   0.639   0.322   0.0001   0.0002   0.0003
    1SOL   HW1    2   0.187   0.713   0.394   0.0004   0.0005   0.0006
    1SOL   HW2    3   0.145   0.584   0.235   0.0007   0.0008   0.0009
   0.0000   0.0000   0.0000"#;

        let result = GroParser::parse_string(gro_content, PathBuf::from("test.gro"));

        assert!(result.is_ok());
        let trajectory = result.unwrap();
        assert_eq!(trajectory.num_frames(), 1);
        assert_eq!(trajectory.num_atoms, 3);

        let frame = trajectory.get_frame(0).unwrap();
        assert!(frame.velocities.is_some());
    }

    #[test]
    fn test_element_from_atom_name() {
        // Test common element patterns
        assert_eq!(GroParser::element_from_atom_name("C"), Element::C);
        assert_eq!(GroParser::element_from_atom_name("CA"), Element::C);
        assert_eq!(GroParser::element_from_atom_name("CB"), Element::C);
        assert_eq!(GroParser::element_from_atom_name("N"), Element::N);
        assert_eq!(GroParser::element_from_atom_name("O"), Element::O);
        assert_eq!(GroParser::element_from_atom_name("OW"), Element::O);
        assert_eq!(GroParser::element_from_atom_name("H"), Element::H);
        assert_eq!(GroParser::element_from_atom_name("HW"), Element::H);
        assert_eq!(GroParser::element_from_atom_name("S"), Element::S);
    }

    #[test]
    fn test_parse_atom_line() {
        let line = "    1SOL    OW    1   0.126   0.639   0.322";
        let result = GroParser::parse_atom_line(line, 3, 0);

        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert_eq!(parsed.residue_id, 1);
        assert_eq!(parsed.residue_name, "SOL");
        assert_eq!(parsed.atom_name, "OW");
        assert_eq!(parsed.element, Element::O);
        assert_eq!(parsed.position.x, 0.126);
        assert_eq!(parsed.position.y, 0.639);
        assert_eq!(parsed.position.z, 0.322);
    }
}
