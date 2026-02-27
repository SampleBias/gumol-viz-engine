//! XYZ file format parser
//!
//! The XYZ format is a simple coordinate file format:
//! Line 1: Number of atoms
//! Line 2: Comment line (title)
//! Lines 3+: Element symbol X Y Z (and optional fields)

use crate::core::atom::{AtomData, Element};
use crate::core::trajectory::{FrameData, Trajectory};
use crate::io::{IOError, IOResult};
use bevy::prelude::*;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};

/// XYZ format parser
pub struct XYZParser;

impl XYZParser {
    /// Parse an XYZ file and return trajectory data
    pub fn parse_file(path: &Path) -> IOResult<Trajectory> {
        let file = File::open(path).map_err(|_e| IOError::FileNotFound(path.display().to_string()))?;
        let reader = BufReader::new(file);
        Self::parse_reader(reader, path.to_path_buf())
    }

    /// Parse XYZ format from a reader
    pub fn parse_reader<R: Read>(reader: R, file_path: PathBuf) -> IOResult<Trajectory> {
        let reader = BufReader::new(reader);
        let lines: Vec<String> = reader.lines().collect::<Result<_, _>>()?;

        Self::parse_lines(&lines, file_path)
    }

    /// Parse XYZ format from string content
    pub fn parse_string(content: &str, file_path: PathBuf) -> IOResult<Trajectory> {
        let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        Self::parse_lines(&lines, file_path)
    }

    /// Parse XYZ format from a vector of lines
    fn parse_lines(lines: &[String], file_path: PathBuf) -> IOResult<Trajectory> {
        let mut frame_index = 0;
        let mut frames = Vec::new();
        let mut num_atoms = 0;
        let mut time_step = 1.0; // Default time step (femtoseconds)

        let mut line_iter = lines.iter().enumerate();

        while let Some((line_num, line)) = line_iter.next() {
            // Skip empty lines
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // Parse number of atoms
            if frame_index == 0 {
                num_atoms = line
                    .parse::<usize>()
                    .map_err(|_| IOError::ParseError {
                        line: line_num,
                        message: format!("Expected number of atoms, got: {}", line),
                    })?;
            } else {
                let current_num_atoms = line.parse::<usize>().map_err(|_| IOError::ParseError {
                    line: line_num,
                    message: format!("Expected number of atoms, got: {}", line),
                })?;
                if current_num_atoms != num_atoms {
                    return Err(IOError::ParseError {
                        line: line_num,
                        message: format!(
                            "Number of atoms changed from {} to {}",
                            num_atoms, current_num_atoms
                        ),
                    });
                }
            }

            // Read comment line
            if let Some((_comment_line_num, comment_line)) = line_iter.next() {
                let comment = comment_line.trim();

                // Try to extract time from comment line
                // Common formats: "time=100.0", "t=100.0", "frame=10", "i=10"
                if let Some(time_str) = comment
                    .split_whitespace()
                    .find(|s| s.starts_with("time=") || s.starts_with("t="))
                {
                    if let Some(time_str) = time_str.split('=').nth(1) {
                        if let Ok(t) = time_str.parse::<f32>() {
                            time_step = t;
                        }
                    }
                }
            }

            // Read atom positions
            let mut frame = FrameData::new(frame_index, frame_index as f32 * time_step);
            let mut atom_data_map = HashMap::new();

            for i in 0..num_atoms {
                if let Some((atom_line_num, atom_line)) = line_iter.next() {
                    let parts: Vec<&str> = atom_line.split_whitespace().collect();

                    if parts.len() < 4 {
                        return Err(IOError::ParseError {
                            line: atom_line_num,
                            message: format!(
                                "Expected at least 4 fields (element X Y Z), got {}",
                                parts.len()
                            ),
                        });
                    }

                    // Parse element
                    let element = Element::from_symbol(parts[0]).unwrap_or_else(|_| {
                        warn!("Unknown element: {}, using Unknown", parts[0]);
                        Element::Unknown
                    });

                    // Parse position
                    let x = parts[1]
                        .parse::<f32>()
                        .map_err(|_| IOError::ParseError {
                            line: atom_line_num,
                            message: format!("Invalid X coordinate: {}", parts[1]),
                        })?;
                    let y = parts[2]
                        .parse::<f32>()
                        .map_err(|_| IOError::ParseError {
                            line: atom_line_num,
                            message: format!("Invalid Y coordinate: {}", parts[2]),
                        })?;
                    let z = parts[3]
                        .parse::<f32>()
                        .map_err(|_| IOError::ParseError {
                            line: atom_line_num,
                            message: format!("Invalid Z coordinate: {}", parts[3]),
                        })?;

                    let position = Vec3::new(x, y, z);

                    // Set position in frame
                    frame.set_position(i as u32, position);

                    // Create atom data (only for first frame)
                    if frame_index == 0 {
                        let atom_data = AtomData::new(
                            i as u32,
                            element,
                            0, // residue ID
                            "UNK".to_string(), // residue name
                            "A".to_string(),    // chain ID
                            element.symbol().to_string(),
                        );
                        atom_data_map.insert(i as u32, atom_data);
                    }
                } else {
                    return Err(IOError::ParseError {
                        line: line_num,
                        message: "Unexpected end of file while reading atoms".to_string(),
                    });
                }
            }

            // Store atom data for first frame
            if frame_index == 0 && !atom_data_map.is_empty() {
                // In a real implementation, we'd store this in the trajectory
            }

            frames.push(frame);
            frame_index += 1;
        }

        // Create trajectory
        let mut trajectory = Trajectory::new(file_path, num_atoms, time_step);
        for frame in frames {
            trajectory.add_frame(frame);
        }

        Ok(trajectory)
    }

    /// Parse multiple frames from an XYZ file
    pub fn parse_multi_frame(path: &Path) -> IOResult<Trajectory> {
        Self::parse_file(path)
    }

    /// Stream frames from a large XYZ file (memory-efficient)
    pub fn stream_frames(path: &Path) -> IOResult<FrameStream> {
        let file = File::open(path).map_err(|_e| IOError::FileNotFound(path.display().to_string()))?;
        Ok(FrameStream {
            reader: BufReader::new(file),
            file_path: path.to_path_buf(),
            current_frame: 0,
            num_atoms: 0,
            time_step: 1.0,
        })
    }
}

/// Stream frames from an XYZ file
pub struct FrameStream {
    reader: BufReader<File>,
    file_path: PathBuf,
    current_frame: usize,
    num_atoms: usize,
    time_step: f32,
}

impl FrameStream {
    /// Read the next frame
    pub fn next_frame(&mut self) -> IOResult<Option<FrameData>> {
        // Read number of atoms
        let mut line = String::new();
        if self.reader.read_line(&mut line)? == 0 {
            return Ok(None); // End of file
        }

        let num_atoms = line
            .trim()
            .parse::<usize>()
            .map_err(|_| IOError::ParseError {
                line: self.current_frame * (self.num_atoms + 2),
                message: format!("Expected number of atoms, got: {}", line),
            })?;

        if self.current_frame == 0 {
            self.num_atoms = num_atoms;
        }

        // Read comment line
        line.clear();
        self.reader.read_line(&mut line)?;

        // Parse atoms
        let mut frame = FrameData::new(self.current_frame, self.current_frame as f32 * self.time_step);

        for i in 0..num_atoms {
            line.clear();
            self.reader.read_line(&mut line)?;

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 4 {
                return Err(IOError::ParseError {
                    line: self.current_frame * (self.num_atoms + 2) + 2 + i,
                    message: format!("Expected at least 4 fields, got {}", parts.len()),
                });
            }

            let x = parts[1].parse::<f32>().map_err(|_| IOError::ParseError {
                line: self.current_frame * (self.num_atoms + 2) + 2 + i,
                message: format!("Invalid X coordinate: {}", parts[1]),
            })?;
            let y = parts[2].parse::<f32>().map_err(|_| IOError::ParseError {
                line: self.current_frame * (self.num_atoms + 2) + 2 + i,
                message: format!("Invalid Y coordinate: {}", parts[2]),
            })?;
            let z = parts[3].parse::<f32>().map_err(|_| IOError::ParseError {
                line: self.current_frame * (self.num_atoms + 2) + 2 + i,
                message: format!("Invalid Z coordinate: {}", parts[3]),
            })?;

            frame.set_position(i as u32, Vec3::new(x, y, z));
        }

        self.current_frame += 1;
        Ok(Some(frame))
    }
}

/// Write trajectory to XYZ format
pub struct XYZWriter;

impl XYZWriter {
    /// Write a trajectory to an XYZ file
    pub fn write_trajectory(path: &Path, trajectory: &Trajectory) -> IOResult<()> {
        let mut file = File::create(path)?;

        for frame in &trajectory.frames {
            Self::write_frame(&mut file, frame)?;
        }

        Ok(())
    }

    /// Write a single frame to XYZ format
    pub fn write_frame<W: std::io::Write>(writer: &mut W, frame: &FrameData) -> IOResult<()> {
        use std::io::Write;

        // Write number of atoms
        writeln!(writer, "{}", frame.positions.len())?;

        // Write comment line with time
        writeln!(writer, "time={:.2} frame={}", frame.time, frame.index)?;

        // Write atom positions
        let mut sorted_ids: Vec<_> = frame.positions.keys().copied().collect();
        sorted_ids.sort();

        for atom_id in sorted_ids {
            if let Some(pos) = frame.get_position(atom_id) {
                writeln!(writer, "X {:.6} {:.6} {:.6}", pos.x, pos.y, pos.z)?;
            }
        }

        Ok(())
    }
}

/// Register XYZ parsing systems with Bevy
pub fn register(_app: &mut App) {
    info!("XYZ parser registered");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_xyz() {
        let xyz_content = r#"3
water
O 0.0 0.0 0.0
H 0.757 0.0 0.0
H -0.757 0.0 0.0"#;

        let result = XYZParser::parse_string(
            xyz_content,
            PathBuf::from("test.xyz"),
        );

        assert!(result.is_ok());
        let trajectory = result.unwrap();
        assert_eq!(trajectory.num_frames(), 1);
        assert_eq!(trajectory.num_atoms, 3);
    }

    #[test]
    fn test_parse_multi_frame_xyz() {
        let xyz_content = r#"2
frame 0
X 0.0 0.0 0.0
X 1.0 0.0 0.0
2
frame 1
X 0.1 0.0 0.0
X 1.1 0.0 0.0"#;

        let result = XYZParser::parse_string(
            xyz_content,
            PathBuf::from("test.xyz"),
        );

        assert!(result.is_ok());
        let trajectory = result.unwrap();
        assert_eq!(trajectory.num_frames(), 2);
    }

    #[test]
    fn test_write_xyz() {
        let mut frame = FrameData::new(0, 0.0);
        frame.set_position(0, Vec3::new(0.0, 0.0, 0.0));
        frame.set_position(1, Vec3::new(1.0, 0.0, 0.0));

        let mut buffer = Vec::new();
        XYZWriter::write_frame(&mut buffer, &frame).unwrap();

        let output = String::from_utf8(buffer).unwrap();
        assert!(output.contains("2"));
        assert!(output.contains("time=0.00"));
    }
}
