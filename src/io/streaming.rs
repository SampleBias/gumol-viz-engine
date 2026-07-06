//! On-demand frame loading for large trajectories.

use crate::core::trajectory::{FrameData, Trajectory, TrajectoryMetadata};
use crate::io::dcd::DcdReader;
use crate::io::{IOError, IOResult};
use bevy::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Minimum atom×frame product before DCD uses streaming instead of full RAM load.
pub const STREAMING_ATOM_FRAMES_THRESHOLD: u64 = 1_000_000;

/// Provides random access to trajectory frames without holding all frames in memory.
pub trait FrameProvider: Send + Sync {
    fn num_frames(&self) -> usize;
    fn num_atoms(&self) -> usize;
    fn time_step(&self) -> f32;
    fn file_path(&self) -> &Path;
    fn metadata(&self) -> &TrajectoryMetadata;
    fn get_frame(&self, index: usize) -> IOResult<FrameData>;
}

/// In-memory frame storage (default for small trajectories).
#[derive(Clone)]
pub struct InMemoryFrameProvider {
    trajectory: Trajectory,
}

impl InMemoryFrameProvider {
    pub fn new(trajectory: Trajectory) -> Self {
        Self { trajectory }
    }

    pub fn into_trajectory(self) -> Trajectory {
        self.trajectory
    }
}

impl FrameProvider for InMemoryFrameProvider {
    fn num_frames(&self) -> usize {
        self.trajectory.num_frames()
    }

    fn num_atoms(&self) -> usize {
        self.trajectory.num_atoms
    }

    fn time_step(&self) -> f32 {
        self.trajectory.time_step
    }

    fn file_path(&self) -> &Path {
        &self.trajectory.file_path
    }

    fn metadata(&self) -> &TrajectoryMetadata {
        &self.trajectory.metadata
    }

    fn get_frame(&self, index: usize) -> IOResult<FrameData> {
        self.trajectory
            .get_frame(index)
            .cloned()
            .ok_or_else(|| IOError::ParseError {
                line: 0,
                message: format!("Frame index {index} out of range"),
            })
    }
}

/// Memory-mapped / seek-based DCD reader for large binary trajectories.
pub struct DcdFrameProvider {
    reader: Arc<Mutex<DcdReader>>,
    num_frames: usize,
    num_atoms: usize,
    time_step: f32,
    file_path: PathBuf,
    metadata: TrajectoryMetadata,
}

impl DcdFrameProvider {
    pub fn open(path: &Path) -> IOResult<Self> {
        let reader = DcdReader::open(path)?;
        let header = reader.header().clone();
        let num_atoms = reader.num_atoms();
        let num_frames = reader.num_frames();
        let time_step = reader.time_step();

        let mut metadata = TrajectoryMetadata::default();
        metadata.title = header.title.clone();
        metadata.software = if header.charmm {
            "CHARMM".to_string()
        } else {
            "NAMD/CHARMM".to_string()
        };
        metadata.num_steps = Some(header.num_sets as u64);
        metadata.step_size = Some(header.delta);

        Ok(Self {
            reader: Arc::new(Mutex::new(reader)),
            num_frames,
            num_atoms,
            time_step,
            file_path: path.to_path_buf(),
            metadata,
        })
    }

    /// Load every frame into memory (for small DCD files).
    pub fn load_all_frames(&self) -> IOResult<Vec<FrameData>> {
        let reader = self.reader.lock().map_err(|_| {
            IOError::InvalidFormat("DCD reader lock poisoned".to_string())
        })?;
        let mut frames = Vec::with_capacity(self.num_frames);
        for i in 0..self.num_frames {
            frames.push(reader.read_frame(i)?);
        }
        Ok(frames)
    }

    pub fn should_stream(num_atoms: usize, num_frames: usize) -> bool {
        (num_atoms as u64).saturating_mul(num_frames as u64) >= STREAMING_ATOM_FRAMES_THRESHOLD
    }
}

impl FrameProvider for DcdFrameProvider {
    fn num_frames(&self) -> usize {
        self.num_frames
    }

    fn num_atoms(&self) -> usize {
        self.num_atoms
    }

    fn time_step(&self) -> f32 {
        self.time_step
    }

    fn file_path(&self) -> &Path {
        &self.file_path
    }

    fn metadata(&self) -> &TrajectoryMetadata {
        &self.metadata
    }

    fn get_frame(&self, index: usize) -> IOResult<FrameData> {
        if index >= self.num_frames {
            return Err(IOError::ParseError {
                line: 0,
                message: format!("Frame index {index} out of range ({} frames)", self.num_frames),
            });
        }
        let reader = self.reader.lock().map_err(|_| IOError::InvalidFormat(
            "DCD reader lock poisoned".to_string(),
        ))?;
        reader.read_frame(index)
    }
}

/// Build the appropriate frame provider for a loaded trajectory.
pub fn frame_provider_from_trajectory(trajectory: Trajectory) -> Arc<dyn FrameProvider> {
    Arc::new(InMemoryFrameProvider::new(trajectory))
}

/// Open a DCD file, streaming when large enough to exceed memory budget.
pub fn open_dcd(path: &Path) -> IOResult<(Trajectory, Option<Arc<dyn FrameProvider>>)> {
    let provider = DcdFrameProvider::open(path)?;
    let num_atoms = provider.num_atoms();
    let num_frames = provider.num_frames();
    let time_step = provider.time_step();
    let metadata = provider.metadata().clone();

    if DcdFrameProvider::should_stream(num_atoms, num_frames) {
        info!(
            "DCD streaming enabled: {} frames × {} atoms (threshold {})",
            num_frames,
            num_atoms,
            STREAMING_ATOM_FRAMES_THRESHOLD
        );
        let mut trajectory = Trajectory::new(path.to_path_buf(), num_atoms, time_step);
        trajectory.metadata = metadata;
        Ok((trajectory, Some(Arc::new(provider))))
    } else {
        let frames = provider.load_all_frames()?;
        let mut trajectory = Trajectory::new(path.to_path_buf(), num_atoms, time_step);
        trajectory.metadata = metadata;
        for frame in frames {
            trajectory.add_frame(frame);
        }
        Ok((trajectory, None))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_threshold() {
        assert!(!DcdFrameProvider::should_stream(100, 100));
        assert!(DcdFrameProvider::should_stream(10_000, 200));
    }
}
