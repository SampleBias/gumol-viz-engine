//! Basic XYZ file viewer example

use bevy::prelude::*;
use gumol_viz_engine::GumolVizPlugin;
use gumol_viz_engine::io::xyz::XYZParser;
use gumol_viz_engine::core::trajectory::Trajectory;

#[derive(Resource, Clone, Default)]
struct TrajectoryResource(Option<Trajectory>);

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let file_path = if args.len() > 1 {
        args[1].clone()
    } else {
        eprintln!("Usage: {} <xyz_file>", args[0]);
        eprintln!("Loading demo water molecule...");
        String::new()
    };

    // Load trajectory if file provided
    let trajectory = if !file_path.is_empty() {
        match XYZParser::parse_file(std::path::Path::new(&file_path)) {
            Ok(traj) => {
                println!("Loaded trajectory: {} frames, {} atoms", traj.num_frames(), traj.num_atoms);
                Some(traj)
            }
            Err(e) => {
                eprintln!("Error loading file: {}", e);
                None
            }
        }
    } else {
        None
    };

    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Gumol XYZ Viewer".to_string(),
                resolution: (1920., 1080.).into(),
                present_mode: bevy::window::PresentMode::AutoVsync,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(bevy_egui::EguiPlugin)
        .add_plugins(bevy_mod_picking::DefaultPickingPlugins)
        .add_plugins(bevy_panorbit_camera::PanOrbitCameraPlugin)
        .add_plugins(GumolVizPlugin)
        .insert_resource(TrajectoryResource(trajectory))
        .add_systems(Startup, setup_scene);
    app.run();
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    trajectory_res: Res<TrajectoryResource>,
) {
    info!("Setting up XYZ viewer scene...");

    // Add camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 0.0, 20.0)
            .looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    // Add lights
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

    // Load atoms from trajectory
    if let Some(traj) = &trajectory_res.0 {
        if let Some(frame) = traj.get_frame(0) {
            let sphere_mesh = meshes.add(gumol_viz_engine::rendering::generate_atom_mesh(0.5));

            let gray_material = materials.add(StandardMaterial {
                base_color: Color::srgb(0.7, 0.7, 0.7),
                metallic: 0.1,
                perceptual_roughness: 0.2,
                ..default()
            });

            // Spawn atoms
            for (atom_id, position) in frame.positions.iter() {
                commands.spawn(PbrBundle {
                    mesh: sphere_mesh.clone(),
                    material: gray_material.clone(),
                    transform: Transform::from_translation(*position),
                    ..default()
                });
            }

            info!("Loaded {} atoms", frame.positions.len());
        }
    } else {
        // Demo water molecule
        spawn_water_molecule(&mut commands, &mut meshes, &mut materials);
    }

    info!("Scene setup complete!");
}

fn spawn_water_molecule(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let sphere_mesh = meshes.add(gumol_viz_engine::rendering::generate_atom_mesh(0.5));

    let red_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.8, 0.1, 0.1),
        ..default()
    });

    let white_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.9, 0.9),
        ..default()
    });

    // Oxygen
    commands.spawn(PbrBundle {
        mesh: sphere_mesh.clone(),
        material: red_material.clone(),
        transform: Transform::from_xyz(0.0, 0.0, 0.0),
        ..default()
    });

    // Hydrogens
    commands.spawn(PbrBundle {
        mesh: sphere_mesh.clone(),
        material: white_material.clone(),
        transform: Transform::from_xyz(0.757, 0.0, 0.0),
        ..default()
    });

    commands.spawn(PbrBundle {
        mesh: sphere_mesh.clone(),
        material: white_material.clone(),
        transform: Transform::from_xyz(-0.757, 0.0, 0.0),
        ..default()
    });
}
