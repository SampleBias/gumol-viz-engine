//! Bevy ECS systems
//!
//! System ordering:
//!   Startup: load_cli_file
//!   Update:  handle_load_file_events
//!            -> clear instanced/pick/bonds on load
//!            -> instanced spawn (+ pick proxies + index)
//!            -> bond spawn, timeline, position sync
//!            -> visualization + selection highlight

pub mod bonds;
pub mod loading;
pub mod spawning;
pub mod timeline;
pub mod visualization;

use bevy::prelude::*;

/// Register all resources, events, and systems with explicit ordering.
///
/// Production atom rendering uses the instanced GPU pipeline
/// (`rendering::instanced`). Legacy per-atom spawning in `spawning.rs` is kept
/// only for `AtomEntities` compatibility during migration.
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
                loading::poll_async_load,
                loading::handle_load_topology_events,
                loading::track_topology_requirement,
                loading::print_simulation_data,
                timeline::handle_timeline_input,
            ),
            // Group 2: react to file load — clear before spawn
            (
                bonds::clear_spatial_index_on_load,
                crate::rendering::instanced::clear_instanced_atoms_on_load,
                crate::rendering::wireframe::clear_wireframe_on_load,
                crate::rendering::ribbon::clear_ribbon_on_load,
                bonds::clear_bonds_on_load,
                timeline::update_timeline_on_load,
            ),
            // Group 3: spawn instanced atoms, pick proxies, and index
            crate::rendering::instanced::spawn_instanced_atoms_on_load,
            crate::rendering::instanced::center_camera_on_file_load_instanced,
            // Group 4: bonds, wireframe, ribbon after instanced atoms exist
            (
                bonds::build_spatial_index_on_spawn,
                bonds::spawn_bonds,
                crate::rendering::wireframe::spawn_wireframe_bonds,
                crate::rendering::ribbon::build_backbone_on_load,
                crate::rendering::ribbon::spawn_ribbon_on_load,
            ),
            // Group 5: timeline advancement
            timeline::update_timeline,
            // Group 6: position updates
            (
                crate::rendering::instanced::update_instanced_positions_from_timeline,
                crate::interaction::pick_proxy::update_pick_proxy_positions,
                bonds::update_bond_positions,
                crate::rendering::wireframe::update_wireframe_bond_positions,
                crate::rendering::ribbon::update_ribbon_positions,
            ),
            // Group 6b: performance (culling, LOD)
            (
                crate::rendering::culling::cull_instanced_atoms,
                crate::rendering::lod_system::update_instanced_lod_meshes,
            ),
            // Group 7: visualization & selection
            (
                visualization::clamp_unavailable_render_modes,
                visualization::sync_mode_params,
                crate::rendering::instanced::update_instanced_visualization,
                crate::rendering::instanced::update_instanced_selection_highlight,
                visualization::update_bond_visibility,
                visualization::update_bond_scale,
                visualization::update_bond_appearance,
                crate::rendering::wireframe::update_wireframe_visibility,
                crate::rendering::ribbon::update_ribbon_visibility,
                crate::rendering::ribbon::update_ribbon_for_mode,
            ),
        )
            .chain(),
    );

    info!("Systems module registered with instanced rendering pipeline");
}
