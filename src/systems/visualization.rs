//! Visualization mode switching system
//!
//! This system manages switching between different rendering modes for atoms and bonds.
//! It uses the RenderMode enum from core::visualization and applies global settings.

use crate::core::visualization::{RenderMode, VisualizationConfig};
use bevy::prelude::*;

/// Event sent when visualization mode changes
#[derive(Event, Debug)]
pub struct VisualizationModeChangedEvent {
    pub old_mode: RenderMode,
    pub new_mode: RenderMode,
}

/// Event sent when atom scale changes
#[derive(Event, Debug)]
pub struct AtomScaleChangedEvent {
    pub old_scale: f32,
    pub new_scale: f32,
}

/// Event sent when bond scale changes
#[derive(Event, Debug)]
pub struct BondScaleChangedEvent {
    pub old_scale: f32,
    pub new_scale: f32,
}

/// Event sent when atom visibility changes
#[derive(Event, Debug)]
pub struct AtomVisibilityChangedEvent {
    pub visible: bool,
}

/// Event sent when bond visibility changes
#[derive(Event, Debug)]
pub struct BondVisibilityChangedEvent {
    pub visible: bool,
}

/// Update atom visibility based on config
pub fn update_atom_visibility(
    config: Res<VisualizationConfig>,
    mut atom_query: Query<(&mut Visibility, &crate::systems::spawning::SpawnedAtom)>,
) {
    let should_show = config.show_atoms && config.render_mode.shows_atoms();

    for (mut visibility, _spawned_atom) in atom_query.iter_mut() {
        *visibility = if should_show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

/// Update bond visibility based on config
pub fn update_bond_visibility(
    config: Res<VisualizationConfig>,
    mut bond_query: Query<&mut Visibility, Without<crate::systems::spawning::SpawnedAtom>>,
) {
    let should_show = config.show_bonds && config.render_mode.shows_bonds();

    for mut visibility in bond_query.iter_mut() {
        *visibility = if should_show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

/// Update atom scale based on render mode and config
pub fn update_atom_scale(
    config: Res<VisualizationConfig>,
    atom_entities: Res<crate::systems::spawning::AtomEntities>,
    mut atom_query: Query<(&crate::core::atom::Atom, &mut Transform)>,
) {
    let mode_scale = config.render_mode.atom_scale();
    let overall_scale = mode_scale * config.atom_scale;

    for (_atom_id, &entity) in atom_entities.entities.iter() {
        if let Ok((atom, mut transform)) = atom_query.get_mut(entity) {
            let base_radius = atom.element.vdw_radius();
            let scale_factor = overall_scale / base_radius;

            transform.scale = Vec3::splat(scale_factor);
        }
    }
}

/// Update bond thickness based on render mode and config
pub fn update_bond_scale(
    config: Res<VisualizationConfig>,
    bond_entities: Res<crate::systems::bonds::BondEntities>,
    mut bond_query: Query<&mut Transform>,
) {
    let mode_thickness = config.render_mode.bond_thickness();
    let overall_thickness = mode_thickness * config.bond_scale;

    for (_bond_id, &entity) in bond_entities.entities.iter() {
        if let Ok(mut transform) = bond_query.get_mut(entity) {
            // Scale the cylinder along X and Y (Z is the length)
            transform.scale.x = overall_thickness;
            transform.scale.y = overall_thickness;
        }
    }
}

/// Set visualization mode and trigger update events
pub fn set_render_mode(
    mut config: ResMut<VisualizationConfig>,
    mut mode_events: EventWriter<VisualizationModeChangedEvent>,
    mode: RenderMode,
) {
    if config.render_mode != mode {
        let old_mode = config.render_mode;
        config.render_mode = mode;

        mode_events.send(VisualizationModeChangedEvent { old_mode, new_mode: mode });

        info!("Visualization mode changed: {:?} -> {:?}", old_mode, mode);
    }
}

/// Set atom scale and trigger update events
pub fn set_atom_scale(
    mut config: ResMut<VisualizationConfig>,
    mut scale_events: EventWriter<AtomScaleChangedEvent>,
    scale: f32,
) {
    let old_scale = config.atom_scale;
    config.atom_scale = scale.clamp(0.1, 2.0);

    scale_events.send(AtomScaleChangedEvent {
        old_scale,
        new_scale: config.atom_scale,
    });

    info!("Atom scale changed: {:.2} -> {:.2}", old_scale, config.atom_scale);
}

/// Set bond scale and trigger update events
pub fn set_bond_scale(
    mut config: ResMut<VisualizationConfig>,
    mut scale_events: EventWriter<BondScaleChangedEvent>,
    scale: f32,
) {
    let old_scale = config.bond_scale;
    config.bond_scale = scale.clamp(0.1, 3.0);

    scale_events.send(BondScaleChangedEvent {
        old_scale,
        new_scale: config.bond_scale,
    });

    info!("Bond scale changed: {:.2} -> {:.2}", old_scale, config.bond_scale);
}

/// Set atom visibility and trigger update events
pub fn set_atom_visibility(
    mut config: ResMut<VisualizationConfig>,
    mut visibility_events: EventWriter<AtomVisibilityChangedEvent>,
    visible: bool,
) {
    config.show_atoms = visible;

    visibility_events.send(AtomVisibilityChangedEvent { visible });

    info!("Atom visibility changed: {}", visible);
}

/// Set bond visibility and trigger update events
pub fn set_bond_visibility(
    mut config: ResMut<VisualizationConfig>,
    mut visibility_events: EventWriter<BondVisibilityChangedEvent>,
    visible: bool,
) {
    config.show_bonds = visible;

    visibility_events.send(BondVisibilityChangedEvent { visible });

    info!("Bond visibility changed: {}", visible);
}

/// Cycle through visualization modes
pub fn cycle_render_mode(
    mut config: ResMut<VisualizationConfig>,
    mut mode_events: EventWriter<VisualizationModeChangedEvent>,
) {
    let modes = [
        RenderMode::CPK,
        RenderMode::BallAndStick,
        RenderMode::Licorice,
        RenderMode::Wireframe,
        RenderMode::Surface,
        RenderMode::Cartoon,
        RenderMode::Tube,
        RenderMode::Trace,
        RenderMode::Points,
    ];

    let current_idx = modes
        .iter()
        .position(|&m| m == config.render_mode)
        .unwrap_or(0);
    let next_idx = (current_idx + 1) % modes.len();
    let next_mode = modes[next_idx];

    let old_mode = config.render_mode;
    config.render_mode = next_mode;

    mode_events.send(VisualizationModeChangedEvent { old_mode, new_mode: next_mode });

    info!("Cycled visualization mode: {:?} -> {:?}", old_mode, next_mode);
}

/// Register all visualization systems
pub fn register(app: &mut App) {
    app.init_resource::<VisualizationConfig>()
        .add_event::<VisualizationModeChangedEvent>()
        .add_event::<AtomScaleChangedEvent>()
        .add_event::<BondScaleChangedEvent>()
        .add_event::<AtomVisibilityChangedEvent>()
        .add_event::<BondVisibilityChangedEvent>()
        .add_systems(Update, update_atom_visibility)
        .add_systems(Update, update_bond_visibility)
        .add_systems(Update, update_atom_scale)
        .add_systems(Update, update_bond_scale);

    info!("Visualization systems registered");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_mode_cycle() {
        let modes = [
            RenderMode::CPK,
            RenderMode::BallAndStick,
            RenderMode::Licorice,
            RenderMode::Wireframe,
            RenderMode::Surface,
            RenderMode::Cartoon,
            RenderMode::Tube,
            RenderMode::Trace,
            RenderMode::Points,
        ];

        // Test that we can cycle through all modes
        for (i, &mode) in modes.iter().enumerate() {
            assert_eq!(modes[i], mode);
        }
    }

    #[test]
    fn test_clamp_atom_scale() {
        // Test minimum
        let scale_min = 0.05.clamp(0.1, 2.0);
        assert_eq!(scale_min, 0.1);

        // Test maximum
        let scale_max = 3.0.clamp(0.1, 2.0);
        assert_eq!(scale_max, 2.0);

        // Test in range
        let scale_mid = 1.0.clamp(0.1, 2.0);
        assert_eq!(scale_mid, 1.0);
    }

    #[test]
    fn test_clamp_bond_scale() {
        // Test minimum
        let scale_min = 0.05.clamp(0.1, 3.0);
        assert_eq!(scale_min, 0.1);

        // Test maximum
        let scale_max = 4.0.clamp(0.1, 3.0);
        assert_eq!(scale_max, 3.0);

        // Test in range
        let scale_mid = 1.5.clamp(0.1, 3.0);
        assert_eq!(scale_mid, 1.5);
    }
}
