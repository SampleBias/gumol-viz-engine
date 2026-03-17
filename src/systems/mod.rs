//! Bevy ECS systems
//!
//! System ordering:
//!   Startup: load_cli_file
//!   Update:  handle_load_file_events
//!            -> instanced spawn, clear, camera center, timeline reset, bond clear
//!            -> update_timeline -> update_instanced_positions_from_timeline
//!            -> spawn_bonds, update_bond_positions
//!            -> visualization updates (atom/bond visibility & scale)

pub mod bonds;
pub mod loading;
pub mod spawning;
pub mod timeline;
pub mod visualization;

use bevy::prelude::*;

/// Register all resources, events, and systems with explicit ordering.
///
/// The per-atom spawning path (`spawning::spawn_atoms_on_load`) is replaced by
/// the instanced path (`rendering::instanced::spawn_instanced_atoms_on_load`)
/// which creates one entity per element instead of one per atom.
pub fn register(app: &mut App) {
    loading::register(app);
    spawning::register(app);
    timeline::register(app);
    bonds::register(app);
    visualization::register(app);

    app.add_systems(Startup, loading::load_cli_file);

    app.add_systems(
        Update,
        (
            // Group 1: file loading & input handling
            (
                loading::handle_load_file_events,
                loading::print_simulation_data,
                timeline::handle_timeline_input,
            ),
            // Group 2: react to file load
            (
                crate::rendering::instanced::spawn_instanced_atoms_on_load,
                crate::rendering::instanced::clear_instanced_atoms_on_load,
                crate::rendering::instanced::center_camera_on_file_load_instanced,
                timeline::update_timeline_on_load,
                bonds::clear_bonds_on_load,
            ),
            // Group 3: timeline advancement
            timeline::update_timeline,
            // Group 4: position updates (instanced path)
            crate::rendering::instanced::update_instanced_positions_from_timeline,
            // Group 5: bond detection and sync (no-op until per-atom entities exist)
            (bonds::spawn_bonds, bonds::update_bond_positions),
            // Group 6: visualization updates
            (
                visualization::update_atom_visibility,
                visualization::update_bond_visibility,
                visualization::update_atom_scale,
                visualization::update_bond_scale,
            ),
        )
            .chain(),
    );

    info!("Systems module registered with instanced rendering pipeline");
}
