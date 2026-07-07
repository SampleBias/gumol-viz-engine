//! OBJ format export

use crate::core::bond::Bond;
use crate::core::visualization::VisualizationConfig;
use crate::export::mesh_export::{generate_cylinder_mesh, generate_sphere_mesh, transform_vertex};
use crate::export::scene_snapshot::{capture_scene, SceneSnapshot};
use crate::rendering::atom_index::InstancedAtomIndex;
use crate::rendering::instanced::{InstancedAtomEntity, InstancedAtomMesh};
use crate::systems::bonds::BondEntities;
use crate::systems::loading::SimulationData;
use bevy::prelude::*;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

#[derive(Event, Debug)]
pub struct RequestExportObjEvent {
    pub path: PathBuf,
}

pub fn handle_export_obj(
    mut requests: EventReader<RequestExportObjEvent>,
    sim_data: Res<SimulationData>,
    viz_config: Res<VisualizationConfig>,
    index: Res<InstancedAtomIndex>,
    instanced: Query<(&InstancedAtomEntity, &InstancedAtomMesh)>,
    bond_query: Query<(&Transform, &Bond)>,
    bond_entities: Res<BondEntities>,
) {
    for event in requests.read() {
        let snapshot = capture_scene(
            &index,
            &instanced,
            &sim_data.atom_data,
            &bond_query,
            &bond_entities,
            &viz_config,
        );

        let path = event.path.clone();
        std::thread::spawn(move || {
            if let Err(e) = write_obj(&path, &snapshot) {
                error!("OBJ export failed: {}", e);
            } else {
                info!("Exported OBJ to {:?}", path);
            }
        });
    }
}

/// Write a scene snapshot to an OBJ file on disk.
pub fn write_obj_to_path(path: &std::path::Path, data: &SceneSnapshot) -> std::io::Result<()> {
    write_obj(&path.to_path_buf(), data)
}

fn write_obj(path: &PathBuf, data: &SceneSnapshot) -> std::io::Result<()> {
    let file = File::create(path)?;
    let mut w = BufWriter::new(file);

    writeln!(w, "# Gumol Viz Engine - OBJ Export")?;
    writeln!(
        w,
        "# Atoms: {}, Bonds: {}",
        data.atoms.len(),
        data.bonds.len()
    )?;
    writeln!(w, "o molecule")?;

    let mut vertex_offset: u32 = 1;

    let (sphere_verts, sphere_indices) = generate_sphere_mesh(1.0);
    for atom in &data.atoms {
        for v in &sphere_verts {
            let scaled = [v[0] * atom.radius, v[1] * atom.radius, v[2] * atom.radius];
            let world = transform_vertex(scaled, atom.position, Quat::IDENTITY);
            writeln!(w, "v {} {} {}", world[0], world[1], world[2])?;
        }
        for tri in sphere_indices.chunks(3) {
            writeln!(
                w,
                "f {} {} {}",
                tri[0] + vertex_offset,
                tri[1] + vertex_offset,
                tri[2] + vertex_offset
            )?;
        }
        vertex_offset += sphere_verts.len() as u32;
    }

    for bond in &data.bonds {
        let (cyl_verts, cyl_indices) = generate_cylinder_mesh(bond.length, bond.radius);
        for v in &cyl_verts {
            let world = transform_vertex(*v, bond.translation, bond.rotation);
            writeln!(w, "v {} {} {}", world[0], world[1], world[2])?;
        }
        for tri in cyl_indices.chunks(3) {
            writeln!(
                w,
                "f {} {} {}",
                tri[0] + vertex_offset,
                tri[1] + vertex_offset,
                tri[2] + vertex_offset
            )?;
        }
        vertex_offset += cyl_verts.len() as u32;
    }

    w.flush()?;
    Ok(())
}

pub fn register(app: &mut App) {
    app.add_event::<RequestExportObjEvent>()
        .add_systems(Update, handle_export_obj);
}
