//! Memory usage estimation for loaded trajectories.

use crate::systems::loading::SimulationData;

/// Warn when estimated resident memory exceeds this threshold (bytes).
pub const MEMORY_WARN_THRESHOLD_BYTES: u64 = 4 * 1024 * 1024 * 1024;

/// Approximate bytes per atom per frame (Vec3 position + HashMap overhead).
const BYTES_PER_ATOM_PER_FRAME: u64 = 48;

/// Approximate bytes per static atom metadata record.
const BYTES_PER_ATOM_METADATA: u64 = 256;

/// Estimate resident RAM for the loaded simulation.
pub fn estimate_simulation_bytes(sim_data: &SimulationData) -> u64 {
    if !sim_data.loaded {
        return 0;
    }

    let atoms = sim_data.num_atoms() as u64;
    let frames = sim_data.num_frames() as u64;

    // Streaming trajectories keep only metadata + one frame hot in practice.
    let trajectory_bytes = if sim_data.is_streaming() {
        atoms * BYTES_PER_ATOM_PER_FRAME
    } else {
        atoms * frames * BYTES_PER_ATOM_PER_FRAME
    };
    let metadata_bytes = atoms * BYTES_PER_ATOM_METADATA;
    let bond_bytes = sim_data.bond_data.len() as u64 * 64;

    trajectory_bytes + metadata_bytes + bond_bytes
}

/// Human-readable memory size.
pub fn format_bytes(bytes: u64) -> String {
    const GB: f64 = 1024.0 * 1024.0 * 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    const KB: f64 = 1024.0;

    let b = bytes as f64;
    if b >= GB {
        format!("{:.2} GB", b / GB)
    } else if b >= MB {
        format!("{:.2} MB", b / MB)
    } else if b >= KB {
        format!("{:.2} KB", b / KB)
    } else {
        format!("{bytes} B")
    }
}

/// Build a warning message when memory exceeds budget.
pub fn memory_warning(sim_data: &SimulationData) -> Option<String> {
    let bytes = estimate_simulation_bytes(sim_data);
    if bytes > MEMORY_WARN_THRESHOLD_BYTES {
        Some(format!(
            "Estimated memory {} exceeds 4 GB — consider streaming (plan 04) for large trajectories.",
            format_bytes(bytes)
        ))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::Vec3;
    use crate::core::trajectory::Trajectory;
    use std::path::PathBuf;

    #[test]
    fn test_estimate_zero_when_unloaded() {
        let sim = SimulationData::default();
        assert_eq!(estimate_simulation_bytes(&sim), 0);
    }

    #[test]
    fn test_format_bytes() {
        assert!(format_bytes(1024).contains("KB"));
        assert!(format_bytes(5 * 1024 * 1024 * 1024).contains("GB"));
    }

    #[test]
    fn test_memory_warning_threshold() {
        let atom_count = 250_000usize;
        let frame_count = 500usize;
        let sim = SimulationData::new(
            synthetic_trajectory_for_test(atom_count, frame_count),
            (0..atom_count)
                .map(|i| {
                    crate::core::atom::AtomData::new(
                        i as u32,
                        crate::core::atom::Element::C,
                        0,
                        "UNK".into(),
                        "A".into(),
                        "C".into(),
                    )
                })
                .collect(),
        );
        assert!(estimate_simulation_bytes(&sim) > MEMORY_WARN_THRESHOLD_BYTES);
        assert!(memory_warning(&sim).is_some());
    }

    fn synthetic_trajectory_for_test(atom_count: usize, frame_count: usize) -> crate::core::trajectory::Trajectory {
        let mut trajectory =
            crate::core::trajectory::Trajectory::new(PathBuf::from("big.xyz"), atom_count, 1.0);
        for f in 0..frame_count {
            let mut frame = crate::core::trajectory::FrameData::new(f, f as f32);
            for i in 0..atom_count.min(10) {
                frame.set_position(i as u32, Vec3::ZERO);
            }
            trajectory.frames.push(frame);
        }
        trajectory
    }
}
