//! Timeline and trajectory management

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Resource containing timeline state
#[derive(Resource, Clone, Debug)]
pub struct TimelineState {
    /// Current frame index
    pub current_frame: usize,
    /// Total number of frames
    pub total_frames: usize,
    /// Is the timeline playing?
    pub is_playing: bool,
    /// Playback speed multiplier (1.0 = normal)
    pub playback_speed: f32,
    /// Loop playback?
    pub loop_playback: bool,
    /// Interpolate between frames for smooth animation?
    pub interpolate: bool,
    /// Current interpolation factor (0.0 to 1.0)
    pub interpolation_factor: f32,
    /// Time accumulator for frame timing
    pub time_accumulator: f32,
}

impl Default for TimelineState {
    fn default() -> Self {
        Self {
            current_frame: 0,
            total_frames: 1,
            is_playing: false,
            playback_speed: 1.0,
            loop_playback: true,
            interpolate: true,
            interpolation_factor: 0.0,
            time_accumulator: 0.0,
        }
    }
}

impl TimelineState {
    /// Create a new timeline state
    pub fn new(total_frames: usize) -> Self {
        Self {
            total_frames,
            ..default()
        }
    }

    /// Advance to the next frame
    pub fn next_frame(&mut self) {
        self.current_frame = (self.current_frame + 1).min(self.total_frames - 1);
        self.interpolation_factor = 0.0;
    }

    /// Go to the previous frame
    pub fn previous_frame(&mut self) {
        self.current_frame = self.current_frame.saturating_sub(1);
        self.interpolation_factor = 0.0;
    }

    /// Go to a specific frame
    pub fn goto_frame(&mut self, frame: usize) {
        self.current_frame = frame.min(self.total_frames - 1);
        self.interpolation_factor = 0.0;
    }

    /// Toggle playback
    pub fn toggle_playback(&mut self) {
        self.is_playing = !self.is_playing;
    }

    /// Start playback
    pub fn play(&mut self) {
        self.is_playing = true;
    }

    /// Pause playback
    pub fn pause(&mut self) {
        self.is_playing = false;
    }

    /// Stop playback and reset to frame 0
    pub fn stop(&mut self) {
        self.is_playing = false;
        self.goto_frame(0);
    }

    /// Get progress as a percentage (0.0 to 1.0)
    pub fn progress(&self) -> f32 {
        if self.total_frames == 0 {
            0.0
        } else {
            self.current_frame as f32 / (self.total_frames - 1) as f32
        }
    }

    /// Get the current simulation time (in femtoseconds)
    pub fn simulation_time(&self, time_step: f32) -> f32 {
        self.current_frame as f32 * time_step
    }
}

/// Data for a single trajectory frame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameData {
    /// Frame index
    pub index: usize,
    /// Atom positions (atom ID -> position)
    pub positions: HashMap<u32, Vec3>,
    /// Atom velocities (optional, atom ID -> velocity)
    pub velocities: Option<HashMap<u32, Vec3>>,
    /// Atom forces (optional, atom ID -> force)
    pub forces: Option<HashMap<u32, Vec3>>,
    /// Box dimensions for periodic systems (optional)
    pub box_size: Option<[f32; 3]>,
    /// Time of this frame (in femtoseconds)
    pub time: f32,
    /// Potential energy (optional)
    pub potential_energy: Option<f32>,
    /// Kinetic energy (optional)
    pub kinetic_energy: Option<f32>,
    /// Temperature (optional, in Kelvin)
    pub temperature: Option<f32>,
    /// Pressure (optional, in bar)
    pub pressure: Option<f32>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl FrameData {
    /// Create a new empty frame
    pub fn new(index: usize, time: f32) -> Self {
        Self {
            index,
            time,
            positions: HashMap::new(),
            velocities: None,
            forces: None,
            box_size: None,
            potential_energy: None,
            kinetic_energy: None,
            temperature: None,
            pressure: None,
            metadata: HashMap::new(),
        }
    }

    /// Set the position of an atom
    pub fn set_position(&mut self, atom_id: u32, position: Vec3) {
        self.positions.insert(atom_id, position);
    }

    /// Get the position of an atom
    pub fn get_position(&self, atom_id: u32) -> Option<Vec3> {
        self.positions.get(&atom_id).copied()
    }

    /// Get all atom IDs in this frame
    pub fn atom_ids(&self) -> impl Iterator<Item = &u32> {
        self.positions.keys()
    }
}

/// Complete trajectory data
#[derive(Debug, Clone)]
pub struct Trajectory {
    /// File path of the trajectory
    pub file_path: PathBuf,
    /// All frames
    pub frames: Vec<FrameData>,
    /// Number of atoms
    pub num_atoms: usize,
    /// Time step in femtoseconds
    pub time_step: f32,
    /// Total simulation time in femtoseconds
    pub total_time: f32,
    /// Trajectory metadata
    pub metadata: TrajectoryMetadata,
}

impl Trajectory {
    /// Create a new trajectory
    pub fn new(file_path: PathBuf, num_atoms: usize, time_step: f32) -> Self {
        Self {
            file_path,
            frames: Vec::new(),
            num_atoms,
            time_step,
            total_time: 0.0,
            metadata: TrajectoryMetadata::default(),
        }
    }

    /// Add a frame to the trajectory
    pub fn add_frame(&mut self, frame: FrameData) {
        self.total_time = frame.time;
        self.frames.push(frame);
    }

    /// Get a specific frame
    pub fn get_frame(&self, index: usize) -> Option<&FrameData> {
        self.frames.get(index)
    }

    /// Get the number of frames
    pub fn num_frames(&self) -> usize {
        self.frames.len()
    }

    /// Check if the trajectory is empty
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }
}

/// Metadata about the trajectory
#[derive(Debug, Clone, Default)]
pub struct TrajectoryMetadata {
    /// Title or description
    pub title: String,
    /// Classification
    pub classification: String,
    /// Simulation software used
    pub software: String,
    /// Force field used
    pub force_field: String,
    /// Ensemble (NVT, NPT, etc.)
    pub ensemble: String,
    /// Temperature (in Kelvin)
    pub temperature: Option<f32>,
    /// Pressure (in bar)
    pub pressure: Option<f32>,
    /// Number of steps
    pub num_steps: Option<u64>,
    /// Step size (in femtoseconds)
    pub step_size: Option<f32>,
    /// Creation date
    pub creation_date: Option<String>,
    /// Additional metadata
    pub extra: HashMap<String, String>,
}

/// Interpolate between two frames
pub fn interpolate_frames(frame_a: &FrameData, frame_b: &FrameData, alpha: f32) -> FrameData {
    let mut interpolated = FrameData::new(
        frame_a.index,
        frame_a.time + (frame_b.time - frame_a.time) * alpha,
    );

    // Interpolate positions
    for (atom_id, pos_a) in &frame_a.positions {
        if let Some(pos_b) = frame_b.positions.get(atom_id) {
            let pos = pos_a.lerp(*pos_b, alpha);
            interpolated.set_position(*atom_id, pos);
        }
    }

    // Interpolate velocities if available
    if let (Some(vel_a), Some(vel_b)) = (&frame_a.velocities, &frame_b.velocities) {
        let mut velocities = HashMap::new();
        for (atom_id, vel) in vel_a {
            if let Some(vel_b) = vel_b.get(atom_id) {
                velocities.insert(*atom_id, vel.lerp(*vel_b, alpha));
            }
        }
        interpolated.velocities = Some(velocities);
    }

    // Interpolate energies if available
    if let (Some(pe_a), Some(pe_b)) = (frame_a.potential_energy, frame_b.potential_energy) {
        interpolated.potential_energy = Some(pe_a + (pe_b - pe_a) * alpha);
    }
    if let (Some(ke_a), Some(ke_b)) = (frame_a.kinetic_energy, frame_b.kinetic_energy) {
        interpolated.kinetic_energy = Some(ke_a + (ke_b - ke_a) * alpha);
    }

    interpolated
}

/// Calculate RMSD between two frames
pub fn calculate_rmsd(frame_a: &FrameData, frame_b: &FrameData, atom_ids: &[u32]) -> Option<f32> {
    if atom_ids.is_empty() {
        return None;
    }

    let mut sum_sq_diff = 0.0;
    let mut count = 0;

    for atom_id in atom_ids {
        if let (Some(pos_a), Some(pos_b)) = (frame_a.get_position(*atom_id), frame_b.get_position(*atom_id)) {
            let diff = pos_a - pos_b;
            sum_sq_diff += diff.length_squared();
            count += 1;
        }
    }

    if count == 0 {
        None
    } else {
        Some((sum_sq_diff / count as f32).sqrt())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_data() {
        let mut frame = FrameData::new(0, 0.0);
        frame.set_position(0, Vec3::new(1.0, 0.0, 0.0));
        frame.set_position(1, Vec3::new(0.0, 1.0, 0.0));

        assert_eq!(frame.index, 0);
        assert_eq!(frame.get_position(0), Some(Vec3::new(1.0, 0.0, 0.0)));
    }

    #[test]
    fn test_interpolation() {
        let mut frame_a = FrameData::new(0, 0.0);
        frame_a.set_position(0, Vec3::new(0.0, 0.0, 0.0));

        let mut frame_b = FrameData::new(1, 1.0);
        frame_b.set_position(0, Vec3::new(1.0, 0.0, 0.0));

        let interpolated = interpolate_frames(&frame_a, &frame_b, 0.5);
        assert_eq!(interpolated.index, 0);
        assert_eq!(interpolated.time, 0.5);
        assert_eq!(
            interpolated.get_position(0),
            Some(Vec3::new(0.5, 0.0, 0.0))
        );
    }

    #[test]
    fn test_rmsd() {
        let mut frame_a = FrameData::new(0, 0.0);
        frame_a.set_position(0, Vec3::new(0.0, 0.0, 0.0));

        let mut frame_b = FrameData::new(1, 1.0);
        frame_b.set_position(0, Vec3::new(1.0, 0.0, 0.0));

        let rmsd = calculate_rmsd(&frame_a, &frame_b, &[0]);
        assert!(rmsd.is_some());
        assert!((rmsd.unwrap() - 1.0).abs() < 0.001);
    }
}
