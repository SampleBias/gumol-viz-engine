//! glTF 2.0 format export
//!
//! Exports atoms and bonds to glTF for use in web viewers, Blender, etc.
//! Uses embedded buffer (base64) for a single-file output.

use crate::core::bond::Bond;
use crate::export::mesh_export::{generate_cylinder_mesh, generate_sphere_mesh, transform_vertex};
use crate::systems::spawning::SpawnedAtom;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use bevy::prelude::*;
use gltf_json::buffer::View as BufferView;
use gltf_json::mesh::Semantic;
use gltf_json::validation::{Checked, USize64};
use gltf_json::{Accessor, Buffer, Index, Mesh, Root, Scene};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

/// Event to request glTF export
#[derive(Event, Debug)]
pub struct RequestExportGltfEvent {
    pub path: PathBuf,
}

/// Data collected for export
struct ExportData {
    atoms: Vec<(Vec3, f32)>,
    bonds: Vec<(Vec3, Quat, f32, f32)>,
}

/// Handle glTF export requests
pub fn handle_export_gltf(
    mut requests: EventReader<RequestExportGltfEvent>,
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
                    0.1,
                ));
            }
        }

        let path = event.path.clone();
        std::thread::spawn(move || {
            if let Err(e) = write_gltf(&path, &data) {
                error!("glTF export failed: {}", e);
            } else {
                info!("Exported glTF to {:?}", path);
            }
        });
    }
}

fn write_gltf(path: &PathBuf, data: &ExportData) -> std::io::Result<()> {
    let mut positions: Vec<f32> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    let (sphere_verts, sphere_indices) = generate_sphere_mesh(1.0);
    for (pos, radius) in &data.atoms {
        for v in &sphere_verts {
            let scaled = [v[0] * radius, v[1] * radius, v[2] * radius];
            let world = transform_vertex(scaled, *pos, Quat::IDENTITY);
            positions.extend_from_slice(&world);
        }
        let offset = (positions.len() / 3) as u32;
        for i in &sphere_indices {
            indices.push(*i + offset);
        }
    }

    for (translation, rotation, length, radius) in &data.bonds {
        let (cyl_verts, cyl_indices) = generate_cylinder_mesh(*length, *radius);
        let offset = (positions.len() / 3) as u32;
        for v in &cyl_verts {
            let world = transform_vertex(*v, *translation, *rotation);
            positions.extend_from_slice(&world);
        }
        for i in &cyl_indices {
            indices.push(*i + offset);
        }
    }

    let pos_bytes: Vec<u8> = bytemuck::cast_slice(&positions).to_vec();
    let idx_bytes: Vec<u8> = bytemuck::cast_slice(&indices).to_vec();

    let mut buffer_data = Vec::new();
    buffer_data.extend_from_slice(&pos_bytes);
    let pos_len = pos_bytes.len();
    let idx_offset = pos_len;
    buffer_data.extend_from_slice(&idx_bytes);

    let buffer_uri = format!(
        "data:application/octet-stream;base64,{}",
        BASE64.encode(&buffer_data)
    );

    let root = Root {
        asset: gltf_json::Asset {
            version: "2.0".to_string(),
            generator: Some("Gumol Viz Engine".to_string()),
            ..Default::default()
        },
        buffers: vec![Buffer {
            byte_length: USize64::from(buffer_data.len() as u64),
            uri: Some(buffer_uri),
            ..Default::default()
        }],
        buffer_views: vec![
            BufferView {
                buffer: Index::new(0),
                byte_offset: Some(USize64::from(0)),
                byte_length: USize64::from(pos_len as u64),
                target: Some(Checked::Valid(gltf_json::buffer::Target::ArrayBuffer)),
                ..Default::default()
            },
            BufferView {
                buffer: Index::new(0),
                byte_offset: Some(USize64::from(idx_offset as u64)),
                byte_length: USize64::from(idx_bytes.len() as u64),
                target: Some(Checked::Valid(gltf_json::buffer::Target::ElementArrayBuffer)),
                ..Default::default()
            },
        ],
        accessors: vec![
            Accessor {
                buffer_view: Some(Index::new(0)),
                byte_offset: Some(USize64::from(0)),
                component_type: Checked::Valid(gltf_json::accessor::GenericComponentType(
                    gltf_json::accessor::ComponentType::Float,
                )),
                count: USize64::from((positions.len() / 3) as u64),
                type_: Checked::Valid(gltf_json::accessor::Type::Vec3),
                ..Default::default()
            },
            Accessor {
                buffer_view: Some(Index::new(1)),
                byte_offset: Some(USize64::from(0)),
                component_type: Checked::Valid(gltf_json::accessor::GenericComponentType(
                    gltf_json::accessor::ComponentType::U32,
                )),
                count: USize64::from(indices.len() as u64),
                type_: Checked::Valid(gltf_json::accessor::Type::Scalar),
                ..Default::default()
            },
        ],
        meshes: vec![Mesh {
            primitives: vec![gltf_json::mesh::Primitive {
                attributes: {
                    let mut m = BTreeMap::new();
                    m.insert(
                        Checked::Valid(Semantic::Positions),
                        Index::new(0),
                    );
                    m
                },
                indices: Some(Index::new(1)),
                material: None,
                mode: Checked::Valid(gltf_json::mesh::Mode::Triangles),
                ..Default::default()
            }],
            ..Default::default()
        }],
        nodes: vec![gltf_json::Node {
            mesh: Some(Index::new(0)),
            ..Default::default()
        }],
        scenes: vec![Scene {
            nodes: vec![Index::new(0)],
            ..Default::default()
        }],
        scene: Some(Index::new(0)),
        ..Default::default()
    };

    let json = root.to_string_pretty().map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    let file = File::create(path)?;
    let mut w = BufWriter::new(file);
    w.write_all(json.as_bytes())?;
    w.flush()?;

    Ok(())
}

/// Register glTF export systems
pub fn register(app: &mut App) {
    app.add_event::<RequestExportGltfEvent>()
        .add_systems(Update, handle_export_gltf);
}
