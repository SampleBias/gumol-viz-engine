//! Timeline and animation system
//!
//! This system handles trajectory playback, frame advancement, and
//! smooth animation through interpolation.

use crate::core::trajectory::{FrameData, TimelineState, interpolate_frames};
use bevy::prelude::*;

/// Target frames per second for playback
pub const TARGET_FPS: f32 = 60.0;

/// Minimum frame time for smooth animation
pub const MIN_FRAME_TIME: f32 = 1.0 / 30.0; // Max 30 FPS minimum

/// Update timeline state based on playback status
pub fn update_timeline(
    time: Res<Time>,
    sim_data: Res<crate::systems::loading::SimulationData>,
    mut timeline: ResMut<TimelineState>,
) {
    if !timeline.is_playing {
        return;
    }

    if !sim_data.loaded || sim_data.num_frames() == 0 {
        return;
    }

    // Update total frames in case it changed
    timeline.total_frames = sim_data.num_frames();

    // Calculate time delta for this frame
    let delta_time = time.delta_seconds();

    // Accumulate time
    timeline.time_accumulator += delta_time * timeline.playback_speed;

    // Determine if we need to advance to the next frame
    // We assume 1 frame per second of simulation time by default
    // This can be adjusted based on the actual time step
    let frame_duration = if sim_data.num_frames() > 1 {
        sim_data.trajectory.total_time / (sim_data.num_frames() - 1) as f32
    } else {
        1.0
    };

    // Advance frames based on accumulated time
    while timeline.time_accumulator >= frame_duration {
        timeline.time_accumulator -= frame_duration;

        // Advance frame
        timeline.current_frame += 1;

        // Handle looping or stopping
        if timeline.current_frame >= timeline.total_frames {
            if timeline.loop_playback {
                timeline.current_frame = 0;
            } else {
                timeline.current_frame = timeline.total_frames - 1;
                timeline.pause();
            }
        }
    }

    // Calculate interpolation factor
    if timeline.interpolate {
        timeline.interpolation_factor = timeline.time_accumulator / frame_duration;
        timeline.interpolation_factor = timeline.interpolation_factor.clamp(0.0, 1.0);
    } else {
        timeline.interpolation_factor = 0.0;
    }
}

/// Update atom positions based on current timeline frame
pub fn update_atom_positions_from_timeline(
    sim_data: Res<crate::systems::loading::SimulationData>,
    timeline: Res<TimelineState>,
    mut atom_query: Query<(&crate::systems::spawning::SpawnedAtom, &mut Transform)>,
) {
    if !sim_data.loaded || sim_data.num_frames() == 0 {
        return;
    }

    let current_frame = timeline.current_frame;

    // Get current frame
    let current_frame_data = match sim_data.trajectory.get_frame(current_frame) {
        Some(frame) => frame,
        None => return,
    };

    // If interpolating and not at the last frame, get the next frame
    let next_frame_data = if timeline.interpolate && timeline.interpolation_factor > 0.0 {
        let next_frame = (current_frame + 1).min(sim_data.num_frames() - 1);
        sim_data.trajectory.get_frame(next_frame)
    } else {
        None
    };

    // Update positions
    for (spawned_atom, mut transform) in atom_query.iter_mut() {
        let atom_id = spawned_atom.atom_id;

        let position = if let (Some(current), Some(next), Some(alpha)) = (
            current_frame_data.get_position(atom_id),
            next_frame_data.and_then(|f| f.get_position(atom_id)),
            Some(timeline.interpolation_factor).filter(|_| timeline.interpolate),
        ) {
            // Interpolate between frames
            current.lerp(next, alpha)
        } else {
            // No interpolation, use current frame
            match current_frame_data.get_position(atom_id) {
                Some(pos) => pos,
                None => continue,
            }
        };

        transform.translation = position;
    }
}

/// Handle keyboard input for timeline control
pub fn handle_timeline_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut timeline: ResMut<TimelineState>,
    sim_data: Res<crate::systems::loading::SimulationData>,
) {
    if !sim_data.loaded || sim_data.num_frames() == 0 {
        return;
    }

    // Space: Toggle play/pause
    if keyboard.just_pressed(KeyCode::Space) {
        timeline.toggle_playback();
        info!(
            "Timeline {}",
            if timeline.is_playing { "playing" } else { "paused" }
        );
    }

    // Left arrow: Previous frame
    if keyboard.just_pressed(KeyCode::ArrowLeft) {
        timeline.pause();
        timeline.previous_frame();
        info!("Timeline: Frame {}", timeline.current_frame);
    }

    // Right arrow: Next frame
    if keyboard.just_pressed(KeyCode::ArrowRight) {
        timeline.pause();
        timeline.next_frame();
        info!("Timeline: Frame {}", timeline.current_frame);
    }

    // Home: Go to first frame
    if keyboard.just_pressed(KeyCode::Home) {
        timeline.goto_frame(0);
        info!("Timeline: Frame 0 (start)");
    }

    // End: Go to last frame
    if keyboard.just_pressed(KeyCode::End) {
        let last_frame = sim_data.num_frames().saturating_sub(1);
        timeline.goto_frame(last_frame);
        info!("Timeline: Frame {} (end)", last_frame);
    }

    // Up arrow: Increase playback speed
    if keyboard.just_pressed(KeyCode::ArrowUp) {
        timeline.playback_speed = (timeline.playback_speed * 1.5).min(10.0);
        info!("Playback speed: {:.2}x", timeline.playback_speed);
    }

    // Down arrow: Decrease playback speed
    if keyboard.just_pressed(KeyCode::ArrowDown) {
        timeline.playback_speed = (timeline.playback_speed / 1.5).max(0.1);
        info!("Playback speed: {:.2}x", timeline.playback_speed);
    }

    // L: Toggle loop
    if keyboard.just_pressed(KeyCode::KeyL) {
        timeline.loop_playback = !timeline.loop_playback;
        info!("Loop playback: {}", timeline.loop_playback);
    }

    // I: Toggle interpolation
    if keyboard.just_pressed(KeyCode::KeyI) {
        timeline.interpolate = !timeline.interpolate;
        info!("Interpolation: {}", timeline.interpolate);
    }
}

/// Update timeline when file is loaded
pub fn update_timeline_on_load(
    mut timeline: ResMut<TimelineState>,
    mut file_loaded_events: EventReader<crate::systems::loading::FileLoadedEvent>,
) {
    for event in file_loaded_events.read() {
        timeline.total_frames = event.num_frames;
        timeline.current_frame = 0;
        timeline.is_playing = false;
        timeline.interpolation_factor = 0.0;
        timeline.time_accumulator = 0.0;
        info!(
            "Timeline updated: {} frames loaded",
            event.num_frames
        );
    }
}

/// Event sent when timeline playback starts
#[derive(Event, Debug)]
pub struct PlaybackStartedEvent;

/// Event sent when timeline playback stops/pauses
#[derive(Event, Debug)]
pub struct PlaybackStoppedEvent;

/// Event sent when frame changes
#[derive(Event, Debug)]
pub struct FrameChangedEvent {
    pub new_frame: usize,
}

/// Register all timeline systems
pub fn register(app: &mut App) {
    app.init_resource::<TimelineState>()
        .add_event::<PlaybackStartedEvent>()
        .add_event::<PlaybackStoppedEvent>()
        .add_event::<FrameChangedEvent>()
        .add_systems(Update, update_timeline)
        .add_systems(Update, update_atom_positions_from_timeline)
        .add_systems(Update, handle_timeline_input)
        .add_systems(Update, update_timeline_on_load);

    info!("Timeline systems registered");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert!(TARGET_FPS > 0.0);
        assert!(MIN_FRAME_TIME > 0.0);
    }
}
