//! GPU-instanced atom rendering pipeline.
//!
//! Groups atoms by element and renders all atoms of the same element in a
//! single draw call (one entity per element ≈ 118 draw calls instead of N).
//! Uses a custom WGSL shader with per-instance position, scale, and color
//! stored in a vertex buffer with `VertexStepMode::Instance`.

use crate::core::atom::{AtomData, Element};
use crate::core::trajectory::{FrameData, TimelineState};
use crate::core::visualization::VisualizationConfig;
use crate::interaction::selection::SelectionState;
use crate::rendering::atom_index::InstancedAtomIndex;
use crate::rendering::generate_atom_mesh;
use bevy::core_pipeline::core_3d::Transparent3d;
use bevy::ecs::{query::QueryItem, system::SystemParamItem};
use bevy::pbr::{
    MeshPipeline, MeshPipelineKey, RenderMeshInstances, SetMeshBindGroup, SetMeshViewBindGroup,
};
use bevy::prelude::*;
use bevy::render::{
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    mesh::{GpuBufferInfo, GpuMesh, MeshVertexBufferLayoutRef},
    render_asset::RenderAssets,
    render_phase::{
        AddRenderCommand, DrawFunctions, PhaseItem, PhaseItemExtraIndex, RenderCommand,
        RenderCommandResult, SetItemPipeline, TrackedRenderPass, ViewSortedRenderPhases,
    },
    render_resource::*,
    renderer::RenderDevice,
    view::{ExtractedView, NoFrustumCulling},
    Render, RenderApp, RenderSet,
};
use bytemuck::{Pod, Zeroable};
use std::collections::HashMap;

// ============================================================================
// INSTANCE DATA
// ============================================================================

/// Per-atom data packed into a GPU vertex buffer (32 bytes, tightly packed).
#[derive(Clone, Copy, Pod, Zeroable, Debug, PartialEq)]
#[repr(C)]
pub struct AtomInstanceData {
    pub position: Vec3,
    pub scale: f32,
    pub color: Vec4,
}

// ============================================================================
// APP-WORLD COMPONENTS / RESOURCES / EVENTS
// ============================================================================

/// Holds all instance data for one element.  Extracted to the render world
/// each frame so the GPU buffer can be rebuilt when positions change.
#[derive(Component, Clone, Debug)]
pub struct InstancedAtomMesh {
    pub instances: Vec<AtomInstanceData>,
}

impl ExtractComponent for InstancedAtomMesh {
    type QueryData = &'static InstancedAtomMesh;
    type QueryFilter = ();
    type Out = Self;

    fn extract_component(item: QueryItem<'_, Self::QueryData>) -> Option<Self> {
        Some(InstancedAtomMesh {
            instances: item.instances.clone(),
        })
    }
}

/// Marker identifying which element an instanced entity represents.
#[derive(Component)]
pub struct InstancedAtomEntity {
    pub element: Element,
}

/// Tracks per-element entities and total atom count.
#[derive(Resource, Default, Debug)]
pub struct InstancedAtomEntities {
    pub entities: HashMap<Element, Entity>,
    pub total_atoms: usize,
}

/// Fired after instanced atoms are spawned.
#[derive(Event, Debug)]
pub struct InstancedAtomsSpawnedEvent {
    pub count: usize,
    pub draw_calls: usize,
}

// ============================================================================
// APP-WORLD SYSTEMS
// ============================================================================

/// Spawn one entity per element with instance data for all atoms of that element.
/// Returns entity map and atom IDs grouped by element (for index building).
pub fn spawn_atoms_instanced_internal(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    frame_data: &FrameData,
    atom_data: &[AtomData],
    viz_config: &VisualizationConfig,
) -> (HashMap<Element, Entity>, HashMap<Element, Vec<u32>>) {
    info!(
        "Spawning {} atoms with instanced rendering",
        atom_data.len()
    );

    let mode_scale = viz_config.render_mode.atom_scale() * viz_config.atom_scale;
    let show_atoms = viz_config.show_atoms && viz_config.render_mode.shows_atoms();
    let instance_scale = if show_atoms { mode_scale.max(0.001) } else { 0.0 };

    let mut atoms_by_element: HashMap<Element, Vec<&AtomData>> = HashMap::new();
    for atom_info in atom_data {
        if frame_data.get_position(atom_info.id).is_some() {
            atoms_by_element
                .entry(atom_info.element)
                .or_default()
                .push(atom_info);
        }
    }

    info!("Grouped into {} element types", atoms_by_element.len());

    let mut entity_map = HashMap::new();
    let mut ids_by_element: HashMap<Element, Vec<u32>> = HashMap::new();

    for (element, atoms) in &atoms_by_element {
        let radius = element.vdw_radius() * 0.5;
        let mesh = meshes.add(generate_atom_mesh(radius));
        let color_rgb = element.cpk_color();

        let mut element_ids = Vec::with_capacity(atoms.len());
        let instances: Vec<AtomInstanceData> = atoms
            .iter()
            .map(|a| {
                element_ids.push(a.id);
                let position = frame_data.get_position(a.id).unwrap_or(Vec3::ZERO);
                AtomInstanceData {
                    position,
                    scale: instance_scale,
                    color: Vec4::new(color_rgb[0], color_rgb[1], color_rgb[2], 1.0),
                }
            })
            .collect();

        ids_by_element.insert(*element, element_ids);

        let entity = commands
            .spawn((
                mesh,
                SpatialBundle::INHERITED_IDENTITY,
                InstancedAtomMesh { instances },
                InstancedAtomEntity { element: *element },
                NoFrustumCulling,
            ))
            .id();

        entity_map.insert(*element, entity);
    }

    info!(
        "Instanced: {} draw calls for {} atoms ({:.1}% reduction)",
        entity_map.len(),
        atom_data.len(),
        (1.0 - entity_map.len() as f64 / atom_data.len().max(1) as f64) * 100.0
    );

    (entity_map, ids_by_element)
}

/// System: spawn instanced atoms when a file finishes loading.
pub fn spawn_instanced_atoms_on_load(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    sim_data: Res<crate::systems::loading::SimulationData>,
    viz_config: Res<VisualizationConfig>,
    mut file_loaded_events: EventReader<crate::systems::loading::FileLoadedEvent>,
    mut instanced_entities: ResMut<InstancedAtomEntities>,
    mut atom_index: ResMut<InstancedAtomIndex>,
    mut pick_entities: ResMut<crate::interaction::pick_proxy::PickProxyEntities>,
    mut spawned_event: EventWriter<InstancedAtomsSpawnedEvent>,
) {
    if !instanced_entities.entities.is_empty() {
        return;
    }

    let should_spawn = file_loaded_events.read().next().is_some();

    if should_spawn && sim_data.loaded && !sim_data.atom_data.is_empty() {
        if let Some(first_frame) = sim_data.trajectory.get_frame(0) {
            let (new_entities, ids_by_element) = spawn_atoms_instanced_internal(
                &mut commands,
                &mut meshes,
                first_frame,
                &sim_data.atom_data,
                &viz_config,
            );

            let draw_calls = new_entities.len();
            instanced_entities.entities = new_entities;
            instanced_entities.total_atoms = sim_data.atom_data.len();
            *atom_index = InstancedAtomIndex::build(&ids_by_element);

            let positions: HashMap<u32, Vec3> = first_frame
                .positions
                .iter()
                .map(|(&id, &pos)| (id, pos))
                .collect();

            pick_entities.entities = crate::interaction::pick_proxy::spawn_pick_proxies(
                &mut commands,
                &mut meshes,
                &mut materials,
                &sim_data.atom_data,
                &positions,
            );

            spawned_event.send(InstancedAtomsSpawnedEvent {
                count: sim_data.atom_data.len(),
                draw_calls,
            });
        }
    }
}

/// Clear instanced atoms when a new file is loaded.
pub fn clear_instanced_atoms_on_load(
    mut commands: Commands,
    mut instanced_entities: ResMut<InstancedAtomEntities>,
    mut atom_index: ResMut<InstancedAtomIndex>,
    mut pick_entities: ResMut<crate::interaction::pick_proxy::PickProxyEntities>,
    file_loaded_events: EventReader<crate::systems::loading::FileLoadedEvent>,
) {
    if !file_loaded_events.is_empty() {
        if !pick_entities.entities.is_empty() {
            for (_, entity) in pick_entities.entities.drain() {
                commands.entity(entity).despawn_recursive();
            }
        }

        if !instanced_entities.entities.is_empty() {
            for (_, entity) in instanced_entities.entities.drain() {
                commands.entity(entity).despawn_recursive();
            }
            instanced_entities.total_atoms = 0;
            atom_index.clear();
            info!("Cleared instanced atoms for new file load");
        }
    }
}

/// System: center the camera on the molecule after loading.
pub fn center_camera_on_file_load_instanced(
    mut file_loaded_events: EventReader<crate::systems::loading::FileLoadedEvent>,
    sim_data: Res<crate::systems::loading::SimulationData>,
    mut camera_query: Query<&mut bevy_panorbit_camera::PanOrbitCamera>,
) {
    if file_loaded_events.read().next().is_none() {
        return;
    }

    if let Some(frame) = sim_data.trajectory.get_frame(0) {
        let mut sum = Vec3::ZERO;
        let mut count = 0;
        for pos in frame.positions.values() {
            sum += *pos;
            count += 1;
        }
        if count > 0 {
            let center = sum / count as f32;
            for mut cam in camera_query.iter_mut() {
                cam.focus = center;
                cam.target_focus = center;
                info!("Camera centered at {:?}", center);
            }
        }
    }
}

/// System: update instanced positions when the timeline frame changes.
pub fn update_instanced_positions_from_timeline(
    sim_data: Res<crate::systems::loading::SimulationData>,
    timeline: Res<TimelineState>,
    index: Res<InstancedAtomIndex>,
    mut instanced_query: Query<(&InstancedAtomEntity, &mut InstancedAtomMesh)>,
) {
    if !timeline.is_changed() || !sim_data.loaded || sim_data.num_frames() == 0 {
        return;
    }

    let current_frame = match sim_data.trajectory.get_frame(timeline.current_frame) {
        Some(f) => f,
        None => return,
    };

    let next_frame = if timeline.interpolate && timeline.interpolation_factor > 0.0 {
        let next_idx = (timeline.current_frame + 1).min(sim_data.num_frames() - 1);
        sim_data.trajectory.get_frame(next_idx)
    } else {
        None
    };

    for (entity_info, mut mesh) in instanced_query.iter_mut() {
        let element = entity_info.element;

        let Some(atom_ids) = index.element_atom_ids.get(&element) else {
            continue;
        };

        for (i, &atom_id) in atom_ids.iter().enumerate() {
            if i >= mesh.instances.len() {
                break;
            }

            let current_pos = match current_frame.get_position(atom_id) {
                Some(p) => p,
                None => continue,
            };

            let position = if let Some(nf) = next_frame {
                if let Some(next_pos) = nf.get_position(atom_id) {
                    current_pos.lerp(next_pos, timeline.interpolation_factor)
                } else {
                    current_pos
                }
            } else {
                current_pos
            };

            mesh.instances[i].position = position;
        }
    }
}

/// Update instance scales when visualization mode changes.
pub fn update_instanced_visualization(
    viz_config: Res<VisualizationConfig>,
    mut instanced_query: Query<&mut InstancedAtomMesh>,
) {
    if !viz_config.is_changed() {
        return;
    }

    let mode_scale = viz_config.render_mode.atom_scale() * viz_config.atom_scale;
    let show_atoms = viz_config.show_atoms && viz_config.render_mode.shows_atoms();
    let instance_scale = if show_atoms { mode_scale.max(0.001) } else { 0.0 };

    for mut mesh in instanced_query.iter_mut() {
        for instance in mesh.instances.iter_mut() {
            instance.scale = instance_scale;
        }
    }
}

/// Highlight selected atoms by tinting instance colors yellow.
pub fn update_instanced_selection_highlight(
    selection: Res<SelectionState>,
    sim_data: Res<crate::systems::loading::SimulationData>,
    index: Res<InstancedAtomIndex>,
    mut instanced_query: Query<(&InstancedAtomEntity, &mut InstancedAtomMesh)>,
) {
    if !selection.is_changed() {
        return;
    }

    let selected: std::collections::HashSet<u32> = selection.selected_atom_ids.iter().copied().collect();

    for (entity_info, mut mesh) in instanced_query.iter_mut() {
        let Some(atom_ids) = index.element_atom_ids.get(&entity_info.element) else {
            continue;
        };

        for (i, &atom_id) in atom_ids.iter().enumerate() {
            if i >= mesh.instances.len() {
                break;
            }

            let cpk = sim_data
                .atom_data
                .iter()
                .find(|a| a.id == atom_id)
                .map(|a| a.element.cpk_color())
                .unwrap_or(entity_info.element.cpk_color());

            if selected.contains(&atom_id) {
                mesh.instances[i].color = Vec4::new(1.0, 1.0, 0.0, 1.0);
            } else {
                mesh.instances[i].color = Vec4::new(cpk[0], cpk[1], cpk[2], 1.0);
            }
        }
    }
}

// ============================================================================
// RENDER PIPELINE
// ============================================================================

/// Bevy plugin that wires up the render-world side of instanced atom rendering.
struct InstancedAtomRenderPlugin;

impl Plugin for InstancedAtomRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<InstancedAtomMesh>::default());
        app.sub_app_mut(RenderApp)
            .add_render_command::<Transparent3d, DrawInstancedAtoms>()
            .init_resource::<SpecializedMeshPipelines<InstancedAtomPipeline>>()
            .add_systems(
                Render,
                (
                    queue_instanced_atoms.in_set(RenderSet::QueueMeshes),
                    prepare_instance_buffers.in_set(RenderSet::PrepareResources),
                ),
            );
    }

    fn finish(&self, app: &mut App) {
        app.sub_app_mut(RenderApp)
            .init_resource::<InstancedAtomPipeline>();
    }
}

// --- Pipeline resource ---

#[derive(Resource)]
struct InstancedAtomPipeline {
    shader: Handle<Shader>,
    mesh_pipeline: MeshPipeline,
}

impl FromWorld for InstancedAtomPipeline {
    fn from_world(world: &mut World) -> Self {
        let mesh_pipeline = world.resource::<MeshPipeline>().clone();
        InstancedAtomPipeline {
            shader: world
                .resource::<AssetServer>()
                .load("shaders/instanced_atom.wgsl"),
            mesh_pipeline,
        }
    }
}

impl SpecializedMeshPipeline for InstancedAtomPipeline {
    type Key = MeshPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayoutRef,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut descriptor = self.mesh_pipeline.specialize(key, layout)?;

        descriptor.vertex.shader = self.shader.clone();
        descriptor.fragment.as_mut().unwrap().shader = self.shader.clone();

        // Append instance vertex buffer (locations 3-5 complement the mesh's 0-2).
        descriptor.vertex.buffers.push(VertexBufferLayout {
            array_stride: std::mem::size_of::<AtomInstanceData>() as u64,
            step_mode: VertexStepMode::Instance,
            attributes: vec![
                // i_pos: vec3<f32>
                VertexAttribute {
                    format: VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 3,
                },
                // i_scale: f32
                VertexAttribute {
                    format: VertexFormat::Float32,
                    offset: VertexFormat::Float32x3.size(),
                    shader_location: 4,
                },
                // i_color: vec4<f32>
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: VertexFormat::Float32x3.size() + VertexFormat::Float32.size(),
                    shader_location: 5,
                },
            ],
        });

        Ok(descriptor)
    }
}

// --- Queue ---

#[allow(clippy::too_many_arguments)]
fn queue_instanced_atoms(
    transparent_3d_draw_functions: Res<DrawFunctions<Transparent3d>>,
    custom_pipeline: Res<InstancedAtomPipeline>,
    msaa: Res<Msaa>,
    mut pipelines: ResMut<SpecializedMeshPipelines<InstancedAtomPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    meshes: Res<RenderAssets<GpuMesh>>,
    render_mesh_instances: Res<RenderMeshInstances>,
    material_meshes: Query<Entity, With<InstancedAtomMesh>>,
    mut transparent_render_phases: ResMut<ViewSortedRenderPhases<Transparent3d>>,
    views: Query<Entity, With<ExtractedView>>,
) {
    let draw_custom = transparent_3d_draw_functions
        .read()
        .id::<DrawInstancedAtoms>();

    for view in &views {
        let Some(transparent_phase) = transparent_render_phases.get_mut(&view) else {
            continue;
        };

        for entity in &material_meshes {
            let Some(mesh_instance) = render_mesh_instances.render_mesh_queue_data(entity) else {
                continue;
            };
            let Some(mesh) = meshes.get(mesh_instance.mesh_asset_id) else {
                continue;
            };

            let key = MeshPipelineKey::from_msaa_samples(msaa.samples())
                | MeshPipelineKey::from_primitive_topology(mesh.primitive_topology());

            let pipeline = pipelines
                .specialize(&pipeline_cache, &custom_pipeline, key, &mesh.layout)
                .unwrap();

            transparent_phase.add(Transparent3d {
                entity,
                pipeline,
                draw_function: draw_custom,
                distance: 0.0,
                batch_range: 0..1,
                extra_index: PhaseItemExtraIndex::NONE,
            });
        }
    }
}

// --- Prepare GPU buffers ---

#[derive(Component)]
struct InstanceBuffer {
    buffer: Buffer,
    length: usize,
}

fn prepare_instance_buffers(
    mut commands: Commands,
    query: Query<(Entity, &InstancedAtomMesh)>,
    render_device: Res<RenderDevice>,
) {
    for (entity, instance_data) in &query {
        if instance_data.instances.is_empty() {
            continue;
        }

        let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("instanced_atom_buffer"),
            contents: bytemuck::cast_slice(&instance_data.instances),
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        });

        commands.entity(entity).insert(InstanceBuffer {
            buffer,
            length: instance_data.instances.len(),
        });
    }
}

// --- Draw command ---

type DrawInstancedAtoms = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMeshBindGroup<1>,
    DrawMeshInstanced,
);

struct DrawMeshInstanced;

impl<P: PhaseItem> RenderCommand<P> for DrawMeshInstanced {
    type Param = (
        Res<'static, RenderAssets<GpuMesh>>,
        Res<'static, RenderMeshInstances>,
    );
    type ViewQuery = ();
    type ItemQuery = &'static InstanceBuffer;

    #[inline]
    fn render<'w>(
        item: &P,
        _view: (),
        instance_buffer: Option<&'w InstanceBuffer>,
        (meshes, render_mesh_instances): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(mesh_instance) = render_mesh_instances
            .into_inner()
            .render_mesh_queue_data(item.entity())
        else {
            return RenderCommandResult::Failure;
        };
        let Some(gpu_mesh) = meshes.into_inner().get(mesh_instance.mesh_asset_id) else {
            return RenderCommandResult::Failure;
        };
        let Some(instance_buffer) = instance_buffer else {
            return RenderCommandResult::Failure;
        };

        pass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.slice(..));
        pass.set_vertex_buffer(1, instance_buffer.buffer.slice(..));

        match &gpu_mesh.buffer_info {
            GpuBufferInfo::Indexed {
                buffer,
                index_format,
                count,
            } => {
                pass.set_index_buffer(buffer.slice(..), 0, *index_format);
                pass.draw_indexed(0..*count, 0, 0..instance_buffer.length as u32);
            }
            GpuBufferInfo::NonIndexed => {
                pass.draw(0..gpu_mesh.vertex_count, 0..instance_buffer.length as u32);
            }
        }

        RenderCommandResult::Success
    }
}

// ============================================================================
// REGISTRATION
// ============================================================================

/// Register resources, events, and the render pipeline plugin.
/// App-world update systems are registered centrally in `systems::register`.
pub fn register(app: &mut App) {
    app.init_resource::<InstancedAtomEntities>()
        .init_resource::<InstancedAtomIndex>()
        .add_event::<InstancedAtomsSpawnedEvent>()
        .add_plugins(InstancedAtomRenderPlugin);

    info!("Instanced rendering plugin registered");
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atom_instance_data_size() {
        assert_eq!(
            std::mem::size_of::<AtomInstanceData>(),
            32,
            "AtomInstanceData must be exactly 32 bytes for the GPU vertex buffer"
        );
    }

    #[test]
    fn test_atom_instance_data_fields() {
        let instance = AtomInstanceData {
            position: Vec3::new(1.0, 2.0, 3.0),
            scale: 1.5,
            color: Vec4::new(0.5, 0.5, 0.5, 1.0),
        };
        assert_eq!(instance.position, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(instance.scale, 1.5);
        assert_eq!(instance.color.w, 1.0);
    }

    #[test]
    fn test_instanced_atom_entities_default() {
        let entities = InstancedAtomEntities::default();
        assert!(entities.entities.is_empty());
        assert_eq!(entities.total_atoms, 0);
    }

    #[test]
    fn test_bytemuck_cast() {
        let instances = vec![
            AtomInstanceData {
                position: Vec3::ZERO,
                scale: 1.0,
                color: Vec4::ONE,
            },
            AtomInstanceData {
                position: Vec3::X,
                scale: 2.0,
                color: Vec4::ZERO,
            },
        ];
        let bytes: &[u8] = bytemuck::cast_slice(&instances);
        assert_eq!(bytes.len(), 64);
    }
}
