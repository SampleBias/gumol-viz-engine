//! Basic file loading example

use bevy::prelude::*;
use gumol_viz_engine::GumolVizPlugin;

fn main() {
    println!("Gumol Viz Engine - Basic Load Example");
    println!("=====================================\n");

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Gumol Viz Engine - Basic Load".to_string(),
                resolution: (1280., 720.).into(),
                present_mode: bevy::window::PresentMode::AutoVsync,
                resizable: true,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(bevy_panorbit_camera::PanOrbitCameraPlugin)
        .add_plugins(GumolVizPlugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    println!("Initializing scene...");

    // Camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 0.0, 15.0)
            .looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    // Lights
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 50000.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(5.0, 5.0, 5.0),
        ..default()
    });

    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 0.2,
    });

    // Create a demo molecule (benzene ring)
    create_benzene_ring(&mut commands, &mut meshes, &mut materials);

    println!("Scene initialized successfully!");
    println!("\nControls:");
    println!("  - Left click + drag: Rotate camera");
    println!("  - Right click + drag: Pan camera");
    println!("  - Scroll: Zoom in/out");
}

fn create_benzene_ring(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    println!("Creating benzene ring...");

    let sphere = meshes.add(gumol_viz_engine::rendering::generate_atom_mesh(0.5));

    let cylinder = meshes.add(gumol_viz_engine::rendering::generate_bond_mesh(2.9, 0.1));

    let carbon_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.2, 0.2),
        metallic: 0.1,
        perceptual_roughness: 0.3,
        ..default()
    });

    let hydrogen_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.9, 0.9),
        metallic: 0.0,
        perceptual_roughness: 0.1,
        ..default()
    });

    let bond_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.5, 0.5, 0.5),
        metallic: 0.2,
        perceptual_roughness: 0.3,
        ..default()
    });

    // Benzene ring vertices (regular hexagon)
    let radius = 2.0;
    let carbons: Vec<Vec3> = (0..6)
        .map(|i| {
            let angle = i as f32 * std::f32::consts::PI / 3.0;
            Vec3::new(radius * angle.cos(), radius * angle.sin(), 0.0)
        })
        .collect();

    // Spawn carbon atoms
    for carbon in &carbons {
        commands.spawn(PbrBundle {
            mesh: sphere.clone(),
            material: carbon_mat.clone(),
            transform: Transform::from_translation(*carbon),
            ..default()
        });
    }

    // Spawn carbon-carbon bonds
    for i in 0..6 {
        let next_i = (i + 1) % 6;
        let pos_a = carbons[i];
        let pos_b = carbons[next_i];

        let midpoint = (pos_a + pos_b) / 2.0;
        let direction = (pos_b - pos_a).normalize();
        let up = Vec3::Z;
        let rotation = Quat::from_rotation_arc(up, direction);

        commands.spawn(PbrBundle {
            mesh: cylinder.clone(),
            material: bond_mat.clone(),
            transform: Transform {
                translation: midpoint,
                rotation,
                ..default()
            },
            ..default()
        });
    }

    // Spawn hydrogen atoms
    for carbon in &carbons {
        let hydrogen_pos = *carbon * 1.4;
        commands.spawn(PbrBundle {
            mesh: sphere.clone(),
            material: hydrogen_mat.clone(),
            transform: Transform::from_translation(hydrogen_pos),
            ..default()
        });
    }

    println!("  - 6 carbon atoms");
    println!("  - 6 hydrogen atoms");
    println!("  - 6 C-C bonds");
}
