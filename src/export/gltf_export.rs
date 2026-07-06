//! glTF 2.0 format export
//!
//! Exports atoms and bonds to glTF for use in web viewers, Blender, etc.
//! Uses embedded buffer (base64) for a single-file output.

use crate::core::bond::Bond;
use crate::core::visualization::VisualizationConfig;
use crate::export::mesh_export::{generate_cylinder_mesh, generate_sphere_mesh, transform_vertex};
use crate::export::scene_snapshot::{capture_scene, SceneSnapshot};
use crate::rendering::atom_index::InstancedAtomIndex;
use crate::rendering::instanced::{InstancedAtomEntity, InstancedAtomMesh};
use crate::systems::bonds::BondEntities;
use crate::systems::loading::SimulationData;
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

/// Handle glTF export requests
pub fn handle_export_gltf(
    mut requests: EventReader<RequestExportGltfEvent>,
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
            if let Err(e) = write_gltf(&path, &snapshot) {
                error!("glTF export failed: {}", e);
            } else {
                info!("Exported glTF to {:?}", path);
            }
        });
    }
}

fn write_gltf(path: &PathBuf, data: &SceneSnapshot) -> std::io::Result<()> {
    let mut positions: Vec<f32> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    let (sphere_verts, sphere_indices) = generate_sphere_mesh(1.0);
    for atom in &data.atoms {
        for v in &sphere_verts {
            let scaled = [v[0] * atom.radius, v[1] * atom.radius, v[2] * atom.radius];
            let world = transform_vertex(scaled, atom.position, Quat::IDENTITY);
            positions.extend_from_slice(&world);
        }
        let offset = (positions.len() / 3) as u32;
        for i in &sphere_indices {
            indices.push(*i + offset);
        }
    }

    for bond in &data.bonds {
        let (cyl_verts, cyl_indices) = generate_cylinder_mesh(bond.length, bond.radius);
        let offset = (positions.len() / 3) as u32;
        for v in &cyl_verts {
            let world = transform_vertex(*v, bond.translation, bond.rotation);
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
            extensions: None,
            extras: Default::default(),
        }],
        buffer_views: vec![
            BufferView {
                buffer: Index::new(0),
                byte_length: USize64::from(pos_len as u64),
                byte_offset: Some(USize64::from(0u64)),
                byte_stride: None,
                target: Some(Checked::Valid(gltf_json::buffer::Target::ArrayBuffer)),
                extensions: None,
                extras: Default::default(),
            },
            BufferView {
                buffer: Index::new(0),
                byte_length: USize64::from(idx_bytes.len() as u64),
                byte_offset: Some(USize64::from(idx_offset as u64)),
                byte_stride: None,
                target: Some(Checked::Valid(
                    gltf_json::buffer::Target::ElementArrayBuffer,
                )),
                extensions: None,
                extras: Default::default(),
            },
        ],
        accessors: vec![
            Accessor {
                buffer_view: Some(Index::new(0)),
                byte_offset: Some(USize64::from(0u64)),
                count: USize64::from((positions.len() / 3) as u64),
                component_type: Checked::Valid(gltf_json::accessor::GenericComponentType(
                    gltf_json::accessor::ComponentType::F32,
                )),
                extensions: None,
                extras: Default::default(),
                type_: Checked::Valid(gltf_json::accessor::Type::Vec3),
                min: None,
                max: None,
                normalized: false,
                sparse: None,
            },
            Accessor {
                buffer_view: Some(Index::new(1)),
                byte_offset: Some(USize64::from(0u64)),
                count: USize64::from(indices.len() as u64),
                component_type: Checked::Valid(gltf_json::accessor::GenericComponentType(
                    gltf_json::accessor::ComponentType::U32,
                )),
                extensions: None,
                extras: Default::default(),
                type_: Checked::Valid(gltf_json::accessor::Type::Scalar),
                min: None,
                max: None,
                normalized: false,
                sparse: None,
            },
        ],
        meshes: vec![Mesh {
            primitives: vec![gltf_json::mesh::Primitive {
                attributes: {
                    let mut m = BTreeMap::new();
                    m.insert(Checked::Valid(Semantic::Positions), Index::new(0));
                    m
                },
                extensions: None,
                extras: Default::default(),
                indices: Some(Index::new(1)),
                material: None,
                mode: Checked::Valid(gltf_json::mesh::Mode::Triangles),
                targets: None,
            }],
            extensions: None,
            extras: Default::default(),
            weights: None,
        }],
        nodes: vec![gltf_json::Node {
            mesh: Some(Index::new(0)),
            ..Default::default()
        }],
        scenes: vec![Scene {
            extensions: None,
            extras: Default::default(),
            nodes: vec![Index::new(0)],
        }],
        scene: Some(Index::new(0)),
        ..Default::default()
    };

    let json = root.to_string_pretty().map_err(std::io::Error::other)?;

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
