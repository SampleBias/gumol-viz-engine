//! Wireframe bond rendering using `LineList` topology.
//!
//! Used when `RenderMode::Wireframe` is active — atoms are hidden and bonds
//! are drawn as thin unlit lines between connected atom pairs.

use crate::core::visualization::VisualizationConfig;
use crate::performance::PerformanceSettings;
use crate::rendering::atom_index::InstancedAtomIndex;
use crate::rendering::instanced::{
    InstancedAtomEntity, InstancedAtomMesh, InstancedAtomsSpawnedEvent,
};
use crate::systems::bonds::{resolve_bond_list, BondDetectionConfig};
use crate::systems::loading::SimulationData;
use crate::utils::spatial_index::AtomSpatialIndex;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;
use std::collections::HashMap;

const RENDER_ASSET_USAGES: RenderAssetUsages = RenderAssetUsages::RENDER_WORLD;

/// Marker for the single wireframe bond entity (one draw call for all lines).
#[derive(Component)]
pub struct WireframeBonds;

#[derive(Resource, Default, Debug)]
pub struct WireframeBondEntities {
    pub entity: Option<Entity>,
}

/// Build a line-list mesh from bond endpoint pairs.
pub fn generate_bond_line_mesh(segments: &[(Vec3, Vec3)]) -> Mesh {
    let mut positions = Vec::with_capacity(segments.len() * 2);
    for (a, b) in segments {
        positions.push([a.x, a.y, a.z]);
        positions.push([b.x, b.y, b.z]);
    }

    let indices: Vec<u32> = (0..positions.len() as u32).collect();

    let mut mesh = Mesh::new(PrimitiveTopology::LineList, RENDER_ASSET_USAGES);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

fn collect_bond_segments(
    sim_data: &SimulationData,
    positions: &HashMap<u32, Vec3>,
    index: &InstancedAtomIndex,
    instanced: &Query<(&InstancedAtomEntity, &InstancedAtomMesh)>,
    bond_config: &BondDetectionConfig,
    perf: &PerformanceSettings,
    spatial_index: Option<&AtomSpatialIndex>,
) -> Vec<(Vec3, Vec3)> {
    let positions = if positions.is_empty() {
        index.collect_positions(instanced)
    } else {
        positions.clone()
    };

    let bonds = resolve_bond_list(sim_data, &positions, bond_config, perf, spatial_index);

    let mut segments = Vec::with_capacity(bonds.len());
    for bond in bonds {
        let Some(a) = positions.get(&bond.atom_a_id) else {
            continue;
        };
        let Some(b) = positions.get(&bond.atom_b_id) else {
            continue;
        };
        segments.push((*a, *b));
    }
    segments
}

/// Spawn wireframe line entity after atoms load (hidden until wireframe mode).
#[allow(clippy::too_many_arguments)]
pub fn spawn_wireframe_bonds(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    sim_data: Res<SimulationData>,
    bond_config: Res<BondDetectionConfig>,
    perf: Res<PerformanceSettings>,
    spatial_index: Res<AtomSpatialIndex>,
    index: Res<InstancedAtomIndex>,
    instanced: Query<(&InstancedAtomEntity, &InstancedAtomMesh)>,
    mut wireframe_entities: ResMut<WireframeBondEntities>,
    mut spawned_events: EventReader<InstancedAtomsSpawnedEvent>,
) {
    if spawned_events.read().next().is_none() || wireframe_entities.entity.is_some() {
        return;
    }

    if !sim_data.loaded || index.atom_to_instance.is_empty() {
        return;
    }

    let segments = collect_bond_segments(
        &sim_data,
        &HashMap::new(),
        &index,
        &instanced,
        &bond_config,
        &perf,
        Some(&spatial_index),
    );
    if segments.is_empty() {
        return;
    }

    let mesh = meshes.add(generate_bond_line_mesh(&segments));
    let material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.55, 0.55, 0.55),
        unlit: true,
        ..default()
    });

    let entity = commands
        .spawn((
            PbrBundle {
                mesh,
                material,
                visibility: Visibility::Hidden,
                ..default()
            },
            WireframeBonds,
        ))
        .id();

    wireframe_entities.entity = Some(entity);
    info!("Spawned wireframe bond lines ({} segments)", segments.len());
}

/// Rebuild line positions when the timeline moves atoms.
#[allow(clippy::too_many_arguments)]
pub fn update_wireframe_bond_positions(
    sim_data: Res<SimulationData>,
    bond_config: Res<BondDetectionConfig>,
    perf: Res<PerformanceSettings>,
    spatial_index: Res<AtomSpatialIndex>,
    index: Res<InstancedAtomIndex>,
    instanced: Query<(&InstancedAtomEntity, &InstancedAtomMesh)>,
    wireframe_entities: ResMut<WireframeBondEntities>,
    mut meshes: ResMut<Assets<Mesh>>,
    mesh_query: Query<&Handle<Mesh>, With<WireframeBonds>>,
    timeline: Res<crate::core::trajectory::TimelineState>,
) {
    let Some(entity) = wireframe_entities.entity else {
        return;
    };

    if !sim_data.loaded {
        return;
    }

    if !timeline.is_changed() && !index.is_changed() {
        return;
    }

    let Ok(mesh_handle) = mesh_query.get(entity) else {
        return;
    };

    let segments = collect_bond_segments(
        &sim_data,
        &HashMap::new(),
        &index,
        &instanced,
        &bond_config,
        &perf,
        Some(&spatial_index),
    );
    if segments.is_empty() {
        return;
    }

    if let Some(mesh) = meshes.get_mut(mesh_handle) {
        *mesh = generate_bond_line_mesh(&segments);
    }
}

/// Show wireframe lines only in wireframe mode; hide cylinder bonds separately.
pub fn update_wireframe_visibility(
    viz_config: Res<VisualizationConfig>,
    wireframe_entities: ResMut<WireframeBondEntities>,
    mut visibility_query: Query<&mut Visibility, With<WireframeBonds>>,
) {
    if !viz_config.is_changed() {
        return;
    }

    let Some(entity) = wireframe_entities.entity else {
        return;
    };

    let Ok(mut visibility) = visibility_query.get_mut(entity) else {
        return;
    };

    let show = viz_config.show_bonds && viz_config.render_mode.uses_wireframe_lines();
    *visibility = if show {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
}

pub fn clear_wireframe_on_load(
    mut commands: Commands,
    mut wireframe_entities: ResMut<WireframeBondEntities>,
    mut file_loaded_events: EventReader<crate::systems::loading::FileLoadedEvent>,
) {
    if file_loaded_events.read().next().is_none() {
        return;
    }

    if let Some(entity) = wireframe_entities.entity.take() {
        commands.entity(entity).despawn_recursive();
        info!("Wireframe bonds cleared on file load");
    }
}

pub fn register(app: &mut App) {
    app.init_resource::<WireframeBondEntities>();
    info!("Wireframe rendering module registered");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_bond_line_mesh_segment_count() {
        let segments = vec![(Vec3::ZERO, Vec3::X), (Vec3::Y, Vec3::Z)];
        let mesh = generate_bond_line_mesh(&segments);
        assert_eq!(mesh.count_vertices(), 4);
    }
}
