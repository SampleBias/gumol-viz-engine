//! Solvent-accessible molecular surface (coarse voxel shell, v0.2).

use crate::core::atom::AtomData;
use crate::core::visualization::{RenderMode, VisualizationConfig};
use crate::rendering::atom_index::InstancedAtomIndex;
use crate::rendering::instanced::{
    InstancedAtomEntity, InstancedAtomMesh, InstancedAtomsSpawnedEvent,
};
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;

const RENDER_ASSET_USAGES: RenderAssetUsages = RenderAssetUsages::RENDER_WORLD;

/// Standard water probe radius (Å).
pub const DEFAULT_PROBE_RADIUS: f32 = 1.4;

/// Max grid dimension per axis (caps mesh build cost).
const MAX_GRID_DIM: usize = 56;

#[derive(Component)]
pub struct MolecularSurface;

#[derive(Resource, Default, Debug)]
pub struct SurfaceEntities {
    pub entity: Option<Entity>,
}

/// Compute grid spacing from system size and atom count.
pub fn grid_spacing_for_system(num_atoms: usize, bbox_diagonal: f32) -> f32 {
    let target = if num_atoms > 20_000 {
        40
    } else if num_atoms > 2_000 {
        48
    } else {
        MAX_GRID_DIM
    };
    (bbox_diagonal / target as f32).clamp(0.75, 2.5)
}

/// Build a solvent-accessible surface mesh (union of VdW + probe spheres, voxel shell).
pub fn build_solvent_accessible_surface(
    atoms: &[AtomData],
    positions: &std::collections::HashMap<u32, Vec3>,
    spacing: f32,
    probe: f32,
) -> Mesh {
    let spheres: Vec<(Vec3, f32)> = atoms
        .iter()
        .filter_map(|a| {
            positions
                .get(&a.id)
                .map(|p| (*p, a.element.vdw_radius() + probe))
        })
        .collect();

    build_voxel_shell_mesh(&spheres, spacing)
}

fn build_voxel_shell_mesh(spheres: &[(Vec3, f32)], spacing: f32) -> Mesh {
    if spheres.is_empty() {
        return empty_mesh();
    }

    let mut min = Vec3::splat(f32::MAX);
    let mut max = Vec3::splat(f32::MIN);
    for (center, radius) in spheres {
        min = min.min(*center - Vec3::splat(*radius));
        max = max.max(*center + Vec3::splat(*radius));
    }

    let mut step = spacing.max(0.5);
    let mut nx = ((max.x - min.x) / step).ceil() as usize + 1;
    let mut ny = ((max.y - min.y) / step).ceil() as usize + 1;
    let mut nz = ((max.z - min.z) / step).ceil() as usize + 1;

    while nx.max(ny).max(nz) > MAX_GRID_DIM {
        step *= 1.15;
        nx = ((max.x - min.x) / step).ceil() as usize + 1;
        ny = ((max.y - min.y) / step).ceil() as usize + 1;
        nz = ((max.z - min.z) / step).ceil() as usize + 1;
    }

    let inside = compute_inside_field(nx, ny, nz, min, step, spheres);
    mesh_from_voxel_shell(&inside, nx, ny, nz, min, step)
}

fn compute_inside_field(
    nx: usize,
    ny: usize,
    nz: usize,
    origin: Vec3,
    step: f32,
    spheres: &[(Vec3, f32)],
) -> Vec<bool> {
    let mut inside = vec![false; nx * ny * nz];
    for iz in 0..nz {
        for iy in 0..ny {
            for ix in 0..nx {
                let p = origin + Vec3::new(ix as f32 * step, iy as f32 * step, iz as f32 * step);
                let idx = ix + nx * (iy + ny * iz);
                inside[idx] = spheres.iter().any(|(c, r)| p.distance(*c) <= *r);
            }
        }
    }
    inside
}

fn mesh_from_voxel_shell(
    inside: &[bool],
    nx: usize,
    ny: usize,
    nz: usize,
    origin: Vec3,
    step: f32,
) -> Mesh {
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    let idx = |x: usize, y: usize, z: usize| x + nx * (y + ny * z);
    let mut push_quad = |a: Vec3, b: Vec3, c: Vec3, d: Vec3, normal: Vec3| {
        let base = positions.len() as u32;
        positions.push(a.to_array());
        positions.push(b.to_array());
        positions.push(c.to_array());
        positions.push(d.to_array());
        let n = normal.normalize().to_array();
        normals.extend(std::iter::repeat(n).take(4));
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    };

    for iz in 0..nz {
        for iy in 0..ny {
            for ix in 0..nx {
                if !inside[idx(ix, iy, iz)] {
                    continue;
                }
                let p0 = origin + Vec3::new(ix as f32 * step, iy as f32 * step, iz as f32 * step);
                let p1 = p0 + Vec3::new(step, 0.0, 0.0);
                let p2 = p0 + Vec3::new(step, step, 0.0);
                let p3 = p0 + Vec3::new(0.0, step, 0.0);
                let p4 = p0 + Vec3::new(0.0, 0.0, step);
                let p5 = p1 + Vec3::new(0.0, 0.0, step);
                let p6 = p2 + Vec3::new(0.0, 0.0, step);
                let p7 = p3 + Vec3::new(0.0, 0.0, step);

                if ix == 0 || !inside[idx(ix - 1, iy, iz)] {
                    push_quad(p0, p3, p7, p4, Vec3::NEG_X);
                }
                if ix + 1 >= nx || !inside[idx(ix + 1, iy, iz)] {
                    push_quad(p1, p5, p6, p2, Vec3::X);
                }
                if iy == 0 || !inside[idx(ix, iy - 1, iz)] {
                    push_quad(p0, p4, p5, p1, Vec3::NEG_Y);
                }
                if iy + 1 >= ny || !inside[idx(ix, iy + 1, iz)] {
                    push_quad(p3, p2, p6, p7, Vec3::Y);
                }
                if iz == 0 || !inside[idx(ix, iy, iz - 1)] {
                    push_quad(p0, p1, p2, p3, Vec3::NEG_Z);
                }
                if iz + 1 >= nz || !inside[idx(ix, iy, iz + 1)] {
                    push_quad(p4, p7, p6, p5, Vec3::Z);
                }
            }
        }
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RENDER_ASSET_USAGES);
    if positions.is_empty() {
        return mesh;
    }
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

fn empty_mesh() -> Mesh {
    Mesh::new(PrimitiveTopology::TriangleList, RENDER_ASSET_USAGES)
}

pub fn clear_surface_on_load(
    mut commands: Commands,
    mut surface_entities: ResMut<SurfaceEntities>,
    file_loaded_events: EventReader<crate::systems::loading::FileLoadedEvent>,
) {
    if file_loaded_events.is_empty() {
        return;
    }
    if let Some(entity) = surface_entities.entity.take() {
        commands.entity(entity).despawn_recursive();
    }
}

#[allow(clippy::too_many_arguments)]
pub fn spawn_surface_on_load(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    sim_data: Res<crate::systems::loading::SimulationData>,
    index: Res<InstancedAtomIndex>,
    instanced: Query<(&InstancedAtomEntity, &InstancedAtomMesh)>,
    mut surface_entities: ResMut<SurfaceEntities>,
    spawned_events: EventReader<InstancedAtomsSpawnedEvent>,
) {
    if spawned_events.is_empty() || surface_entities.entity.is_some() {
        return;
    }
    if !sim_data.loaded || sim_data.atom_data.is_empty() {
        return;
    }

    let positions = index.collect_positions(&instanced);
    if positions.is_empty() {
        return;
    }

    let mut min = Vec3::splat(f32::MAX);
    let mut max = Vec3::splat(f32::MIN);
    for pos in positions.values() {
        min = min.min(*pos);
        max = max.max(*pos);
    }
    let diagonal = (max - min).length();
    let spacing = grid_spacing_for_system(sim_data.num_atoms(), diagonal);

    let mesh = build_solvent_accessible_surface(
        &sim_data.atom_data,
        &positions,
        spacing,
        DEFAULT_PROBE_RADIUS,
    );

    let handle = meshes.add(mesh);
    let material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.55, 0.72, 0.95, 0.55),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.35,
        double_sided: true,
        cull_mode: None,
        ..default()
    });

    let entity = commands
        .spawn((
            PbrBundle {
                mesh: handle,
                material,
                visibility: Visibility::Hidden,
                ..default()
            },
            MolecularSurface,
        ))
        .id();

    surface_entities.entity = Some(entity);
    info!(
        "Spawned molecular surface (spacing {:.2} Å, {} atoms)",
        spacing,
        sim_data.num_atoms()
    );
}

pub fn update_surface_visibility(
    config: Res<VisualizationConfig>,
    surface_entities: Res<SurfaceEntities>,
    mut visibility_query: Query<&mut Visibility, With<MolecularSurface>>,
) {
    if !config.is_changed() {
        return;
    }
    let show = config.render_mode == RenderMode::Surface;
    if let Some(entity) = surface_entities.entity {
        if let Ok(mut vis) = visibility_query.get_mut(entity) {
            *vis = if show {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
    }
}

pub fn register(app: &mut App) {
    app.init_resource::<SurfaceEntities>();
    info!("Molecular surface module registered");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::atom::{AtomData, Element};

    #[test]
    fn test_water_surface_has_geometry() {
        let atoms = vec![
            AtomData::new(0, Element::O, 0, "HOH".into(), "A".into(), "O".into()),
            AtomData::new(1, Element::H, 0, "HOH".into(), "A".into(), "H1".into()),
            AtomData::new(2, Element::H, 0, "HOH".into(), "A".into(), "H2".into()),
        ];
        let mut positions = std::collections::HashMap::new();
        positions.insert(0, Vec3::ZERO);
        positions.insert(1, Vec3::new(0.757, 0.0, 0.0));
        positions.insert(2, Vec3::new(-0.757, 0.0, 0.0));

        let mesh = build_solvent_accessible_surface(&atoms, &positions, 0.5, DEFAULT_PROBE_RADIUS);
        let verts = mesh.attribute(Mesh::ATTRIBUTE_POSITION).expect("positions");
        assert!(verts.len() > 3, "water SAS should produce vertices");
    }
}
