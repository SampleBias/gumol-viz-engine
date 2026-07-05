//! CPU-side frustum culling for instanced atom spheres.

use crate::core::visualization::VisualizationConfig;
use crate::performance::{PerformanceDiagnostics, PerformanceSettings};
use crate::rendering::instanced::{InstancedAtomEntity, InstancedAtomMesh};
use bevy::prelude::*;
use bevy::render::{
    camera::CameraProjection,
    primitives::{Aabb, Frustum},
};

/// Hide off-screen instances by setting their scale to zero.
pub fn cull_instanced_atoms(
    perf: Res<PerformanceSettings>,
    viz: Res<VisualizationConfig>,
    mut diagnostics: ResMut<PerformanceDiagnostics>,
    camera: Query<(&Camera, &GlobalTransform, &Projection)>,
    mut instanced: Query<(&InstancedAtomEntity, &mut InstancedAtomMesh)>,
) {
    let mode_scale = viz.render_mode.atom_scale() * viz.atom_scale;
    let show_atoms = viz.show_atoms && viz.render_mode.shows_atoms();
    let base_scale = if show_atoms { mode_scale.max(0.001) } else { 0.0 };

    let Ok((camera, transform, projection)) = camera.get_single() else {
        return;
    };

    let view = transform.affine().inverse();
    let clip_from_view = projection.get_clip_from_view();
    let clip_from_world = clip_from_view * view;
    let frustum = Frustum::from_clip_from_world(&clip_from_world);

    let mut visible = 0usize;
    let mut culled = 0usize;

    for (entity_info, mut mesh) in instanced.iter_mut() {
        mesh.mode_scale = base_scale;
        let world_radius = entity_info.element.vdw_radius() * 0.5 * base_scale;

        for instance in mesh.instances.iter_mut() {
            if base_scale <= 0.0 {
                instance.scale = 0.0;
                continue;
            }

            let visible_instance = if perf.frustum_culling_enabled {
                let center = instance.position;
                let aabb = Aabb::from_min_max(
                    center - Vec3::splat(world_radius),
                    center + Vec3::splat(world_radius),
                );
                frustum.intersects_obb(&aabb, &transform.affine(), true, false)
            } else {
                true
            };

            instance.scale = if visible_instance { base_scale } else { 0.0 };

            if visible_instance {
                visible += 1;
            } else {
                culled += 1;
            }
        }
    }

    diagnostics.visible_instance_count = visible;
    diagnostics.culled_instance_count = culled;
    let _ = camera;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frustum_type_exists() {
        let _ = std::any::type_name::<Frustum>();
    }
}
