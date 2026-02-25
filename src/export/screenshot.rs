//! Screenshot capture and export
//!
//! Captures the primary window to PNG/JPEG using Bevy's ScreenshotManager.

use bevy::prelude::*;
use bevy::render::view::window::screenshot::ScreenshotManager;
use bevy::window::PrimaryWindow;
use std::path::PathBuf;

/// Event to request a screenshot. Path determines output format (png, jpg, etc.)
#[derive(Event, Debug)]
pub struct RequestScreenshotEvent {
    pub path: PathBuf,
}

/// Handle screenshot requests via ScreenshotManager
pub fn handle_screenshot_requests(
    mut screenshot_manager: ResMut<ScreenshotManager>,
    mut requests: EventReader<RequestScreenshotEvent>,
    primary_window: Query<Entity, With<PrimaryWindow>>,
) {
    let Ok(window_entity) = primary_window.get_single() else {
        return;
    };

    for event in requests.read() {
        if screenshot_manager
            .save_screenshot_to_disk(window_entity, &event.path)
            .is_ok()
        {
            info!("Screenshot requested: {:?}", event.path);
        } else {
            warn!("Screenshot already in progress, skipping");
        }
    }
}

/// Register screenshot systems
pub fn register(app: &mut App) {
    app.add_event::<RequestScreenshotEvent>()
        .add_systems(Update, handle_screenshot_requests);
}
