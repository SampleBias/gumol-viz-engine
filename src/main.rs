use bevy::prelude::*;
use gumol_viz_engine::GumolVizPlugin;
use gumol_viz_engine::systems::loading::{CliFileArg, LoadFileEvent, SimulationData};
use std::path::PathBuf;

fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting Gumol Viz Engine v{}", gumol_viz_engine::VERSION);

    App::new()
        // Add default Bevy plugins
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Gumol Viz Engine".to_string(),
                resolution: (1920., 1080.).into(),
                present_mode: bevy::window::PresentMode::AutoVsync,
                resizable: true,
                ..default()
            }),
            ..default()
        }))
        // Add UI plugins
        .add_plugins(bevy_egui::EguiPlugin)
        .add_plugins(bevy_mod_picking::DefaultPickingPlugins)
        .add_plugins(bevy_panorbit_camera::PanOrbitCameraPlugin)
        // Add Gumol Viz Engine plugin
        .add_plugins(GumolVizPlugin)
        // Add example-specific systems
        .add_systems(Startup, setup_scene)
        .add_systems(Startup, load_demo_trajectory)
        .add_systems(Update, toggle_fullscreen)
        .run();
}

/// Setup the initial scene with a demo molecule
fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    info!("Setting up demo scene...");

    // Add camera
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 0.0, 15.0)
                .looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        bevy_panorbit_camera::PanOrbitCamera::default(),
    ));

    // Add light
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

    // Add ambient light
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 0.3,
    });

    // Add a demo water molecule (H2O)
    spawn_water_molecule(&mut commands, &mut meshes, &mut materials);

    info!("Demo scene setup complete!");
}

/// Spawn a simple water molecule for demonstration
fn spawn_water_molecule(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    use gumol_viz_engine::core::atom::Element;
    use gumol_viz_engine::rendering;

    // Oxygen atom (red, larger)
    let oxygen_mesh = meshes.add(rendering::generate_atom_mesh(Element::O.vdw_radius()));
    let oxygen_material = materials.add(StandardMaterial {
        base_color: Color::rgb(0.8, 0.1, 0.1),
        metallic: 0.1,
        perceptual_roughness: 0.2,
        ..default()
    });

    commands.spawn(PbrBundle {
        mesh: oxygen_mesh.clone(),
        material: oxygen_material.clone(),
        transform: Transform::from_xyz(0.0, 0.0, 0.0),
        ..default()
    });

    // Hydrogen atoms (white, smaller)
    let hydrogen_mesh = meshes.add(rendering::generate_atom_mesh(Element::H.vdw_radius()));
    let hydrogen_material = materials.add(StandardMaterial {
        base_color: Color::rgb(0.9, 0.9, 0.9),
        metallic: 0.0,
        perceptual_roughness: 0.1,
        ..default()
    });

    // H1 position
    commands.spawn(PbrBundle {
        mesh: hydrogen_mesh.clone(),
        material: hydrogen_material.clone(),
        transform: Transform::from_xyz(0.757, 0.0, 0.0),
        ..default()
    });

    // H2 position
    commands.spawn(PbrBundle {
        mesh: hydrogen_mesh.clone(),
        material: hydrogen_material.clone(),
        transform: Transform::from_xyz(-0.757, 0.0, 0.0),
        ..default()
    });

    // Add O-H bonds as cylinders
    let bond_mesh = meshes.add(rendering::generate_bond_mesh(0.96, 0.1));
    let bond_material = materials.add(StandardMaterial {
        base_color: Color::rgb(0.7, 0.7, 0.7),
        metallic: 0.2,
        perceptual_roughness: 0.3,
        ..default()
    });

    // Bond 1
    commands.spawn(PbrBundle {
        mesh: bond_mesh.clone(),
        material: bond_material.clone(),
        transform: Transform {
            translation: Vec3::new(0.379, 0.0, 0.0),
            rotation: Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
            scale: Vec3::ONE,
        },
        ..default()
    });

    // Bond 2
    commands.spawn(PbrBundle {
        mesh: bond_mesh.clone(),
        material: bond_material.clone(),
        transform: Transform {
            translation: Vec3::new(-0.379, 0.0, 0.0),
            rotation: Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
            scale: Vec3::ONE,
        },
        ..default()
    });
}

/// Toggle fullscreen on F11
fn toggle_fullscreen(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut windows: Query<&mut Window>,
) {
    if keyboard.just_pressed(KeyCode::F11) {
        if let Ok(mut window) = windows.get_single_mut() {
            window.mode = match window.mode {
                bevy::window::WindowMode::Windowed => bevy::window::WindowMode::Fullscreen,
                _ => bevy::window::WindowMode::Windowed,
            };
        }
    }
}

/// Load a demo trajectory file (only when no CLI file was provided)
fn load_demo_trajectory(
    cli_arg: Res<CliFileArg>,
    mut load_events: EventWriter<LoadFileEvent>,
) {
    // Skip if user provided a file via CLI (load_cli_file handles it)
    if cli_arg.0.is_some() {
        return;
    }

    let demo_path = PathBuf::from("demo_trajectory.xyz");

    if demo_path.exists() {
        info!("Loading demo trajectory from file");
        load_events.send(LoadFileEvent { path: demo_path });
    } else {
        info!("No demo file found, using built-in water molecule");
    }
}
