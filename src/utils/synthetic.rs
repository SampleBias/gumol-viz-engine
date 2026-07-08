//! Synthetic trajectory data for benchmarks and performance validation.

use crate::core::atom::{AtomData, Element};
use crate::core::trajectory::{FrameData, Trajectory};
use bevy::prelude::*;
use std::collections::HashMap;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Element assignment matching the criterion benchmark mix (C/H/O).
pub fn element_for_index(index: usize) -> Element {
    match index % 3 {
        0 => Element::C,
        1 => Element::H,
        _ => Element::O,
    }
}

/// Position for atom `index` at trajectory frame `frame`.
pub fn position_for_atom(index: usize, frame: usize) -> Vec3 {
    let i = index as f32;
    let f = frame as f32;
    Vec3::new(
        (i * 1.5).sin() * 10.0 + f * 0.01,
        (i * 0.7).cos() * 10.0 + f * 0.005,
        (i * 0.3).sin() * 10.0,
    )
}

/// Build synthetic atom metadata for `count` atoms.
pub fn synthetic_atom_data(count: usize) -> Vec<AtomData> {
    (0..count)
        .map(|i| {
            AtomData::new(
                i as u32,
                element_for_index(i),
                (i / 10) as u32,
                "UNK".into(),
                "A".into(),
                format!("A{i}"),
            )
        })
        .collect()
}

/// Build a position map for `count` atoms at frame 0.
pub fn synthetic_positions(count: usize) -> HashMap<u32, Vec3> {
    (0..count)
        .map(|i| (i as u32, position_for_atom(i, 0)))
        .collect()
}

/// Build an in-memory trajectory for benchmarks.
pub fn synthetic_trajectory(atom_count: usize, frame_count: usize) -> Trajectory {
    let mut trajectory = Trajectory::new(PathBuf::from("synthetic.xyz"), atom_count, 1.0);
    for frame in 0..frame_count {
        let mut frame_data = FrameData::new(frame, frame as f32);
        for atom in 0..atom_count {
            frame_data.set_position(atom as u32, position_for_atom(atom, frame));
        }
        trajectory.frames.push(frame_data);
    }
    trajectory
}

/// Write a multi-frame XYZ file with the standard benchmark layout.
pub fn write_synthetic_xyz(path: &Path, atom_count: usize, frame_count: usize) -> io::Result<()> {
    let mut file = std::fs::File::create(path)?;
    for frame in 0..frame_count {
        writeln!(file, "{atom_count}")?;
        writeln!(file, "synthetic {atom_count} atoms frame {frame}")?;
        for atom in 0..atom_count {
            let element = element_for_index(atom).symbol();
            let pos = position_for_atom(atom, frame);
            writeln!(file, "{element} {:.6} {:.6} {:.6}", pos.x, pos.y, pos.z)?;
        }
    }
    Ok(())
}

/// Default path for the 100K static profiling fixture.
pub const SYNTHETIC_100K_XYZ: &str = "tests/fixtures/synthetic_100k.xyz";

/// Default path for the 100K playback profiling fixture (10 frames).
pub const SYNTHETIC_100K_PLAYBACK_XYZ: &str = "tests/fixtures/synthetic_100k_10f.xyz";

/// Ensure the standard 100K profiling fixtures exist on disk.
pub fn ensure_100k_fixtures() -> io::Result<()> {
    let static_path = Path::new(SYNTHETIC_100K_XYZ);
    if !static_path.exists() {
        if let Some(parent) = static_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        write_synthetic_xyz(static_path, 100_000, 1)?;
    }

    let playback_path = Path::new(SYNTHETIC_100K_PLAYBACK_XYZ);
    if !playback_path.exists() {
        if let Some(parent) = playback_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        write_synthetic_xyz(playback_path, 100_000, 10)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::xyz::XYZParser;

    #[test]
    fn synthetic_atom_data_has_expected_count() {
        assert_eq!(synthetic_atom_data(100).len(), 100);
    }

    #[test]
    fn write_and_parse_synthetic_xyz_roundtrip() {
        let dir = std::env::temp_dir().join("gumol_synthetic_test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("mini.xyz");
        write_synthetic_xyz(&path, 12, 2).expect("write xyz");

        let trajectory = XYZParser::parse_file(&path).expect("parse xyz");
        assert_eq!(trajectory.num_atoms, 12);
        assert_eq!(trajectory.num_frames(), 2);

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }
}
