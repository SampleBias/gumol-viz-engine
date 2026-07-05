//! LOD mesh selection for instanced atom batches.

use crate::performance::{PerformanceDiagnostics, PerformanceSettings};
use crate::rendering::instanced::{InstancedAtomEntity, InstancedAtomMesh};
use crate::rendering::lod::{select_batch_lod, AtomLod};
use crate::rendering::mesh_pool::AtomMeshPool;
use bevy::prelude::*;

/// Swap element batch meshes when screen-space LOD changes.
pub fn update_instanced_lod_meshes(
    perf: Res<PerformanceSettings>,
    mut diagnostics: ResMut<PerformanceDiagnostics>,
    mut mesh_pool: ResMut<AtomMeshPool>,
    mut meshes: ResMut<Assets<Mesh>>,
    camera: Query<(&Camera, &GlobalTransform, &Projection)>,
    mut instanced: Query<(
        &InstancedAtomEntity,
        &mut Handle<Mesh>,
        &InstancedAtomMesh,
    )>,
) {
    if !perf.lod_enabled {
        diagnostics.current_lod = AtomLod::High;
        return;
    }

    let Ok((camera, transform, projection)) = camera.get_single() else {
        return;
    };

    let viewport_h = camera
        .logical_viewport_size()
        .map(|v| v.y)
        .unwrap_or(1080.0);

    let mut batch_lod = mesh_pool.current_lod();

    for (entity_info, mut mesh_handle, inst_mesh) in instanced.iter_mut() {
        let sample_pos = inst_mesh
            .instances
            .first()
            .map(|i| i.position)
            .unwrap_or(Vec3::ZERO);
        let world_radius = entity_info.element.vdw_radius() * 0.5 * inst_mesh.mode_scale.max(0.001);

        batch_lod = select_batch_lod(
            sample_pos,
            world_radius,
            transform,
            projection,
            viewport_h,
            batch_lod,
        );

        let handle = mesh_pool.get_atom_mesh(&mut meshes, entity_info.element, batch_lod);
        *mesh_handle = handle;
    }

    mesh_pool.set_current_lod(batch_lod);
    diagnostics.current_lod = batch_lod;
}
