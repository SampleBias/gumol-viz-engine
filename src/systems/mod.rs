//! Bevy ECS systems
//!
//! System ordering:
//!   Startup: load_cli_file
//!   Update:  handle_load_file_events
//!            -> spawn_atoms_on_load, center_camera, update_timeline_on_load
//!            -> update_timeline -> update_atom_positions_from_timeline
//!            -> spawn_bonds, update_bond_positions
//!            -> visualization updates (atom/bond visibility & scale)

pub mod bonds;
pub mod loading;
pub mod spawning;
pub mod timeline;
pub mod visualization;

use bevy::prelude::*;

/// Register all resources, events, and systems with explicit ordering
pub fn register(app: &mut App) {
    // Step 1: register resources and events from each module
    loading::register(app);
    spawning::register(app);
    timeline::register(app);
    bonds::register(app);
    visualization::register(app);

    // Step 2: register systems with explicit ordering

    // Startup
    app.add_systems(Startup, loading::load_cli_file);

    // Update systems in ordered groups
    app.add_systems(
        Update,
        (
            // Group 1: file loading & input handling (independent, can run in parallel)
            (
                loading::handle_load_file_events,
                loading::print_simulation_data,
                timeline::handle_timeline_input,
            ),
            // Group 2: react to file load (spawning, timeline reset, camera centering)
            (
                spawning::spawn_atoms_on_load,
                spawning::center_camera_on_file_load,
                timeline::update_timeline_on_load,
                bonds::clear_bonds_on_load,
            ),
            // Group 3: timeline advancement
            timeline::update_timeline,
            // Group 4: position updates from timeline
            timeline::update_atom_positions_from_timeline,
            // Group 5: bond detection and position sync (depends on atom positions)
            (bonds::spawn_bonds, bonds::update_bond_positions),
            // Group 6: visualization updates (depends on all entities existing)
            (
                visualization::update_atom_visibility,
                visualization::update_bond_visibility,
                visualization::update_atom_scale,
                visualization::update_bond_scale,
            ),
        )
            .chain(),
    );

    info!("Systems module registered with ordered pipeline");
}
