//! User interaction systems (selection, measurement)

pub mod measurement;
pub mod selection;

use bevy::prelude::*;

/// Register all interaction systems
pub fn register(app: &mut App) {
    selection::register(app);
    measurement::register(app);

    info!("Interaction module registered");
}
