//! Gumol Viz Engine - A high-performance molecular dynamics visualization engine
//!
//! This library provides tools for visualizing molecular dynamics simulations
//! using the Bevy game engine for GPU-accelerated rendering.
//!
//! ## Features
//!
//! - Multiple file format support (XYZ, PDB, GRO, DCD, mmCIF)
//! - High-performance rendering (100,000+ atoms @ 60 FPS)
//! - Game-like interactivity (orbit camera, atom selection, measurements)
//! - Timeline animation with frame interpolation
//! - Multiple visualization modes (CPK, ball-and-stick, licorice, surface)
//! - Export capabilities (screenshots, videos, POV-Ray, OBJ, glTF)
//!
//! ## Quick Start
//!
//! ```no_run
//! use bevy::prelude::*;
//! use gumol_viz_engine::GumolVizPlugin;
//!
//! fn main() {
//!     App::new()
//!         .add_plugins(DefaultPlugins)
//!         .add_plugins(GumolVizPlugin)
//!         .run();
//! }
//! ```
//!
//! ## Modules
//!
//! - [`core`] - Core data structures (atoms, bonds, molecules)
//! - [`io`] - File I/O and format parsers
//! - [`rendering`] - Rendering systems and mesh generation
//! - [`systems`] - Bevy ECS systems
//! - [`camera`] - Camera controls
//! - [`interaction`] - User interaction (selection, measurement)
//! - [`ui`] - User interface (EGUI)
//! - [`export`] - Export functionality
//! - [`utils`] - Utility functions

pub mod core;
pub mod io;
pub mod rendering;
pub mod systems;
pub mod camera;
pub mod interaction;
pub mod ui;
pub mod export;
pub mod utils;

use bevy::prelude::*;

// Re-export common types for convenience
pub use core::{
    atom::{Atom, AtomData, Element},
    bond::{Bond, BondData, BondType},
    molecule::Molecule,
    trajectory::{FrameData, TimelineState, Trajectory},
    visualization::{RenderMode, VisualizationStyle},
};

/// Main plugin for Gumol Viz Engine
///
/// This plugin registers all systems, components, and resources needed
/// for molecular visualization.
pub struct GumolVizPlugin;

impl Plugin for GumolVizPlugin {
    fn build(&self, app: &mut App) {
        // Register modules
        core::register(app);
        io::register(app);
        rendering::register(app);
        systems::register(app);
        camera::register(app);
        interaction::register(app);
        ui::register(app);
        export::register(app);

        info!("Gumol Viz Engine v{} initialized", env!("CARGO_PKG_VERSION"));
    }
}

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Maximum number of atoms that can be visualized
pub const MAX_ATOMS: usize = 1_000_000;

/// Maximum number of frames in a trajectory
pub const MAX_FRAMES: usize = 100_000;

/// Default camera distance
pub const DEFAULT_CAMERA_DISTANCE: f32 = 20.0;

/// Minimum camera distance
pub const MIN_CAMERA_DISTANCE: f32 = 1.0;

/// Maximum camera distance
pub const MAX_CAMERA_DISTANCE: f32 = 1000.0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_constants() {
        assert!(MAX_ATOMS > 0);
        assert!(MAX_FRAMES > 0);
        assert!(DEFAULT_CAMERA_DISTANCE > 0.0);
    }
}
