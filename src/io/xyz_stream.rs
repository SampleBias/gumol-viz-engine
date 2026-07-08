//! Seek-based XYZ trajectory streaming for large multi-frame files.
//!
//! Builds a byte-offset index on open and loads individual frames on demand
//! through the shared [`FrameProvider`] interface.

use crate::core::trajectory::{FrameData, TrajectoryMetadata};
use crate::io::streaming::FrameProvider;
use crate::io::{IOError, IOResult};
use bevy::prelude::*;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Indexed metadata for a multi-frame XYZ file.
#[derive(Debug, Clone)]
pub struct XyzIndex {
    pub num_atoms: usize,
    pub num_frames: usize,
    pub time_step: f32,
    pub frame_offsets: Vec<u64>,
    pub metadata: TrajectoryMetadata,
}

fn parse_time_step_from_comment(comment: &str) -> Option<f32> {
    comment.split_whitespace().find_map(|token| {
        if let Some(v) = token.strip_prefix("time=") {
            v.parse().ok()
        } else if let Some(v) = token.strip_prefix("t=") {
            v.parse().ok()
        } else {
            None
        }
    })
}

/// Scan an XYZ file once and record the byte offset of each frame header.
pub fn build_xyz_index(path: &Path) -> IOResult<XyzIndex> {
    let file = File::open(path).map_err(|_| IOError::FileNotFound(path.display().to_string()))?;
    let mut reader = BufReader::new(file);

    let mut frame_offsets = Vec::new();
    let mut num_atoms = 0usize;
    let mut time_step = 1.0f32;
    let mut title = String::new();
    let mut frame_index = 0usize;

    loop {
        let offset = reader.stream_position().map_err(IOError::Io)?;

        let mut header = String::new();
        let bytes = reader.read_line(&mut header).map_err(IOError::Io)?;
        if bytes == 0 {
            break;
        }

        let trimmed = header.trim();
        if trimmed.is_empty() {
            continue;
        }

        let count = trimmed.parse::<usize>().map_err(|_| IOError::ParseError {
            line: frame_index,
            message: format!("Expected atom count, got: {trimmed}"),
        })?;

        if frame_index == 0 {
            num_atoms = count;
        } else if count != num_atoms {
            return Err(IOError::ParseError {
                line: frame_index,
                message: format!(
                    "Atom count changed from {num_atoms} to {count} in frame {frame_index}"
                ),
            });
        }

        frame_offsets.push(offset);

        let mut comment = String::new();
        reader.read_line(&mut comment).map_err(IOError::Io)?;
        if frame_index == 0 {
            title = comment.trim().to_string();
            if let Some(t) = parse_time_step_from_comment(&title) {
                time_step = t;
            }
        }

        for atom_i in 0..num_atoms {
            let mut atom_line = String::new();
            let n = reader.read_line(&mut atom_line).map_err(IOError::Io)?;
            if n == 0 {
                return Err(IOError::ParseError {
                    line: frame_index,
                    message: format!("Unexpected EOF in frame {frame_index} at atom {atom_i}"),
                });
            }
        }

        frame_index += 1;
    }

    if frame_offsets.is_empty() {
        return Err(IOError::ParseError {
            line: 0,
            message: "No frames found in XYZ file".into(),
        });
    }

    Ok(XyzIndex {
        num_atoms,
        num_frames: frame_offsets.len(),
        time_step,
        frame_offsets,
        metadata: TrajectoryMetadata {
            title,
            software: "XYZ".to_string(),
            ..Default::default()
        },
    })
}

fn parse_frame_at_offset(
    reader: &mut BufReader<File>,
    offset: u64,
    frame_index: usize,
    num_atoms: usize,
    time_step: f32,
) -> IOResult<FrameData> {
    reader.seek(SeekFrom::Start(offset)).map_err(IOError::Io)?;

    let mut line = String::new();
    reader.read_line(&mut line).map_err(IOError::Io)?;
    let count = line
        .trim()
        .parse::<usize>()
        .map_err(|_| IOError::ParseError {
            line: frame_index,
            message: format!(
                "Expected atom count at offset {offset}, got: {}",
                line.trim()
            ),
        })?;
    if count != num_atoms {
        return Err(IOError::ParseError {
            line: frame_index,
            message: format!("Frame {frame_index} atom count {count} != {num_atoms}"),
        });
    }

    line.clear();
    reader.read_line(&mut line).map_err(IOError::Io)?;

    let mut frame = FrameData::new(frame_index, frame_index as f32 * time_step);
    for atom_i in 0..num_atoms {
        line.clear();
        reader.read_line(&mut line).map_err(IOError::Io)?;

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            return Err(IOError::ParseError {
                line: frame_index,
                message: format!(
                    "Frame {frame_index} atom {atom_i}: expected element X Y Z, got {} fields",
                    parts.len()
                ),
            });
        }

        let x = parts[1].parse::<f32>().map_err(|_| IOError::ParseError {
            line: frame_index,
            message: format!("Invalid X coordinate: {}", parts[1]),
        })?;
        let y = parts[2].parse::<f32>().map_err(|_| IOError::ParseError {
            line: frame_index,
            message: format!("Invalid Y coordinate: {}", parts[2]),
        })?;
        let z = parts[3].parse::<f32>().map_err(|_| IOError::ParseError {
            line: frame_index,
            message: format!("Invalid Z coordinate: {}", parts[3]),
        })?;

        frame.set_position(atom_i as u32, Vec3::new(x, y, z));
    }

    Ok(frame)
}

/// Random-access XYZ frame provider backed by a seekable file handle.
pub struct XyzFrameProvider {
    reader: Arc<Mutex<BufReader<File>>>,
    index: XyzIndex,
    file_path: PathBuf,
}

impl XyzFrameProvider {
    pub fn open(path: &Path) -> IOResult<Self> {
        let index = build_xyz_index(path)?;
        let file =
            File::open(path).map_err(|_| IOError::FileNotFound(path.display().to_string()))?;
        Ok(Self {
            reader: Arc::new(Mutex::new(BufReader::new(file))),
            index,
            file_path: path.to_path_buf(),
        })
    }

    pub fn index(&self) -> &XyzIndex {
        &self.index
    }

    pub fn should_stream(num_atoms: usize, num_frames: usize) -> bool {
        crate::io::streaming::should_stream_trajectory(num_atoms, num_frames)
    }

    pub fn load_all_frames(&self) -> IOResult<Vec<FrameData>> {
        let mut reader = self
            .reader
            .lock()
            .map_err(|_| IOError::InvalidFormat("XYZ reader lock poisoned".to_string()))?;
        let mut frames = Vec::with_capacity(self.index.num_frames);
        for (i, &offset) in self.index.frame_offsets.iter().enumerate() {
            frames.push(parse_frame_at_offset(
                &mut reader,
                offset,
                i,
                self.index.num_atoms,
                self.index.time_step,
            )?);
        }
        Ok(frames)
    }
}

impl FrameProvider for XyzFrameProvider {
    fn num_frames(&self) -> usize {
        self.index.num_frames
    }

    fn num_atoms(&self) -> usize {
        self.index.num_atoms
    }

    fn time_step(&self) -> f32 {
        self.index.time_step
    }

    fn file_path(&self) -> &Path {
        &self.file_path
    }

    fn metadata(&self) -> &TrajectoryMetadata {
        &self.index.metadata
    }

    fn get_frame(&self, index: usize) -> IOResult<FrameData> {
        let offset = *self
            .index
            .frame_offsets
            .get(index)
            .ok_or_else(|| IOError::ParseError {
                line: 0,
                message: format!(
                    "Frame index {index} out of range ({} frames)",
                    self.index.num_frames
                ),
            })?;

        let mut reader = self
            .reader
            .lock()
            .map_err(|_| IOError::InvalidFormat("XYZ reader lock poisoned".to_string()))?;

        parse_frame_at_offset(
            &mut reader,
            offset,
            index,
            self.index.num_atoms,
            self.index.time_step,
        )
    }
}

/// Sequential XYZ frame iterator (legacy streaming API).
pub struct XYZStreamer {
    inner: crate::io::xyz::FrameStream,
}

impl XYZStreamer {
    pub fn new(path: &Path) -> IOResult<Self> {
        Ok(Self {
            inner: crate::io::xyz::XYZParser::stream_frames(path)?,
        })
    }

    pub fn next_frame(&mut self) -> IOResult<Option<FrameData>> {
        self.inner.next_frame()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::xyz::XYZParser;
    use std::io::Write;

    fn write_temp_xyz(path: &Path, frame_count: usize) -> std::io::Result<()> {
        let mut file = File::create(path)?;
        for f in 0..frame_count {
            writeln!(file, "2")?;
            writeln!(file, "time=1.0 frame={f}")?;
            writeln!(file, "C {} 0.0 0.0", f as f32 * 0.1)?;
            writeln!(file, "H 1.0 0.0 0.0")?;
        }
        Ok(())
    }

    #[test]
    fn test_build_xyz_index_offsets() {
        let dir = std::env::temp_dir().join(format!("gumol_xyz_stream_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("traj.xyz");
        write_temp_xyz(&path, 5).unwrap();

        let index = build_xyz_index(&path).unwrap();
        assert_eq!(index.num_atoms, 2);
        assert_eq!(index.num_frames, 5);
        assert_eq!(index.frame_offsets.len(), 5);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_xyz_provider_matches_full_parse() {
        let dir = std::env::temp_dir().join(format!("gumol_xyz_stream_cmp_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("traj.xyz");
        write_temp_xyz(&path, 8).unwrap();

        let full = XYZParser::parse_file_buffered(&path).unwrap();
        let provider = XyzFrameProvider::open(&path).unwrap();

        assert_eq!(provider.num_frames(), full.num_frames());
        assert_eq!(provider.num_atoms(), full.num_atoms);

        for i in 0..full.num_frames() {
            let expected = full.get_frame(i).unwrap();
            let streamed = provider.get_frame(i).unwrap();
            for atom_id in expected.atom_ids() {
                let a = expected.get_position(*atom_id).unwrap();
                let b = streamed.get_position(*atom_id).unwrap();
                assert!(
                    (a - b).length() < 1e-5,
                    "frame {i} atom {atom_id}: {a:?} vs {b:?}"
                );
            }
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_xyz_streamer_sequential() {
        let dir = std::env::temp_dir().join(format!("gumol_xyz_stream_seq_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("traj.xyz");
        write_temp_xyz(&path, 3).unwrap();

        let mut streamer = XYZStreamer::new(&path).unwrap();
        let mut count = 0;
        while streamer.next_frame().unwrap().is_some() {
            count += 1;
        }
        assert_eq!(count, 3);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
