//! User interaction systems (selection, measurement)

pub mod measurement;
pub mod pick_proxy;
pub mod selection;

use bevy::prelude::*;

/// Register all interaction systems
pub fn register(app: &mut App) {
    pick_proxy::register(app);
    selection::register(app);
    measurement::register(app);

    info!("Interaction module registered");
}
