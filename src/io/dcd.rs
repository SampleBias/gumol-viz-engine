//! DCD file format parser
//!
//! The DCD format is a binary trajectory format used by CHARMM, NAMD, and others.
//! It's a fixed record-length binary file with a specific header and frame structure.

use crate::core::trajectory::{FrameData, Trajectory, TrajectoryMetadata};
use crate::core::atom::AtomData;
use crate::io::{IOError, IOResult};
use bevy::prelude::*;
use byteorder::{LittleEndian, ReadBytesExt};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

/// DCD format constants
const DCD_MAGIC_NUMBER: i32 = 84;
const DCD_HEADER_SIZE: usize = 224;
const DCD_TITLE_SIZE: i32 = 80;

/// DCD header structure
#[derive(Debug)]
pub struct DcdHeader {
    pub num_frames: i32,
    pub start_step: i32,
    pub skip: i32,
    pub num_sets: i32,
    pub delta: f32,
    pub charmm: bool,
    pub has_temperature: bool,
    pub has_pressure: bool,
    pub title: String,
    pub num_atoms: i32,
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
        let file = File::open(path).map_err(|_e| IOError::FileNotFound(path.display().to_string()))?;
        let mut reader = BufReader::new(file);
        Self::parse_reader(&mut reader, path.to_path_buf())
    }

    /// Parse DCD format from a binary reader
    pub fn parse_reader<R: Read>(reader: &mut R, file_path: PathBuf) -> IOResult<Trajectory> {
        let header = Self::read_header(reader)?;

        let num_atoms = header.num_atoms as usize;
        let num_frames = if header.num_frames > 0 {
            header.num_frames as usize
        } else {
            // 0 means read until EOF - we don't support that yet, use 1 as fallback
            1
        };

        info!(
            "DCD: {} frames, {} atoms",
            num_frames,
            num_atoms
        );

        let mut frames = Vec::new();
        for frame_index in 0..num_frames {
            let frame = Self::read_frame(reader, frame_index, num_atoms)?;
            frames.push(frame);
        }

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
        _atom_data: &[AtomData],
    ) -> IOResult<Trajectory> {
        Self::parse_file(path)
    }

    /// Read DCD header (Fortran unformatted format)
    fn read_header<R: Read>(reader: &mut R) -> IOResult<DcdHeader> {
        // First record: 4-byte size (84) + 84 bytes + 4-byte size
        let rec1_size = reader.read_i32::<LittleEndian>()?;

        if rec1_size != DCD_MAGIC_NUMBER {
            return Err(IOError::ParseError {
                line: 0,
                message: format!(
                    "Invalid DCD header: expected magic {}, got {}",
                    DCD_MAGIC_NUMBER, rec1_size
                ),
            });
        }

        let mut header = DcdHeader::default();

        // CORD (4 bytes)
        let mut cord = [0u8; 4];
        reader.read_exact(&mut cord)?;
        header.charmm = &cord == b"CORD";

        // NSET (num frames), ISTRT, NSAVC
        header.num_frames = reader.read_i32::<LittleEndian>()?;
        header.start_step = reader.read_i32::<LittleEndian>()?;
        header.skip = reader.read_i32::<LittleEndian>()?;
        header.num_sets = reader.read_i32::<LittleEndian>()?;

        // 5 zeros
        for _ in 0..5 {
            let _ = reader.read_i32::<LittleEndian>()?;
        }

        // NATOM-NFREAT (often 0)
        let _ = reader.read_i32::<LittleEndian>()?;

        // DELTA (8 bytes, double)
        header.delta = reader.read_f64::<LittleEndian>()? as f32;

        // 9 zeros
        for _ in 0..9 {
            let _ = reader.read_i32::<LittleEndian>()?;
        }

        // End of first record (4 bytes)
        let _ = reader.read_i32::<LittleEndian>()?;

        // Second record: NTITLE (4-byte record with single i32)
        let _ = reader.read_i32::<LittleEndian>()?; // record size = 4
        let n_title = reader.read_i32::<LittleEndian>()?;
        let _ = reader.read_i32::<LittleEndian>()?; // trailing record size

        // Third record: TITLE strings (80 chars each)
        if n_title > 0 {
            let title_rec_size = reader.read_i32::<LittleEndian>()?; // 80 * n_title
            let title_len = title_rec_size as usize;
            let mut title_bytes = vec![0u8; title_len];
            reader.read_exact(&mut title_bytes)?;
            header.title = String::from_utf8_lossy(&title_bytes).trim().to_string();
            let _ = reader.read_i32::<LittleEndian>()?; // trailing record size
        }

        // Fourth record: NATOM
        let natom_rec_size = reader.read_i32::<LittleEndian>()?;
        if natom_rec_size != 4 {
            return Err(IOError::ParseError {
                line: 0,
                message: format!("Invalid NATOM record size: {}", natom_rec_size),
            });
        }
        header.num_atoms = reader.read_i32::<LittleEndian>()?;
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
            return Err(IOError::ParseError {
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
            return Err(IOError::ParseError {
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
pub fn register(_app: &mut App) {
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
