//! Visualization mode system
//!
//! This system manages different visualization modes for atoms:
//! - CPK: Space-filling atoms with full van der Waals radii
//! - Ball-and-Stick: Smaller atoms with bonds
//! - Licorice: Very small atoms with thick bonds

use crate::core::atom::Element;
use bevy::prelude::*;

/// Visualization mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
#[reflect(Debug, PartialEq, Clone, Copy)]
pub enum VisualizationMode {
    /// Space-filling atoms (van der Waals radii)
    #[default]
    CPK,
    /// Ball-and-stick (smaller atoms, bonds)
    BallAndStick,
    /// Licorice (very small atoms, thick bonds)
    Licorice,
}

impl VisualizationMode {
    /// Get mode name for UI display
    pub fn name(&self) -> &'static str {
        match self {
            VisualizationMode::CPK => "CPK (Space-filling)",
            VisualizationMode::BallAndStick => "Ball-and-Stick",
            VisualizationMode::Licorice => "Licorice",
        }
    }

    /// Get atom radius multiplier for this mode
    pub fn atom_radius_multiplier(&self) -> f32 {
        match self {
            VisualizationMode::CPK => 1.0,           // Full VDW radius
            VisualizationMode::BallAndStick => 0.5,  // 50% VDW radius
            VisualizationMode::Licorice => 0.25,        // 25% VDW radius
        }
    }

    /// Get bond visibility for this mode
    pub fn show_bonds(&self) -> bool {
        match self {
            VisualizationMode::CPK => false,          // Bonds hidden
            VisualizationMode::BallAndStick => true,   // Bonds visible
            VisualizationMode::Licorice => true,   // Bonds visible (thick)
        }
    }

    /// Get bond thickness multiplier for this mode
    pub fn bond_thickness_multiplier(&self) -> f32 {
        match self {
            VisualizationMode::CPK => 0.0,            // No bonds
            VisualizationMode::BallAndStick => 1.0,   // Normal thickness
            VisualizationMode::Licorice => 2.0,         // Thick bonds
        }
    }
}

/// Resource containing visualization configuration
#[derive(Resource, Clone, Debug)]
pub struct VisualizationConfig {
    /// Current visualization mode
    pub mode: VisualizationMode,
    /// Atom size scaling factor (0.1 to 3.0)
    pub atom_scale: f32,
    /// Global bond scale factor
    pub bond_scale: f32,
    /// Show bonds?
    pub show_bonds: bool,
}

impl Default for VisualizationConfig {
    fn default() -> Self {
        Self {
            mode: VisualizationMode::default(),
            atom_scale: 1.0,
            bond_scale: 1.0,
            show_bonds: true,
        }
    }
}

impl VisualizationConfig {
    /// Calculate atom radius for an element in current mode
    pub fn get_atom_radius(&self, element: &Element) -> f32 {
        element.vdw_radius() * self.mode.atom_radius_multiplier() * self.atom_scale
    }

    /// Calculate bond radius in current mode
    pub fn get_bond_radius(&self, base_radius: f32) -> f32 {
        base_radius * self.bond_scale * self.mode.bond_thickness_multiplier()
    }
}

/// Marker component for atoms affected by visualization updates
#[derive(Component)]
pub struct UpdateVisualization;

/// Event sent when visualization mode changes
#[derive(Event, Debug)]
pub struct VisualizationModeChangedEvent {
    pub old_mode: VisualizationMode,
    pub new_mode: VisualizationMode,
}

/// Event sent when atom scale changes
#[derive(Event, Debug)]
pub struct AtomScaleChangedEvent {
    pub old_scale: f32,
    pub new_scale: f32,
}

/// Event sent when bond visibility changes
#[derive(Event, Debug)]
pub struct BondVisibilityChangedEvent {
    pub visible: bool,
}

/// Update atom meshes when visualization settings change
pub fn update_atom_visualization(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    viz_config: Res<VisualizationConfig>,
    atom_query: Query<(Entity, &crate::systems::spawning::SpawnedAtom, &Handle<StandardMaterial>), Without<UpdateVisualization>>,
) {
    // Check if any atoms need updating
    for (entity, _spawned, material_handle) in atom_query.iter() {
        // Mark for update
        commands.entity(entity).insert(UpdateVisualization);
    }
}

/// Process atoms that need visualization updates
pub fn process_visualization_updates(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    viz_config: Res<VisualizationConfig>,
    atom_query: Query<(Entity, &crate::systems::spawning::SpawnedAtom, &mut Handle<StandardMaterial>, With<UpdateVisualization>)>,
    spawn_query: Query<&crate::core::atom::Atom>,
) {
    for (entity, _spawned, material_handle) in atom_query.iter() {
        let atom = spawn_query.get(entity);
        let atom = match atom {
            Ok(a) => a,
            Err(_) => continue,
        };

        // Get current radius
        let new_radius = viz_config.get_atom_radius(&atom.element);

        // Generate new mesh with new radius
        let new_mesh = meshes.add(crate::rendering::generate_atom_mesh(new_radius));

        // Update material handle
        *material_handle = new_mesh;

        // Remove update marker
        commands.entity(entity).remove::<UpdateVisualization>();
    }
}

/// Update bond visualization settings
pub fn update_bond_visualization(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    viz_config: Res<VisualizationConfig>,
    bond_entities: Res<crate::systems::bonds::BondEntities>,
    bond_query: Query<(&crate::core::bond::Bond, &mut Transform, &mut Handle<StandardMaterial>)>,
) {
    // Show/hide bonds based on config
    let show = viz_config.show_bonds && viz_config.mode.show_bonds();

    for (bond_entity, _bond, mut transform, mut material) in bond_query.iter() {
        // Set visibility
        commands.entity(bond_entity).insert(bevy::prelude::Visibility {
            is_visible: show,
        });
    }

    info!("Bond visibility updated: {}", show);
}

/// Update atom scale in spawn system
pub fn update_spawn_atom_scale(
    mut commands: Commands,
    viz_config: Res<VisualizationConfig>,
) {
    // Trigger re-spawning with new scale
    if !commands.try_insert_resource(VisualizationModeChangedEvent {
        old_mode: viz_config.mode,
        new_mode: viz_config.mode,
    }) {
        // Force update by despawning and respawning atoms
        commands.remove_resource::<crate::systems::spawning::AtomEntities>();
    }
}

/// Set visualization mode
pub fn set_visualization_mode(
    mut viz_config: ResMut<VisualizationConfig>,
    mut events: EventWriter<VisualizationModeChangedEvent>,
    mode: VisualizationMode,
) {
    if viz_config.mode != mode {
        let old_mode = viz_config.mode;
        viz_config.mode = mode;
        viz_config.show_bonds = mode.show_bonds();

        events.send(VisualizationModeChangedEvent {
            old_mode,
            new_mode: mode,
        });

        info!("Visualization mode changed: {:?} -> {:?}", old_mode, mode);
    }
}

/// Set atom scale
pub fn set_atom_scale(
    mut viz_config: ResMut<VisualizationConfig>,
    mut events: EventWriter<AtomScaleChangedEvent>,
    scale: f32,
) {
    let old_scale = viz_config.atom_scale;
    viz_config.atom_scale = scale.clamp(0.1, 3.0);

    events.send(AtomScaleChangedEvent {
        old_scale,
        new_scale: scale,
    });

    info!("Atom scale changed: {:.2} -> {:.2}", old_scale, scale);
}

/// Set bond visibility
pub fn set_bond_visibility(
    mut viz_config: ResMut<VisualizationConfig>,
    mut events: EventWriter<BondVisibilityChangedEvent>,
    visible: bool,
) {
    viz_config.show_bonds = visible;

    events.send(BondVisibilityChangedEvent { visible });

    info!("Bond visibility changed: {}", visible);
}

/// Cycle through visualization modes
pub fn cycle_visualization_mode(
    mut viz_config: ResMut<VisualizationConfig>,
    mut mode_events: EventWriter<VisualizationModeChangedEvent>,
) {
    let modes = [
        VisualizationMode::CPK,
        VisualizationMode::BallAndStick,
        VisualizationMode::Licorice,
    ];
    let current_idx = modes.iter().position(|&m| m == viz_config.mode).unwrap_or(0);
    let next_idx = (current_idx + 1) % modes.len();
    let next_mode = modes[next_idx];

    let old_mode = viz_config.mode;
    viz_config.mode = next_mode;
    viz_config.show_bonds = next_mode.show_bonds();

    mode_events.send(VisualizationModeChangedEvent {
        old_mode,
        new_mode: next_mode,
    });

    info!("Cycled visualization mode: {:?} -> {:?}", old_mode, next_mode);
}

/// Register all visualization systems
pub fn register(app: &mut App) {
    app.init_resource::<VisualizationConfig>()
        .add_event::<VisualizationModeChangedEvent>()
        .add_event::<AtomScaleChangedEvent>()
        .add_event::<BondVisibilityChangedEvent>()
        .add_systems(Update, update_atom_visualization)
        .add_systems(Update, process_visualization_updates)
        .add_systems(Update, update_bond_visualization);

    info!("Visualization systems registered");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visualization_mode() {
        let mode = VisualizationMode::default();
        assert_eq!(mode, VisualizationMode::CPK);
        assert_eq!(mode.name(), "CPK (Space-filling)");
        assert_eq!(mode.atom_radius_multiplier(), 1.0);
        assert!(!mode.show_bonds());
    }

    #[test]
    fn test_bond_and_stick_mode() {
        let mode = VisualizationMode::BallAndStick;
        assert_eq!(mode.atom_radius_multiplier(), 0.5);
        assert!(mode.show_bonds());
        assert_eq!(mode.bond_thickness_multiplier(), 1.0);
    }

    #[test]
    fn test_licorice_mode() {
        let mode = VisualizationMode::Licorice;
        assert_eq!(mode.atom_radius_multiplier(), 0.25);
        assert!(mode.show_bonds());
        assert_eq!(mode.bond_thickness_multiplier(), 2.0);
    }

    #[test]
    fn test_visualization_config() {
        let config = VisualizationConfig::default();
        assert_eq!(config.mode, VisualizationMode::CPK);
        assert_eq!(config.atom_scale, 1.0);
        assert!(config.bond_scale, 1.0);
        assert!(config.show_bonds, false); // Will be set based on mode
    }

    #[test]
    fn test_get_atom_radius() {
        use crate::core::atom::Element;

        let config = VisualizationConfig::default();
        let hydrogen_radius = config.get_atom_radius(&Element::H);
        let carbon_radius = config.get_atom_radius(&Element::C);

        // Hydrogen VDW: ~1.2 Å
        assert!((hydrogen_radius - 1.2).abs() < 0.01);

        // Carbon VDW: ~1.7 Å
        assert!((carbon_radius - 1.7).abs() < 0.01);

        // Test scale factor
        config.atom_scale = 0.5;
        let scaled_hydrogen = config.get_atom_radius(&Element::H);
        assert!((scaled_hydrogen - 1.2 * 0.5).abs() < 0.01);
    }
}
