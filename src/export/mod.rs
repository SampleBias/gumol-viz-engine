//! Export functionality (screenshots, videos, 3D formats)

pub mod gltf_export;
pub mod mesh_export;
pub mod obj;
pub mod screenshot;

use bevy::prelude::*;

/// Register all export systems
pub fn register(app: &mut App) {
    screenshot::register(app);
    obj::register(app);
    gltf_export::register(app);
    info!("Export module registered");
}
