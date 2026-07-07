//! POV-Ray scene export (`.pov` text format).
//!
//! Exports the current instanced scene as POV-Ray 3.7 spheres and cylinders
//! with CPK colors and the active camera viewpoint.

use crate::core::bond::Bond;
use crate::core::visualization::VisualizationConfig;
use crate::export::mesh_export::transform_vertex;
use crate::export::scene_snapshot::{capture_scene, SceneSnapshot};
use crate::rendering::atom_index::InstancedAtomIndex;
use crate::rendering::instanced::{InstancedAtomEntity, InstancedAtomMesh};
use crate::systems::bonds::BondEntities;
use crate::systems::loading::SimulationData;
use bevy::prelude::*;
use bevy_panorbit_camera::PanOrbitCamera;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

/// Viewport camera for POV-Ray `camera { ... }` block.
#[derive(Debug, Clone, Copy)]
pub struct CameraSnapshot {
    pub location: Vec3,
    pub look_at: Vec3,
    /// Vertical field of view in degrees.
    pub angle: f32,
}

impl Default for CameraSnapshot {
    fn default() -> Self {
        Self {
            location: Vec3::new(0.0, 0.0, 15.0),
            look_at: Vec3::ZERO,
            angle: 45.0,
        }
    }
}

#[derive(Event, Debug)]
pub struct RequestExportPovRayEvent {
    pub path: PathBuf,
}

/// Build camera snapshot from the primary 3D camera and orbit controller.
pub fn capture_camera(
    camera_transform: &Transform,
    pan_orbit: &PanOrbitCamera,
) -> CameraSnapshot {
    CameraSnapshot {
        location: camera_transform.translation,
        look_at: pan_orbit.focus,
        angle: 45.0,
    }
}

fn vec3_pov(v: Vec3) -> String {
    format!("<{:.6}, {:.6}, {:.6}>", v.x, v.y, v.z)
}

fn rgb_pov(color: [f32; 3]) -> String {
    format!("rgb<{:.4}, {:.4}, {:.4}>", color[0], color[1], color[2])
}

fn bond_endpoints(bond: &crate::export::scene_snapshot::BondSnapshot) -> (Vec3, Vec3) {
    let half = bond.length / 2.0;
    let top = transform_vertex([0.0, half, 0.0], bond.translation, bond.rotation);
    let bottom = transform_vertex([0.0, -half, 0.0], bond.translation, bond.rotation);
    (Vec3::from(top), Vec3::from(bottom))
}

/// Write atoms, bonds, and camera to a POV-Ray 3.7 scene file.
pub fn write_pov_to_path(
    path: &Path,
    data: &SceneSnapshot,
    camera: &CameraSnapshot,
) -> std::io::Result<()> {
    write_pov(&path.to_path_buf(), data, camera)
}

fn write_pov(path: &PathBuf, data: &SceneSnapshot, camera: &CameraSnapshot) -> std::io::Result<()> {
    let file = File::create(path)?;
    let mut w = BufWriter::new(file);

    writeln!(w, "// Gumol Viz Engine — POV-Ray export")?;
    writeln!(
        w,
        "// Atoms: {}, Bonds: {}",
        data.atoms.len(),
        data.bonds.len()
    )?;
    writeln!(w, "#version 3.7;")?;
    writeln!(w)?;
    writeln!(w, "global_settings {{")?;
    writeln!(w, "  assumed_gamma 1.0")?;
    writeln!(w, "}}")?;
    writeln!(w)?;
    writeln!(w, "background {{ color rgb<0.02, 0.02, 0.04> }}")?;
    writeln!(w)?;
    writeln!(w, "camera {{")?;
    writeln!(w, "  perspective")?;
    writeln!(w, "  location {}", vec3_pov(camera.location))?;
    writeln!(w, "  look_at {}", vec3_pov(camera.look_at))?;
    writeln!(w, "  angle {:.2}", camera.angle)?;
    writeln!(w, "}}")?;
    writeln!(w)?;
    writeln!(w, "light_source {{")?;
    writeln!(w, "  {}", vec3_pov(camera.location + Vec3::new(10.0, 15.0, 8.0)))?;
    writeln!(w, "  color rgb<1, 1, 1>")?;
    writeln!(w, "}}")?;
    writeln!(w)?;
    writeln!(w, "light_source {{")?;
    writeln!(w, "  {}", vec3_pov(camera.look_at + Vec3::new(-8.0, 6.0, -10.0)))?;
    writeln!(w, "  color rgb<0.4, 0.4, 0.5>")?;
    writeln!(w, "}}")?;
    writeln!(w)?;

    for (i, atom) in data.atoms.iter().enumerate() {
        writeln!(w, "sphere {{")?;
        writeln!(w, "  {}", vec3_pov(atom.position))?;
        writeln!(w, "  {:.6}", atom.radius)?;
        writeln!(w, "  pigment {{ {} }}", rgb_pov(atom.color))?;
        writeln!(w, "  finish {{ specular 0.35 roughness 0.15 }}")?;
        writeln!(w, "  // atom {}", i + 1)?;
        writeln!(w, "}}")?;
        writeln!(w)?;
    }

    let bond_color = [0.45, 0.45, 0.48];
    for bond in &data.bonds {
        let (a, b) = bond_endpoints(bond);
        writeln!(w, "cylinder {{")?;
        writeln!(w, "  {},", vec3_pov(a))?;
        writeln!(w, "  {},", vec3_pov(b))?;
        writeln!(w, "  {:.6}", bond.radius)?;
        writeln!(w, "  pigment {{ {} }}", rgb_pov(bond_color))?;
        writeln!(w, "  finish {{ specular 0.2 roughness 0.25 }}")?;
        writeln!(w, "}}")?;
        writeln!(w)?;
    }

    w.flush()?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn handle_export_povray(
    mut requests: EventReader<RequestExportPovRayEvent>,
    sim_data: Res<SimulationData>,
    viz_config: Res<VisualizationConfig>,
    index: Res<InstancedAtomIndex>,
    instanced: Query<(&InstancedAtomEntity, &InstancedAtomMesh)>,
    bond_query: Query<(&Transform, &Bond)>,
    bond_entities: Res<BondEntities>,
    camera_query: Query<(&Transform, &PanOrbitCamera), With<Camera3d>>,
) {
    for event in requests.read() {
        if !sim_data.loaded {
            warn!("POV-Ray export skipped: no molecule loaded");
            continue;
        }

        let snapshot = capture_scene(
            &index,
            &instanced,
            &sim_data.atom_data,
            &bond_query,
            &bond_entities,
            &viz_config,
        );

        let camera = camera_query
            .iter()
            .next()
            .map(|(transform, pan_orbit)| capture_camera(transform, pan_orbit))
            .unwrap_or_default();

        let path = event.path.clone();
        std::thread::spawn(move || {
            if let Err(err) = write_pov(&path, &snapshot, &camera) {
                error!("POV-Ray export failed: {err}");
            } else {
                info!("Exported POV-Ray scene to {}", path.display());
            }
        });
    }
}

pub fn register(app: &mut App) {
    app.add_event::<RequestExportPovRayEvent>()
        .add_systems(Update, handle_export_povray);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::scene_snapshot::{AtomSnapshot, BondSnapshot};

    #[test]
    fn test_write_pov_contains_sphere_and_camera() {
        let snapshot = SceneSnapshot {
            atoms: vec![AtomSnapshot {
                position: Vec3::new(0.0, 0.0, 0.0),
                radius: 0.5,
                color: [1.0, 0.0, 0.0],
            }],
            bonds: vec![BondSnapshot {
                translation: Vec3::new(0.0, 0.0, 0.0),
                rotation: Quat::IDENTITY,
                length: 2.0,
                radius: 0.1,
            }],
        };
        let camera = CameraSnapshot {
            location: Vec3::new(0.0, 0.0, 10.0),
            look_at: Vec3::ZERO,
            angle: 40.0,
        };

        let dir = std::env::temp_dir().join("gumol_pov_test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("test.pov");
        write_pov_to_path(&path, &snapshot, &camera).unwrap();

        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("#version 3.7"));
        assert!(text.contains("sphere {"));
        assert!(text.contains("cylinder {"));
        assert!(text.contains("look_at"));
        assert!(text.contains("rgb<1.0000, 0.0000, 0.0000>"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_capture_camera_uses_focus() {
        let transform = Transform::from_xyz(1.0, 2.0, 20.0);
        let mut pan = PanOrbitCamera::default();
        pan.focus = Vec3::new(3.0, 4.0, 5.0);
        let cam = capture_camera(&transform, &pan);
        assert_eq!(cam.location, transform.translation);
        assert_eq!(cam.look_at, pan.focus);
    }
}
