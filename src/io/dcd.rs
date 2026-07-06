//! DCD file format parser
//!
//! The DCD format is a binary trajectory format used by CHARMM, NAMD, and others.
//! Supports full load for small trajectories and seek-based streaming for large ones.

use crate::core::atom::AtomData;
use crate::core::trajectory::{FrameData, Trajectory, TrajectoryMetadata};
use crate::io::{IOError, IOResult};
use bevy::prelude::*;
use byteorder::{LittleEndian, ReadBytesExt};
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

/// DCD format constants
pub const DCD_MAGIC_NUMBER: i32 = 84;

/// DCD header structure
#[derive(Debug, Clone)]
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

/// Seekable DCD reader for streaming frame access.
pub struct DcdReader {
    file: File,
    header: DcdHeader,
    first_frame_offset: u64,
    frame_stride: u64,
    time_step: f32,
}

impl DcdReader {
    /// Open a DCD file and parse the header.
    pub fn open(path: &Path) -> IOResult<Self> {
        let file =
            File::open(path).map_err(|_| IOError::FileNotFound(path.display().to_string()))?;
        let mut reader = BufReader::new(file);
        let (header, first_frame_offset) = Self::read_header(&mut reader)?;
        let frame_stride = Self::frame_stride(header.num_atoms as usize);
        let time_step = header.delta * 20.0;

        let file = reader.into_inner();

        Ok(Self {
            file,
            header,
            first_frame_offset,
            frame_stride,
            time_step,
        })
    }

    pub fn header(&self) -> &DcdHeader {
        &self.header
    }

    pub fn num_frames(&self) -> usize {
        if self.header.num_frames > 0 {
            self.header.num_frames as usize
        } else {
            1
        }
    }

    pub fn num_atoms(&self) -> usize {
        self.header.num_atoms as usize
    }

    pub fn time_step(&self) -> f32 {
        self.time_step
    }

    /// Read a single frame by index (0-based).
    pub fn read_frame(&self, frame_index: usize) -> IOResult<FrameData> {
        let offset = self
            .first_frame_offset
            .saturating_add(self.frame_stride.saturating_mul(frame_index as u64));
        let mut file = &self.file;
        file.seek(SeekFrom::Start(offset))?;
        Self::read_frame_at(
            &mut file,
            frame_index,
            self.header.num_atoms as usize,
            frame_index as f32 * self.time_step,
        )
    }

    fn frame_stride(num_atoms: usize) -> u64 {
        let coord_block = 4 + num_atoms * 4 + 4;
        (coord_block * 3) as u64
    }

    /// Read DCD header (Fortran unformatted format). Returns header and byte offset to first frame.
    fn read_header<R: Read>(reader: &mut R) -> IOResult<(DcdHeader, u64)> {
        let start = 0_u64;
        let mut bytes_read = 0_u64;

        let rec1_size = reader.read_i32::<LittleEndian>()?;
        bytes_read += 4;

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

        let mut cord = [0u8; 4];
        reader.read_exact(&mut cord)?;
        bytes_read += 4;
        header.charmm = &cord == b"CORD";

        header.num_frames = reader.read_i32::<LittleEndian>()?;
        header.start_step = reader.read_i32::<LittleEndian>()?;
        header.skip = reader.read_i32::<LittleEndian>()?;
        header.num_sets = reader.read_i32::<LittleEndian>()?;
        bytes_read += 16;

        for _ in 0..5 {
            let _ = reader.read_i32::<LittleEndian>()?;
            bytes_read += 4;
        }
        let _ = reader.read_i32::<LittleEndian>()?;
        bytes_read += 4;

        header.delta = reader.read_f64::<LittleEndian>()? as f32;
        bytes_read += 8;

        for _ in 0..9 {
            let _ = reader.read_i32::<LittleEndian>()?;
            bytes_read += 4;
        }
        let _ = reader.read_i32::<LittleEndian>()?;
        bytes_read += 4;

        let _ = reader.read_i32::<LittleEndian>()?;
        bytes_read += 4;
        let n_title = reader.read_i32::<LittleEndian>()?;
        bytes_read += 8;

        if n_title > 0 {
            let title_rec_size = reader.read_i32::<LittleEndian>()?;
            bytes_read += 4;
            let title_len = title_rec_size as usize;
            let mut title_bytes = vec![0u8; title_len];
            reader.read_exact(&mut title_bytes)?;
            bytes_read += title_len as u64;
            header.title = String::from_utf8_lossy(&title_bytes).trim().to_string();
            let _ = reader.read_i32::<LittleEndian>()?;
            bytes_read += 4;
        }

        let natom_rec_size = reader.read_i32::<LittleEndian>()?;
        bytes_read += 4;
        if natom_rec_size != 4 {
            return Err(IOError::ParseError {
                line: 0,
                message: format!("Invalid NATOM record size: {}", natom_rec_size),
            });
        }
        header.num_atoms = reader.read_i32::<LittleEndian>()?;
        let _ = reader.read_i32::<LittleEndian>()?;
        bytes_read += 8;

        Ok((header, start + bytes_read))
    }

    /// Read a single DCD frame from the current reader position.
    fn read_frame_at<R: Read>(
        reader: &mut R,
        frame_index: usize,
        num_atoms: usize,
        time: f32,
    ) -> IOResult<FrameData> {
        let mut frame = FrameData::new(frame_index, time);

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
        reader.read_i32::<LittleEndian>()?;

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
        reader.read_i32::<LittleEndian>()?;

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
        reader.read_i32::<LittleEndian>()?;

        for i in 0..num_atoms {
            frame.set_position(i as u32, Vec3::new(x_coords[i], y_coords[i], z_coords[i]));
        }

        Ok(frame)
    }
}

/// DCD format parser
pub struct DcdParser;

impl DcdParser {
    /// Parse a DCD file and return trajectory data (loads all frames into RAM).
    pub fn parse_file(path: &Path) -> IOResult<Trajectory> {
        let reader = DcdReader::open(path)?;
        let header = reader.header().clone();
        let num_atoms = reader.num_atoms();
        let num_frames = reader.num_frames();

        info!("DCD: {} frames, {} atoms", num_frames, num_atoms);

        let metadata = TrajectoryMetadata {
            title: header.title.clone(),
            software: if header.charmm {
                "CHARMM".to_string()
            } else {
                "NAMD/CHARMM".to_string()
            },
            num_steps: Some(header.num_sets as u64),
            step_size: Some(header.delta),
            ..Default::default()
        };

        let mut trajectory = Trajectory::new(path.to_path_buf(), num_atoms, reader.time_step());
        trajectory.metadata = metadata;

        for frame_index in 0..num_frames {
            trajectory.add_frame(reader.read_frame(frame_index)?);
        }

        Ok(trajectory)
    }

    /// Parse DCD with atom metadata from a topology file (coordinates only from DCD).
    pub fn parse_with_atom_data(path: &Path, _atom_data: &[AtomData]) -> IOResult<Trajectory> {
        Self::parse_file(path)
    }

    /// Check whether bytes look like a DCD file (magic number 84).
    pub fn is_dcd_bytes(data: &[u8]) -> bool {
        data.len() >= 4
            && i32::from_le_bytes([data[0], data[1], data[2], data[3]]) == DCD_MAGIC_NUMBER
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
    fn test_dcd_magic_detection() {
        assert!(DcdParser::is_dcd_bytes(&84_i32.to_le_bytes()));
        assert!(!DcdParser::is_dcd_bytes(b"ATOM"));
    }

    #[test]
    fn test_frame_stride() {
        assert_eq!(DcdReader::frame_stride(100), (8 + 100 * 4) * 3);
    }
}
