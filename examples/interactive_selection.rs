//! Interactive atom selection demo.
//!
//! Click atoms to select; Shift+click toggles multi-select; Escape clears.
//! Use the inspector panel for measurements on selected atoms.
//!
//! ```bash
//! cargo run --example interactive_selection
//! cargo run --example interactive_selection -- tests/fixtures/1CRN.pdb
//! ```

use bevy::prelude::*;
use gumol_viz_engine::systems::loading::{CliFileArg, LoadFileEvent};
use gumol_viz_engine::GumolVizPlugin;
use std::path::PathBuf;

fn main() {
    println!("Gumol Viz Engine — Interactive Selection Demo");
    println!("=============================================");
    println!("Click — select atom");
    println!("Shift+Click — add/remove from selection");
    println!("Escape — clear selection");
    println!("F — focus camera on molecule   Shift+F — focus on selection");
    println!();

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Gumol Viz Engine — Selection Demo".to_string(),
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
        .add_systems(Startup, (setup_scene, load_structure))
        .run();
}

fn setup_scene(mut commands: Commands) {
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 0.0, 25.0).looking_at(Vec3::ZERO, Vec3::Y),
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

const STRUCTURE_CANDIDATES: &[&str] = &[
    "tests/fixtures/1CRN.pdb",
    "tests/fixtures/water.xyz",
    "demo_trajectory.xyz",
];

fn load_structure(cli_arg: Res<CliFileArg>, mut load_events: EventWriter<LoadFileEvent>) {
    if cli_arg.0.is_some() {
        return;
    }

    for candidate in STRUCTURE_CANDIDATES {
        let path = PathBuf::from(candidate);
        if path.exists() {
            info!("Loading structure: {}", path.display());
            load_events.send(LoadFileEvent { path });
            return;
        }
    }

    warn!("No demo structure found — pass a PDB/XYZ path on the command line");
}
