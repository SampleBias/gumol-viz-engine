//! OBJ format export
//!
//! Exports atoms (spheres) and bonds (cylinders) to Wavefront OBJ format
//! for use in Blender, MeshLab, and other 3D tools.

use crate::core::bond::Bond;
use crate::export::mesh_export::{generate_cylinder_mesh, generate_sphere_mesh, transform_vertex};
use crate::systems::spawning::SpawnedAtom;
use bevy::prelude::*;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

/// Event to request OBJ export
#[derive(Event, Debug)]
pub struct RequestExportObjEvent {
    pub path: PathBuf,
}

/// Data collected for export (cloned for thread)
struct ExportData {
    atoms: Vec<(Vec3, f32)>,
    bonds: Vec<(Vec3, Quat, f32, f32)>,
}

/// Handle OBJ export requests
pub fn handle_export_obj(
    mut requests: EventReader<RequestExportObjEvent>,
    atom_query: Query<(&Transform, &crate::core::atom::Atom), With<SpawnedAtom>>,
    bond_query: Query<(&Transform, &Bond)>,
    atom_entities: Res<crate::systems::spawning::AtomEntities>,
    bond_entities: Res<crate::systems::bonds::BondEntities>,
) {
    for event in requests.read() {
        let mut data = ExportData {
            atoms: Vec::new(),
            bonds: Vec::new(),
        };

        for (_, entity) in atom_entities.entities.iter() {
            if let Ok((transform, atom)) = atom_query.get(*entity) {
                let radius = atom.element.vdw_radius() * 0.5;
                data.atoms.push((transform.translation, radius));
            }
        }

        for (_, entity) in bond_entities.entities.iter() {
            if let Ok((transform, bond)) = bond_query.get(*entity) {
                data.bonds.push((
                    transform.translation,
                    transform.rotation,
                    bond.length,
                    0.1, // bond radius
                ));
            }
        }

        let path = event.path.clone();
        std::thread::spawn(move || {
            if let Err(e) = write_obj(&path, &data) {
                error!("OBJ export failed: {}", e);
            } else {
                info!("Exported OBJ to {:?}", path);
            }
        });
    }
}

fn write_obj(path: &PathBuf, data: &ExportData) -> std::io::Result<()> {
    let file = File::create(path)?;
    let mut w = BufWriter::new(file);

    writeln!(w, "# Gumol Viz Engine - OBJ Export")?;
    writeln!(w, "# Atoms: {}, Bonds: {}", data.atoms.len(), data.bonds.len())?;
    writeln!(w, "o molecule")?;

    let mut vertex_offset: u32 = 1; // OBJ uses 1-based indexing

    // Export atoms (spheres)
    let (sphere_verts, sphere_indices) = generate_sphere_mesh(1.0);
    for (pos, radius) in &data.atoms {
        let scale = Vec3::splat(*radius);
        for v in &sphere_verts {
            let scaled = [v[0] * scale.x, v[1] * scale.y, v[2] * scale.z];
            let world = transform_vertex(scaled, *pos, Quat::IDENTITY);
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

    // Export bonds (cylinders)
    for (translation, rotation, length, radius) in &data.bonds {
        let (cyl_verts, cyl_indices) = generate_cylinder_mesh(*length, *radius);
        for v in &cyl_verts {
            let world = transform_vertex(*v, *translation, *rotation);
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

/// Register OBJ export systems
pub fn register(app: &mut App) {
    app.add_event::<RequestExportObjEvent>()
        .add_systems(Update, handle_export_obj);
}
