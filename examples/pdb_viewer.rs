//! Basic PDB file viewer example

use bevy::prelude::*;
use gumol_viz_engine::GumolVizPlugin;
use gumol_viz_engine::io::pdb::PDBParser;
use gumol_viz_engine::core::trajectory::Trajectory;

#[derive(Resource, Clone, Default)]
struct TrajectoryResource(Option<Trajectory>);

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let file_path = if args.len() > 1 {
        args[1].clone()
    } else {
        eprintln!("Usage: {} <pdb_file>", args[0]);
        eprintln!("Loading demo protein...");
        String::new()
    };

    // Load trajectory if file provided
    let trajectory = if !file_path.is_empty() {
        match PDBParser::parse_file(std::path::Path::new(&file_path)) {
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
                title: "Gumol PDB Viewer".to_string(),
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
    info!("Setting up PDB viewer scene...");

    // Add camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 0.0, 30.0)
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

            // Materials for different elements (simplified)
            let carbon_material = materials.add(StandardMaterial {
                base_color: Color::srgb(0.2, 0.2, 0.2),
                ..default()
            });

            let nitrogen_material = materials.add(StandardMaterial {
                base_color: Color::srgb(0.1, 0.1, 0.8),
                ..default()
            });

            let oxygen_material = materials.add(StandardMaterial {
                base_color: Color::srgb(0.8, 0.1, 0.1),
                ..default()
            });

            let sulfur_material = materials.add(StandardMaterial {
                base_color: Color::srgb(0.8, 0.8, 0.1),
                ..default()
            });

            // Spawn atoms (simplified - just using atom ID to determine color)
            for (atom_id, position) in frame.positions.iter() {
                let material = match atom_id % 4 {
                    0 => carbon_material.clone(),
                    1 => nitrogen_material.clone(),
                    2 => oxygen_material.clone(),
                    _ => sulfur_material.clone(),
                };

                commands.spawn(PbrBundle {
                    mesh: sphere_mesh.clone(),
                    material,
                    transform: Transform::from_translation(*position),
                    ..default()
                });
            }

            info!("Loaded {} atoms", frame.positions.len());
        }
    } else {
        // Demo peptide
        spawn_demo_peptide(&mut commands, &mut meshes, &mut materials);
    }

    info!("Scene setup complete!");
}

fn spawn_demo_peptide(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let sphere_mesh = meshes.add(gumol_viz_engine::rendering::generate_atom_mesh(0.5));

    let carbon_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.2, 0.2),
        ..default()
    });

    let nitrogen_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.1, 0.1, 0.8),
        ..default()
    });

    let oxygen_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.8, 0.1, 0.1),
        ..default()
    });

    // Simple tripeptide: Alanine-Glycine-Serine
    let atoms = [
        (Vec3::new(0.0, 0.0, 0.0), "C", carbon_mat.clone()),
        (Vec3::new(1.5, 0.0, 0.0), "N", nitrogen_mat.clone()),
        (Vec3::new(2.5, 1.0, 0.0), "CA", carbon_mat.clone()),
        (Vec3::new(3.5, 0.0, 0.0), "C", carbon_mat.clone()),
        (Vec3::new(4.5, 1.0, 0.0), "O", oxygen_mat.clone()),
        (Vec3::new(3.5, -1.0, 0.0), "N", nitrogen_mat.clone()),
        (Vec3::new(4.5, -2.0, 0.0), "CA", carbon_mat.clone()),
    ];

    for (pos, _name, material) in atoms {
        commands.spawn(PbrBundle {
            mesh: sphere_mesh.clone(),
            material,
            transform: Transform::from_translation(pos),
            ..default()
        });
    }

    info!("Loaded demo peptide");
}
