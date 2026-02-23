//! Core data structures for molecular visualization
//!
//! This module defines the fundamental components used throughout
//! the visualization engine, including atoms, bonds, molecules,
//! and trajectory data.

pub mod atom;
pub mod bond;
pub mod molecule;
pub mod trajectory;
pub mod visualization;

use bevy::prelude::*;

/// Register all core components and resources
pub fn register(app: &mut App) {
    // Register components
    app.register_type::<atom::Atom>()
        .register_type::<atom::AtomData>()
        .register_type::<bond::Bond>()
        .register_type::<bond::BondData>()
        .register_type::<molecule::Molecule>()
        .register_type::<visualization::VisualizationStyle>()
        .register_type::<visualization::RenderMode>();

    // Register resources
    app.init_resource::<trajectory::TimelineState>()
        .init_resource::<SimulationData>();

    // Add startup system
    app.add_systems(Startup, initialize_core);

    info!("Core module registered");
}

/// Initialize core systems
fn initialize_core() {
    info!("Core systems initialized");
}

/// Global simulation data resource
#[derive(Resource, Default, Debug)]
pub struct SimulationData {
    /// Loaded trajectory data
    pub trajectory: Option<trajectory::Trajectory>,
    /// Atom metadata
    pub atoms: Vec<atom::AtomData>,
    /// Bond information
    pub bonds: Vec<bond::BondData>,
    /// Molecule information
    pub molecules: Vec<molecule::MoleculeData>,
    /// Metadata about the loaded system
    pub metadata: SimulationMetadata,
}

/// Metadata about the simulation
#[derive(Debug, Clone, Default)]
pub struct SimulationMetadata {
    /// System title
    pub title: String,
    /// Number of atoms
    pub num_atoms: usize,
    /// Number of bonds
    pub num_bonds: usize,
    /// Number of frames
    pub num_frames: usize,
    /// Time step in femtoseconds
    pub time_step: f32,
    /// Box dimensions (if periodic)
    pub box_size: Option<[f32; 3]>,
    /// File format
    pub format: String,
}
