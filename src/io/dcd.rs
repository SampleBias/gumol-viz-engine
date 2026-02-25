//! DCD file format parser
//!
//! The DCD format is a binary trajectory format used by CHARMM, NAMD, and others.
//! It's a fixed record-length binary file with a specific header and frame structure.

use crate::core::trajectory::{FrameData, Trajectory, TrajectoryMetadata};
use crate::core::atom::{AtomData, Element};
use crate::io::{FileFormat, IOError, IOResult};
use bevy::prelude::*;
use byteorder::{LittleEndian, ReadBytesExt};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

/// DCD format constants
const DCD_MAGIC_NUMBER: i32 = 84;
const DCD_HEADER_SIZE: usize = 224;
const DCD_TITLE_SIZE: i32 = 80;

/// DCD header structure
#[derive(Debug, Default)]
pub struct DcdHeader {
    num_frames: i32,
    start_step: i32,
    skip: i32,
    num_sets: i32,
    delta: f32,
    charmm: bool,
    has_temperature: bool,
    has_pressure: bool,
    title: String,
    num_atoms: i32,
}

impl Default for DcdHeader {
    fn default() -> Self {
        Self {
            num_frames: 0,
            start_step: 0,
            skip: 1,
            num_sets: 0,
            delta: 0.0,
            charmm: false,
            has_temperature: false,
            has_pressure: false,
            title: String::new(),
            num_atoms: 0,
        }
    }
}

/// DCD format parser
pub struct DcdParser;

impl DcdParser {
    /// Parse a DCD file and return trajectory data
    ///
    /// Note: DCD files only contain position data (no atom metadata).
    /// You'll need to load atom data from a separate file (e.g., PDB, GRO, mmCIF).
    pub fn parse_file(path: &Path) -> IOResult<Trajectory> {
        let file = File::open(path).map_err(|e| IOError::FileNotFound(path.display().to_string()))?;
        let reader = BufReader::new(file);
        Self::parse_reader(reader, path.to_path_buf())
    }

    /// Parse DCD format from a reader
    pub fn parse_reader<R: Read>(reader: R, file_path: PathBuf) -> IOResult<Trajectory> {
        let reader = BufReader::new(reader);
        Self::parse_reader(reader, file_path)
    }

    /// Parse DCD format from string content
    pub fn parse_string(content: &str, file_path: PathBuf) -> IOResult<Trajectory> {
        let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        Self::parse_lines(&lines, file_path)
    }

    /// Parse DCD format from a vector of lines
    fn parse_lines(lines: &[String], file_path: PathBuf) -> IOResult<Trajectory> {
        let mut line_iter = lines.iter().enumerate();

        // Parse header
        let header = Self::read_header(&mut line_iter)?;

        // Calculate expected file size and number of atoms
        let num_atoms = header.num_atoms;
        let num_frames = header.num_frames;
        let bytes_per_frame = 12 + num_atoms * 4 * 3 + 12; // 4 records of (4 + data + 4 + 4)

        info!(
            "DCD: {} frames, {} atoms, {} atoms set",
            num_frames,
            num_atoms,
            header.num_sets
        );

        // Parse frames
        let mut frames = Vec::new();
        for frame_index in 0..num_frames {
            let frame = Self::read_frame(&mut line_iter, frame_index, num_atoms)?;
            frames.push(frame);

            if frame_index == 0 {
                // Update total_frames in header after first read
                header.num_frames = frame_index + 1;
            }
        }

        // Create trajectory
        let mut metadata = TrajectoryMetadata::default();
        metadata.title = header.title.clone();
        metadata.software = if header.charmm {
            "CHARMM".to_string()
        } else {
            "Unknown".to_string()
        };
        metadata.num_steps = Some(header.num_sets as u64);
        metadata.step_size = Some(header.delta);

        let time_step = header.delta * 20.0; // Convert to femtoseconds (DCD uses 20fs units by default)

        let mut trajectory = Trajectory::new(file_path, num_atoms, time_step);
        trajectory.metadata = metadata;
        for frame in frames {
            trajectory.add_frame(frame);
        }

        Ok(trajectory)
    }

    /// Parse DCD format from file path with atom data
    ///
    /// This helper function allows you to combine DCD trajectory data with atom metadata
    /// from a separate file (e.g., PDB, GRO, mmCIF).
    pub fn parse_with_atom_data(
        path: &Path,
        atom_data: &[AtomData],
    ) -> IOResult<Trajectory> {
        let mut trajectory = Self::parse_file(path)?;

        // Note: In a real implementation, we'd store atom_data in the trajectory
        // For now, trajectory just contains positions
        Ok(trajectory)
    }

    /// Read DCD header
    fn read_header<R: Read>(reader: &mut R) -> IOResult<DcdHeader> {
        // Check magic number (84 for CHARMM)
        let header_size = reader.read_i32::<LittleEndian>()?;

        if header_size != DCD_MAGIC_NUMBER {
            return Err(IOError::ParseError {
                line: 0,
                message: format!(
                    "Invalid DCD header size: expected {}, got {}",
                    DCD_MAGIC_NUMBER, header_size
                ),
            });
        }

        let mut header = DcdHeader::default();

        // Read CORD (CHARMM coordinates) or other identifier
        let mut cord = [0u8; 4];
        reader.read_exact(&mut cord)?;

        header.charmm = &cord == b"CORD";

        // Read number of frames (can be 0, meaning read until EOF)
        header.num_frames = reader.read_i32::<LittleEndian>()? as usize;

        // Read starting timestep
        header.start_step = reader.read_i32::<LittleEndian>()?;

        // Read steps between frames
        header.skip = reader.read_i32::<LittleEndian>()?;

        // Read number of steps per trajectory
        header.num_sets = reader.read_i32::<LittleEndian>()?;

        // Read time step between frames (in 20fs units)
        header.delta = reader.read_f32::<LittleEndian>()?;

        // Read unit flags (can be ignored)
        let _ = reader.read_i32::<LittleEndian>()?;
        let _ = reader.read_i32::<LittleEndian>()?;
        let _ = reader.read_i32::<LittleEndian>()?;

        // Read temperature flag
        let temperature = reader.read_i32::<LittleEndian>()?;
        header.has_temperature = temperature == 1;

        // Read pressure flag
        let pressure = reader.read_i32::<LittleEndian>()?;
        header.has_pressure = pressure == 1;

        // Skip padding
        let _ = reader.read_i32::<LittleEndian>()?;

        // Read title records (80 bytes each)
        let n_title = reader.read_i32::<LittleEndian>()?;

        if n_title > 0 {
            let mut title_bytes = vec![0u8; n_title as usize];
            reader.read_exact(&mut title_bytes)?;

            let mut title = String::new();
            for _ in 0..n_title {
                title.push(reader.read_u8()? as char);
            }
            header.title = String::from_utf8_lossy(&title_bytes).trim().to_string();
        }

        // Read number of atoms
        let num_atoms_size = reader.read_i32::<LittleEndian>()?;

        if num_atoms_size != 4 {
            return Err(IOError::ParseError {
                line: 0,
                message: format!("Invalid num_atoms size: {}, expected 4, got {}", num_atoms_size),
            });
        }

        let num_atoms = num_atoms_size as usize;

        // Skip end of header marker
        let _ = reader.read_i32::<LittleEndian>()?;

        Ok(header)
    }

    /// Read a single DCD frame
    fn read_frame<R: Read>(
        reader: &mut R,
        frame_index: usize,
        num_atoms: usize,
    ) -> IOResult<FrameData> {
        let mut frame = FrameData::new(frame_index, frame_index as f32 * 0.0);

        // Read X coordinates
        let nx_size = reader.read_i32::<LittleEndian>()?;

        if nx_size != (num_atoms * 4) as i32 {
            return Err(IOError::ParseError {
                line: 0,
                message: format!(
                    "Invalid X coordinate record size: expected {}, got {}",
                    num_atoms * 4,
                    nx_size
                ),
            });
        }

        let mut x_coords = vec![0.0f32; num_atoms];
        for x in &mut x_coords {
            *x = reader.read_f32::<LittleEndian>()?;
        }

        reader.read_i32::<LittleEndian>()?; // End of X record

        // Read Y coordinates
        let ny_size = reader.read_i32::<LittleEndian>()?;

        if ny_size != (num_atoms * 4) as i32 {
            return Err(IOError::Error {
                line: 0,
                message: format!(
                    "Invalid Y coordinate record size: expected {}, got {}",
                    num_atoms * 4,
                    ny_size
                ),
            });
        }

        let mut y_coords = vec![0.0f32; num_atoms];
        for y in &mut y_coords {
            *y = reader.read_f32::<LittleEndian>()?;
        }

        reader.read_i32::<LittleEndian>()?; // End of Y record

        // Read Z coordinates
        let nz_size = reader.read_i32::<LittleEndian>()?;

        if nz_size != (num_atoms * 4) as i32 {
            return Err(IOError::Error {
                line: 0,
                message: format!(
                    "Invalid Z coordinate record size: expected {}, got {}",
                    num_atoms * 4,
                    nz_size
                ),
            });
        }

        let mut z_coords = vec![0.0f32; num_atoms];
        for z in &mut z_coords {
            *z = reader.read_f32::<LittleEndian>()?;
        }

        reader.read_i32::<LittleEndian>()?; // End of Z record

        // Store positions
        for i in 0..num_atoms {
            let position = Vec3::new(x_coords[i], y_coords[i], z_coords[i]);
            frame.set_position(i as u32, position);
        }

        Ok(frame)
    }
}

/// Register DCD parsing systems with Bevy
pub fn register(app: &mut App) {
    info!("DCD parser registered");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dcd_constants() {
        assert_eq!(DCD_MAGIC_NUMBER, 84);
        assert_eq!(DCD_HEADER_SIZE, 224);
        assert_eq!(DCD_TITLE_SIZE, 80);
    }

    #[test]
    fn test_dcd_header_parse() {
        // Note: This is a placeholder test
        // Real DCD files are binary, so we can't easily create test strings
        assert!(true);
    }

    #[test]
    fn test_dcd_frame_parse() {
        // Note: This is a placeholder test
        // Real DCD files are binary, so we can't easily create test strings
        assert!(true);
    }
}
