//! Timeline playback demo — loads a multi-frame trajectory and prints controls.
//!
//! ```bash
//! cargo run --example timeline_demo
//! cargo run --example timeline_demo -- path/to/trajectory.xyz
//! ```

use bevy::prelude::*;
use gumol_viz_engine::systems::loading::{CliFileArg, LoadFileEvent};
use gumol_viz_engine::GumolVizPlugin;
use std::path::PathBuf;

fn main() {
    println!("Gumol Viz Engine — Timeline Demo");
    println!("================================");
    println!("Space — play/pause   ← → — step frames");
    println!("Home/End — first/last frame   L — loop   I — interpolation");
    println!();

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Gumol Viz Engine — Timeline Demo".to_string(),
                resolution: (1280., 720.).into(),
                present_mode: bevy::window::PresentMode::AutoVsync,
                resizable: true,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(bevy_egui::EguiPlugin)
        .add_plugins(bevy_mod_picking::DefaultPickingPlugins)
        .add_plugins(bevy_panorbit_camera::PanOrbitCameraPlugin)
        .add_plugins(GumolVizPlugin)
        .add_systems(Startup, (setup_scene, load_trajectory))
        .run();
}

fn setup_scene(mut commands: Commands) {
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 0.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        bevy_panorbit_camera::PanOrbitCamera::default(),
    ));

    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 100_000.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(10.0, 10.0, 10.0),
        ..default()
    });

    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 0.3,
    });
}

const TRAJECTORY_CANDIDATES: &[&str] = &[
    "demo_trajectory.xyz",
    "tests/fixtures/water.xyz",
];

fn load_trajectory(cli_arg: Res<CliFileArg>, mut load_events: EventWriter<LoadFileEvent>) {
    if cli_arg.0.is_some() {
        return;
    }

    for candidate in TRAJECTORY_CANDIDATES {
        let path = PathBuf::from(candidate);
        if path.exists() {
            info!("Loading trajectory: {}", path.display());
            load_events.send(LoadFileEvent { path });
            return;
        }
    }

    warn!("No demo trajectory found — use: cargo run --example timeline_demo -- file.xyz");
}
