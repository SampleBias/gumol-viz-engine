//! Bevy ECS systems

pub mod loading;
pub mod spawning;
pub mod timeline;

use bevy::prelude::*;

/// Register all systems
pub fn register(app: &mut App) {
    loading::register(app);
    spawning::register(app);
    timeline::register(app);

    info!("Systems module registered");
}
