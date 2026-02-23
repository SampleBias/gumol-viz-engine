//! Bevy ECS systems

pub mod loading;
pub mod spawning;

use bevy::prelude::*;

/// Register all systems
pub fn register(app: &mut App) {
    loading::register(app);
    spawning::register(app);

    info!("Systems module registered");
}
