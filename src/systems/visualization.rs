//! Visualization mode switching system
//!
//! This system manages switching between different rendering modes for atoms and bonds.
//! It uses the RenderMode enum from core::visualization and applies global settings.
//! Systems only run when VisualizationConfig has changed to avoid per-frame iteration.

use crate::core::bond::Bond;
use crate::core::visualization::{RenderMode, VisualizationConfig};
use bevy::prelude::*;

/// Event sent when visualization mode changes
#[derive(Event, Debug)]
pub struct VisualizationModeChangedEvent {
    pub old_mode: RenderMode,
    pub new_mode: RenderMode,
}

/// Update atom visibility based on config (only when config changes)
pub fn update_atom_visibility(
    config: Res<VisualizationConfig>,
    mut atom_query: Query<(&mut Visibility, &crate::systems::spawning::SpawnedAtom)>,
) {
    if !config.is_changed() {
        return;
    }

    let should_show = config.show_atoms && config.render_mode.shows_atoms();

    for (mut visibility, _spawned_atom) in atom_query.iter_mut() {
        *visibility = if should_show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

/// Update bond visibility based on config (only when config changes).
/// Filters on `With<Bond>` so only bond entities are affected.
pub fn update_bond_visibility(
    config: Res<VisualizationConfig>,
    mut bond_query: Query<&mut Visibility, With<Bond>>,
) {
    if !config.is_changed() {
        return;
    }

    let should_show = config.show_bonds && config.render_mode.shows_bonds();

    for mut visibility in bond_query.iter_mut() {
        *visibility = if should_show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

/// Update atom scale based on render mode and config (only when config changes).
/// The base mesh is generated at 50% VDW radius, so `overall_scale` is applied uniformly.
pub fn update_atom_scale(
    config: Res<VisualizationConfig>,
    mut atom_query: Query<&mut Transform, With<crate::systems::spawning::SpawnedAtom>>,
) {
    if !config.is_changed() {
        return;
    }

    let mode_scale = config.render_mode.atom_scale();
    let overall_scale = mode_scale * config.atom_scale;

    for mut transform in atom_query.iter_mut() {
        transform.scale = Vec3::splat(overall_scale);
    }
}

/// Update bond thickness based on render mode and config (only when config changes)
pub fn update_bond_scale(
    config: Res<VisualizationConfig>,
    mut bond_query: Query<&mut Transform, With<Bond>>,
) {
    if !config.is_changed() {
        return;
    }

    let mode_thickness = config.render_mode.bond_thickness();
    let overall_thickness = mode_thickness * config.bond_scale;

    for mut transform in bond_query.iter_mut() {
        transform.scale.x = overall_thickness;
        transform.scale.y = overall_thickness;
    }
}

/// Cycle through visualization modes
pub fn cycle_render_mode(
    mut config: ResMut<VisualizationConfig>,
    mut mode_events: EventWriter<VisualizationModeChangedEvent>,
) {
    let modes = RenderMode::ALL;

    let current_idx = modes
        .iter()
        .position(|&m| m == config.render_mode)
        .unwrap_or(0);
    let next_idx = (current_idx + 1) % modes.len();
    let next_mode = modes[next_idx];

    let old_mode = config.render_mode;
    config.render_mode = next_mode;

    mode_events.send(VisualizationModeChangedEvent {
        old_mode,
        new_mode: next_mode,
    });

    info!("Cycled visualization mode: {:?} -> {:?}", old_mode, next_mode);
}

/// Register visualization resources and events. Systems are registered centrally in systems::register.
pub fn register(app: &mut App) {
    app.init_resource::<VisualizationConfig>()
        .add_event::<VisualizationModeChangedEvent>();

    info!("Visualization resources registered");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_mode_cycle_wraps_around() {
        let modes = RenderMode::ALL;
        let last_mode = modes[modes.len() - 1];
        let first_mode = modes[0];

        let idx = modes.iter().position(|&m| m == last_mode).unwrap();
        let next_idx = (idx + 1) % modes.len();
        assert_eq!(modes[next_idx], first_mode);
    }

    #[test]
    fn test_render_mode_all_modes_reachable() {
        let modes = RenderMode::ALL;
        assert!(modes.len() >= 9);

        for window in modes.windows(2) {
            assert_ne!(window[0], window[1], "Adjacent modes must be different");
        }
    }

    #[test]
    fn test_overall_scale_computation() {
        let config = VisualizationConfig::default();
        let mode_scale = config.render_mode.atom_scale();
        let overall = mode_scale * config.atom_scale;
        assert_eq!(overall, 1.0, "CPK mode at 1.0 atom_scale should give 1.0");

        let ball_stick_scale = RenderMode::BallAndStick.atom_scale();
        assert!(
            ball_stick_scale < RenderMode::CPK.atom_scale(),
            "Ball-and-stick atoms should be smaller than CPK"
        );
    }

    #[test]
    fn test_visibility_logic() {
        assert!(RenderMode::CPK.shows_atoms());
        assert!(!RenderMode::CPK.shows_bonds());

        assert!(RenderMode::BallAndStick.shows_atoms());
        assert!(RenderMode::BallAndStick.shows_bonds());

        assert!(!RenderMode::Wireframe.shows_atoms());
        assert!(RenderMode::Wireframe.shows_bonds());
    }

    #[test]
    fn test_bond_thickness_zero_when_no_bonds() {
        assert_eq!(RenderMode::CPK.bond_thickness(), 0.0);
        assert_eq!(RenderMode::Points.bond_thickness(), 0.0);
        assert!(RenderMode::BallAndStick.bond_thickness() > 0.0);
    }
}
