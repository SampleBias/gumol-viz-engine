//! Memory-mapped and parallel XYZ parsing for large trajectories.

use crate::core::trajectory::{FrameData, Trajectory};
use crate::io::xyz::XYZParser;
use crate::io::{IOError, IOResult};
use bevy::prelude::*;
use memmap2::Mmap;
use rayon::prelude::*;
use std::fs::File;
use std::path::{Path, PathBuf};

/// Use mmap when the file is at least this large.
pub const MMAP_THRESHOLD_BYTES: u64 = 512 * 1024;

/// Parallelize frame parsing when there are at least this many frames.
pub const PARALLEL_FRAME_THRESHOLD: usize = 8;

#[derive(Debug, Clone)]
struct FrameSpec {
    first_atom_line: usize,
    frame_index: usize,
}

/// Parse XYZ via memory map (avoids an extra kernel read buffer copy for large files).
pub fn parse_file_mmap(path: &Path) -> IOResult<Trajectory> {
    let file = File::open(path).map_err(|_| IOError::FileNotFound(path.display().to_string()))?;
    let mmap = unsafe { Mmap::map(&file) }.map_err(IOError::Io)?;
    let content = std::str::from_utf8(&mmap)
        .map_err(|_| IOError::InvalidFormat(format!("{} is not valid UTF-8", path.display())))?;
    let lines: Vec<String> = content.lines().map(str::to_string).collect();
    parse_lines_parallel(&lines, path.to_path_buf())
}

fn scan_frame_specs(lines: &[String]) -> IOResult<(usize, f32, Vec<FrameSpec>)> {
    let mut specs = Vec::new();
    let mut num_atoms = 0usize;
    let mut time_step = 1.0f32;
    let mut frame_index = 0usize;
    let mut i = 0usize;

    while i < lines.len() {
        while i < lines.len() && lines[i].trim().is_empty() {
            i += 1;
        }
        if i >= lines.len() {
            break;
        }

        let natoms_line = i;
        let count = lines[i]
            .trim()
            .parse::<usize>()
            .map_err(|_| IOError::ParseError {
                line: i,
                message: format!("Expected atom count, got {}", lines[i]),
            })?;

        if frame_index == 0 {
            num_atoms = count;
        } else if count != num_atoms {
            return Err(IOError::ParseError {
                line: i,
                message: format!(
                    "Atom count changed from {num_atoms} to {count} in frame {frame_index}"
                ),
            });
        }

        i += 1;
        if i >= lines.len() {
            return Err(IOError::ParseError {
                line: natoms_line,
                message: "Unexpected EOF after atom count".into(),
            });
        }

        let comment = lines[i].trim();
        if let Some(time_str) = comment
            .split_whitespace()
            .find(|s| s.starts_with("time=") || s.starts_with("t="))
        {
            if let Some(val) = time_str.split('=').nth(1) {
                if let Ok(t) = val.parse::<f32>() {
                    time_step = t;
                }
            }
        }

        i += 1;
        let first_atom_line = i;
        if i + num_atoms > lines.len() {
            return Err(IOError::ParseError {
                line: i,
                message: "Unexpected EOF while reading atom block".into(),
            });
        }

        specs.push(FrameSpec {
            first_atom_line,
            frame_index,
        });

        i += num_atoms;
        frame_index += 1;
    }

    if specs.is_empty() {
        return Err(IOError::ParseError {
            line: 0,
            message: "No frames found in XYZ data".into(),
        });
    }

    Ok((num_atoms, time_step, specs))
}

fn parse_frame_positions(
    lines: &[String],
    spec: &FrameSpec,
    num_atoms: usize,
    time_step: f32,
) -> Result<FrameData, IOError> {
    let mut frame = FrameData::new(spec.frame_index, spec.frame_index as f32 * time_step);

    for atom_i in 0..num_atoms {
        let line_num = spec.first_atom_line + atom_i;
        let line = lines.get(line_num).ok_or_else(|| IOError::ParseError {
            line: line_num,
            message: "Missing atom line".into(),
        })?;

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            return Err(IOError::ParseError {
                line: line_num,
                message: format!("Expected element X Y Z, got {} fields", parts.len()),
            });
        }

        let x = parts[1].parse::<f32>().map_err(|_| IOError::ParseError {
            line: line_num,
            message: format!("Invalid X: {}", parts[1]),
        })?;
        let y = parts[2].parse::<f32>().map_err(|_| IOError::ParseError {
            line: line_num,
            message: format!("Invalid Y: {}", parts[2]),
        })?;
        let z = parts[3].parse::<f32>().map_err(|_| IOError::ParseError {
            line: line_num,
            message: format!("Invalid Z: {}", parts[3]),
        })?;

        frame.set_position(atom_i as u32, Vec3::new(x, y, z));
    }

    Ok(frame)
}

/// Parse lines, using rayon for multi-frame trajectories when beneficial.
pub fn parse_lines_parallel(lines: &[String], file_path: PathBuf) -> IOResult<Trajectory> {
    let (num_atoms, time_step, specs) = scan_frame_specs(lines)?;

    let frames = if specs.len() >= PARALLEL_FRAME_THRESHOLD {
        specs
            .par_iter()
            .map(|spec| parse_frame_positions(lines, spec, num_atoms, time_step))
            .collect::<Result<Vec<_>, _>>()?
    } else {
        specs
            .iter()
            .map(|spec| parse_frame_positions(lines, spec, num_atoms, time_step))
            .collect::<Result<Vec<_>, _>>()?
    };

    let mut trajectory = Trajectory::new(file_path, num_atoms, time_step);
    for frame in frames {
        trajectory.add_frame(frame);
    }
    Ok(trajectory)
}

/// Auto-select mmap + parallel parsing for large XYZ files.
pub fn parse_file_optimized(path: &Path) -> IOResult<Trajectory> {
    let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    if size >= MMAP_THRESHOLD_BYTES {
        parse_file_mmap(path)
    } else {
        XYZParser::parse_reader(
            File::open(path).map_err(|_| IOError::FileNotFound(path.display().to_string()))?,
            path.to_path_buf(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn multi_frame_xyz(frame_count: usize) -> String {
        let mut out = String::new();
        for f in 0..frame_count {
            out.push_str("2\n");
            out.push_str(&format!("frame {f}\n"));
            out.push_str(&format!("C {} 0.0 0.0\n", f as f32 * 0.1));
            out.push_str("H 1.0 0.0 0.0\n");
        }
        out
    }

    #[test]
    fn test_parallel_parse_matches_sequential() {
        let content = multi_frame_xyz(12);
        let lines: Vec<String> = content.lines().map(str::to_string).collect();
        let path = PathBuf::from("parallel_test.xyz");

        let parallel = parse_lines_parallel(&lines, path.clone()).unwrap();
        let sequential = XYZParser::parse_string(&content, path).unwrap();

        assert_eq!(parallel.num_frames(), sequential.num_frames());
        assert_eq!(parallel.num_atoms, sequential.num_atoms);
        for (a, b) in parallel.frames.iter().zip(sequential.frames.iter()) {
            assert_eq!(a.positions.len(), b.positions.len());
            for id in a.positions.keys() {
                assert_eq!(a.get_position(*id), b.get_position(*id));
            }
        }
    }

    #[test]
    fn test_scan_frame_specs_single_frame() {
        let lines: Vec<String> = "3\nwater\nO 0 0 0\nH 0 0 0\nH 0 0 0"
            .lines()
            .map(str::to_string)
            .collect();
        let (n, _, specs) = scan_frame_specs(&lines).unwrap();
        assert_eq!(n, 3);
        assert_eq!(specs.len(), 1);
    }
}
