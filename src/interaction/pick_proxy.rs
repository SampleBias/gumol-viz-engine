//! Invisible pick-proxy entities for atom selection with instanced rendering.

use crate::core::atom::{Atom, AtomData};
use crate::rendering::atom_index::InstancedAtomIndex;
use crate::rendering::instanced::InstancedAtomMesh;
use bevy::prelude::*;
use bevy_mod_picking::prelude::*;
use std::collections::HashMap;

/// Marker: pick-only entity linked to an atom ID.
#[derive(Component, Debug, Clone, Copy)]
pub struct PickProxy {
    pub atom_id: u32,
}

/// Tracks pick-proxy entities keyed by atom ID.
#[derive(Resource, Default, Debug)]
pub struct PickProxyEntities {
    pub entities: HashMap<u32, Entity>,
}

/// Spawn one pick proxy per atom (tiny nearly-invisible sphere for raycasting).
/// Returns `(entity map, selection enabled)`.
pub fn spawn_pick_proxies(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    atom_data: &[AtomData],
    frame_positions: &HashMap<u32, Vec3>,
    max_proxies: usize,
) -> (HashMap<u32, Entity>, bool) {
    if atom_data.len() > max_proxies {
        warn!(
            "Selection disabled: {} atoms exceeds pick proxy limit ({})",
            atom_data.len(),
            max_proxies
        );
        return (HashMap::new(), false);
    }
    let pick_mesh = meshes.add(crate::rendering::generate_atom_mesh(0.35));
    let pick_material = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 1.0, 1.0, 0.001),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });

    let mut map = HashMap::new();

    for atom in atom_data {
        let Some(position) = frame_positions.get(&atom.id).copied() else {
            continue;
        };

        let entity = commands
            .spawn((
                PbrBundle {
                    mesh: pick_mesh.clone(),
                    material: pick_material.clone(),
                    transform: Transform::from_translation(position),
                    ..default()
                },
                PickableBundle::default(),
                PickProxy { atom_id: atom.id },
                Atom {
                    id: atom.id,
                    element: atom.element,
                    position,
                    residue_id: atom.residue_id,
                    residue_name: atom.residue_name.clone(),
                    chain_id: atom.chain_id.clone(),
                    b_factor: atom.b_factor,
                    occupancy: atom.occupancy,
                    name: atom.name.clone(),
                },
            ))
            .id();

        map.insert(atom.id, entity);
    }

    (map, true)
}

pub fn register(app: &mut App) {
    app.init_resource::<PickProxyEntities>();
    info!("Pick proxy module registered");
}

/// Move pick proxies when timeline updates instanced positions.
pub fn update_pick_proxy_positions(
    index: Res<InstancedAtomIndex>,
    instanced: Query<(&crate::rendering::instanced::InstancedAtomEntity, &InstancedAtomMesh)>,
    mut pick_query: Query<(&PickProxy, &mut Transform)>,
) {
    if index.atom_to_instance.is_empty() {
        return;
    }

    let positions = index.collect_positions(&instanced);

    for (proxy, mut transform) in pick_query.iter_mut() {
        if let Some(pos) = positions.get(&proxy.atom_id) {
            transform.translation = *pos;
        }
    }
}
