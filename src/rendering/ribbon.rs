//! Protein backbone ribbon / tube / trace rendering.

use crate::core::molecule::SecondaryStructure;
use crate::core::secondary_structure::{BackboneResidue, ProteinBackbone};
use crate::core::visualization::{ColorPalette, RenderMode, VisualizationConfig};
use crate::rendering::atom_index::InstancedAtomIndex;
use crate::rendering::instanced::{InstancedAtomEntity, InstancedAtomMesh, InstancedAtomsSpawnedEvent};
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;

const RENDER_ASSET_USAGES: RenderAssetUsages = RenderAssetUsages::RENDER_WORLD;

/// Marker on ribbon mesh entities.
#[derive(Component)]
pub struct ProteinRibbon;

#[derive(Resource, Default, Debug)]
pub struct RibbonEntities {
    pub entities: Vec<Entity>,
}

/// Build backbone from loaded simulation data after instanced atoms spawn.
pub fn build_backbone_on_load(
    mut backbone: ResMut<ProteinBackbone>,
    sim_data: Res<crate::systems::loading::SimulationData>,
    index: Res<InstancedAtomIndex>,
    instanced: Query<(&InstancedAtomEntity, &InstancedAtomMesh)>,
    spawned_events: EventReader<InstancedAtomsSpawnedEvent>,
) {
    if spawned_events.is_empty() {
        return;
    }

    if !sim_data.loaded {
        return;
    }

    let positions = index.collect_positions(&instanced);
    *backbone = crate::core::secondary_structure::build_protein_backbone(
        &sim_data.atom_data,
        &positions,
    );

    if backbone.cartoon_available {
        info!(
            "Protein backbone: {} CA residues (cartoon modes available)",
            backbone.ca_count
        );
    }
}

/// Spawn ribbon mesh entities (hidden until cartoon/tube/trace mode is selected).
pub fn spawn_ribbon_on_load(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    backbone: Res<ProteinBackbone>,
    viz_config: Res<VisualizationConfig>,
    mut ribbon_entities: ResMut<RibbonEntities>,
    spawned_events: EventReader<InstancedAtomsSpawnedEvent>,
) {
    if spawned_events.is_empty() || !ribbon_entities.entities.is_empty() {
        return;
    }

    if !backbone.cartoon_available || backbone.residues.len() < 2 {
        return;
    }

    let mesh = meshes.add(build_ribbon_mesh(
        &backbone.residues,
        viz_config.render_mode,
    ));
    let material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
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
            ProteinRibbon,
        ))
        .id();

    ribbon_entities.entities.push(entity);
    info!("Spawned protein ribbon mesh ({} residues)", backbone.residues.len());
}

/// Rebuild ribbon geometry when render mode switches between cartoon/tube/trace.
pub fn update_ribbon_for_mode(
    viz_config: Res<VisualizationConfig>,
    backbone: Res<ProteinBackbone>,
    mut ribbon_entities: ResMut<RibbonEntities>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mesh_query: Query<&Handle<Mesh>, With<ProteinRibbon>>,
) {
    if !viz_config.is_changed() || ribbon_entities.entities.is_empty() {
        return;
    }

    if !backbone.cartoon_available {
        return;
    }

    let mode = viz_config.render_mode;
    if !mode.shows_ribbon() {
        return;
    }

    for entity in &ribbon_entities.entities {
        let Ok(handle) = mesh_query.get(*entity) else {
            continue;
        };
        if let Some(mesh) = meshes.get_mut(handle) {
            *mesh = build_ribbon_mesh(&backbone.residues, mode);
        }
    }
}

/// Update ribbon positions when timeline advances.
pub fn update_ribbon_positions(
    backbone: Res<ProteinBackbone>,
    index: Res<InstancedAtomIndex>,
    viz_config: Res<VisualizationConfig>,
    timeline: Res<crate::core::trajectory::TimelineState>,
    instanced: Query<(&InstancedAtomEntity, &InstancedAtomMesh)>,
    mut ribbon_entities: ResMut<RibbonEntities>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mesh_query: Query<&Handle<Mesh>, With<ProteinRibbon>>,
) {
    if ribbon_entities.entities.is_empty() || !backbone.cartoon_available {
        return;
    }

    if !timeline.is_changed() && !index.is_changed() {
        return;
    }

    let mut residues = backbone.residues.clone();
    for residue in &mut residues {
        if let Some(pos) = index.get_position(residue.ca_atom_id, &instanced) {
            residue.position = pos;
        }
    }

    let mesh_data = build_ribbon_mesh(&residues, viz_config.render_mode);
    for entity in &ribbon_entities.entities {
        let Ok(handle) = mesh_query.get(*entity) else {
            continue;
        };
        if let Some(mesh) = meshes.get_mut(handle) {
            *mesh = mesh_data.clone();
        }
    }
}

pub fn update_ribbon_visibility(
    viz_config: Res<VisualizationConfig>,
    backbone: Res<ProteinBackbone>,
    mut ribbon_query: Query<&mut Visibility, With<ProteinRibbon>>,
) {
    if !viz_config.is_changed() {
        return;
    }

    let show = viz_config.render_mode.shows_ribbon() && backbone.cartoon_available;

    for mut visibility in ribbon_query.iter_mut() {
        *visibility = if show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

pub fn clear_ribbon_on_load(
    mut commands: Commands,
    mut ribbon_entities: ResMut<RibbonEntities>,
    mut backbone: ResMut<ProteinBackbone>,
    file_loaded_events: EventReader<crate::systems::loading::FileLoadedEvent>,
) {
    if file_loaded_events.is_empty() {
        return;
    }

    for entity in ribbon_entities.entities.drain(..) {
        commands.entity(entity).despawn_recursive();
    }
    backbone.clear();
}

/// Catmull-Rom interpolation between four control points.
fn catmull_rom(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, t: f32) -> Vec3 {
    let t2 = t * t;
    let t3 = t2 * t;
    0.5 * ((2.0 * p1)
        + (-p0 + p2) * t
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
        + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3)
}

fn build_spline_points(control: &[Vec3], subdivisions: usize) -> Vec<Vec3> {
    if control.len() < 2 {
        return control.to_vec();
    }

    let mut out = Vec::new();
    for i in 0..control.len() - 1 {
        let p0 = if i == 0 { control[0] } else { control[i - 1] };
        let p1 = control[i];
        let p2 = control[i + 1];
        let p3 = if i + 2 < control.len() {
            control[i + 2]
        } else {
            control[i + 1]
        };

        for s in 0..subdivisions {
            let t = s as f32 / subdivisions as f32;
            out.push(catmull_rom(p0, p1, p2, p3, t));
        }
    }
    out.push(*control.last().unwrap());
    out
}

fn residue_color(residue: &BackboneResidue) -> Color {
    ColorPalette::secondary_structure_color(residue.secondary_structure)
}

fn ribbon_width(ss: SecondaryStructure, mode: RenderMode) -> f32 {
    match mode {
        RenderMode::Trace => 0.08,
        RenderMode::Tube => match ss {
            SecondaryStructure::AlphaHelix => 0.35,
            SecondaryStructure::BetaStrand | SecondaryStructure::BetaSheet => 0.3,
            _ => 0.25,
        },
        RenderMode::Cartoon => match ss {
            SecondaryStructure::AlphaHelix => 0.5,
            SecondaryStructure::BetaStrand | SecondaryStructure::BetaSheet => 1.2,
            _ => 0.35,
        },
        _ => 0.3,
    }
}

/// Build ribbon/tube/trace mesh along the CA backbone.
pub fn build_ribbon_mesh(residues: &[BackboneResidue], mode: RenderMode) -> Mesh {
    if residues.len() < 2 {
        return Mesh::new(PrimitiveTopology::TriangleList, RENDER_ASSET_USAGES);
    }

    let control: Vec<Vec3> = residues.iter().map(|r| r.position).collect();
    let spline = build_spline_points(&control, 4);

    if mode == RenderMode::Trace {
        return build_trace_mesh(&spline, residues);
    }

    build_tube_or_cartoon_mesh(&spline, residues, mode)
}

fn build_trace_mesh(spline: &[Vec3], residues: &[BackboneResidue]) -> Mesh {
    let vertex_count = spline.len();
    let positions: Vec<[f32; 3]> = spline
        .iter()
        .map(|p| [p.x, p.y, p.z])
        .collect();
    let indices: Vec<u32> = (0..positions.len() as u32).collect();

    let mut mesh = Mesh::new(PrimitiveTopology::LineStrip, RENDER_ASSET_USAGES);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions.clone());
    mesh.insert_indices(Indices::U32(indices));

    let color = if residues.is_empty() {
        Color::WHITE
    } else {
        residue_color(&residues[0])
    };
    let colors = vec![[color.to_srgba().red, color.to_srgba().green, color.to_srgba().blue, color.to_srgba().alpha]; vertex_count];
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh
}

fn build_tube_or_cartoon_mesh(
    spline: &[Vec3],
    residues: &[BackboneResidue],
    mode: RenderMode,
) -> Mesh {
    let segments = 8;
    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();

    for i in 0..spline.len() {
        let point = spline[i];
        let tangent = if i + 1 < spline.len() {
            (spline[i + 1] - spline[i]).normalize_or_zero()
        } else if i > 0 {
            (spline[i] - spline[i - 1]).normalize_or_zero()
        } else {
            Vec3::X
        };

        let up = if tangent.dot(Vec3::Y).abs() > 0.99 {
            Vec3::X
        } else {
            Vec3::Y
        };
        let normal_vec = tangent.cross(up).normalize_or_zero();
        let binormal = tangent.cross(normal_vec).normalize_or_zero();

        let residue_idx = ((i as f32 / spline.len() as f32) * residues.len() as f32) as usize;
        let residue_idx = residue_idx.min(residues.len().saturating_sub(1));
        let ss = residues[residue_idx].secondary_structure;
        let width = ribbon_width(ss, mode);
        let height = if mode == RenderMode::Cartoon
            && matches!(ss, SecondaryStructure::BetaStrand | SecondaryStructure::BetaSheet)
        {
            width * 0.25
        } else {
            width
        };

        let color = residue_color(&residues[residue_idx]);
        let rgba = [
            color.to_srgba().red,
            color.to_srgba().green,
            color.to_srgba().blue,
            color.to_srgba().alpha,
        ];

        for s in 0..segments {
            let angle = 2.0 * std::f32::consts::PI * s as f32 / segments as f32;
            let offset = normal_vec * angle.cos() * width + binormal * angle.sin() * height;
            let v = point + offset;
            vertices.push([v.x, v.y, v.z]);
            normals.push([offset.x, offset.y, offset.z]);
            colors.push(rgba);
        }
    }

    for i in 0..spline.len() - 1 {
        for s in 0..segments {
            let i0 = (i * segments + s) as u32;
            let i1 = (i * segments + (s + 1) % segments) as u32;
            let i2 = ((i + 1) * segments + s) as u32;
            let i3 = ((i + 1) * segments + (s + 1) % segments) as u32;

            indices.extend_from_slice(&[i0, i2, i1, i1, i2, i3]);
        }
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RENDER_ASSET_USAGES);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

pub fn register(app: &mut App) {
    app.init_resource::<ProteinBackbone>()
        .init_resource::<RibbonEntities>();
    info!("Ribbon rendering module registered");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::secondary_structure::BackboneResidue;

    fn sample_residues() -> Vec<BackboneResidue> {
        (0..5)
            .map(|i| BackboneResidue {
                residue_id: i + 1,
                ca_atom_id: i,
                chain_id: "A".into(),
                position: Vec3::new(i as f32 * 3.8, 0.0, 0.0),
                secondary_structure: SecondaryStructure::Coil,
            })
            .collect()
    }

    #[test]
    fn test_build_ribbon_mesh_has_vertices() {
        let mesh = build_ribbon_mesh(&sample_residues(), RenderMode::Cartoon);
        assert!(mesh.count_vertices() > 0);
    }

    #[test]
    fn test_build_trace_mesh_line_strip() {
        let mesh = build_ribbon_mesh(&sample_residues(), RenderMode::Trace);
        assert!(mesh.count_vertices() >= 2);
    }
}
