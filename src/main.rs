use bevy::prelude::*;
use gumol_viz_engine::systems::loading::{CliFileArg, LoadFileEvent};
use gumol_viz_engine::GumolVizPlugin;
use std::path::PathBuf;

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting Gumol Viz Engine v{}", gumol_viz_engine::VERSION);

    #[cfg(feature = "trace")]
    info!(
        "Trace feature enabled — run with Chrome tracing: \
         RUST_LOG=info,bevy_render=debug cargo run --release --features trace"
    );

    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Gumol Viz Engine".to_string(),
            resolution: (1920., 1080.).into(),
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
    .add_systems(Startup, setup_scene)
    .add_systems(Startup, load_default_trajectory)
    .add_systems(Update, toggle_fullscreen);

    #[cfg(feature = "trace")]
    {
        use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
        app.add_plugins((
            LogDiagnosticsPlugin::default(),
            FrameTimeDiagnosticsPlugin::default(),
        ));
    }

    app.run();
}

/// Camera and lighting only — atoms render through the instanced pipeline after file load.
fn setup_scene(mut commands: Commands) {
    info!("Setting up scene...");

    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 0.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        bevy_panorbit_camera::PanOrbitCamera::default(),
    ));

    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 100000.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(10.0, 10.0, 10.0),
        ..default()
    });

    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 5000.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(-5.0, 5.0, -5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 0.3,
    });

    info!("Scene setup complete");
}

/// Load a starter trajectory when no CLI file was provided (instanced rendering path).
fn load_default_trajectory(cli_arg: Res<CliFileArg>, mut load_events: EventWriter<LoadFileEvent>) {
    if cli_arg.0.is_some() {
        return;
    }

    for candidate in DEFAULT_TRAJECTORY_CANDIDATES {
        let path = PathBuf::from(candidate);
        if path.exists() {
            info!("Loading default trajectory: {}", path.display());
            load_events.send(LoadFileEvent { path });
            return;
        }
    }

    info!("No default trajectory found — open a file from the UI or pass one on the command line");
}

const DEFAULT_TRAJECTORY_CANDIDATES: &[&str] = &[
    "demo_trajectory.xyz",
    "tests/fixtures/water.xyz",
    "tests/fixtures/1CRN.pdb",
];

fn toggle_fullscreen(keyboard: Res<ButtonInput<KeyCode>>, mut windows: Query<&mut Window>) {
    if keyboard.just_pressed(KeyCode::F11) {
        if let Ok(mut window) = windows.get_single_mut() {
            window.mode = match window.mode {
                bevy::window::WindowMode::Windowed => bevy::window::WindowMode::Fullscreen,
                _ => bevy::window::WindowMode::Windowed,
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_trajectory_candidates_are_relative_paths() {
        for path in DEFAULT_TRAJECTORY_CANDIDATES {
            assert!(!PathBuf::from(path).is_absolute());
        }
    }
}
