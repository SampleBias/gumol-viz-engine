//! Verify the main plugin registers without panicking.

use bevy::prelude::*;
use bevy::window::WindowPlugin;
use bevy::winit::WinitPlugin;
use gumol_viz_engine::GumolVizPlugin;

#[test]
fn test_gumol_viz_plugin_registers() {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .build()
            .disable::<WindowPlugin>()
            .disable::<WinitPlugin>(),
    );
    app.add_plugins(GumolVizPlugin);

    assert!(
        app.world()
            .get_resource::<gumol_viz_engine::systems::loading::SimulationData>()
            .is_some(),
        "SimulationData resource should be registered"
    );
    assert!(
        app.world()
            .get_resource::<gumol_viz_engine::core::trajectory::TimelineState>()
            .is_some(),
        "TimelineState resource should be registered"
    );
}
