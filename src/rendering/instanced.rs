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
use crate::performance::PerformanceSettings;
use crate::rendering::atom_index::InstancedAtomIndex;
use crate::rendering::mesh_pool::AtomMeshPool;
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
    renderer::{RenderDevice, RenderQueue},
    view::{ExtractedView, NoFrustumCulling},
    MainWorld, Render, RenderApp, RenderSet,
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

/// Maximum draw calls for instanced atom rendering (one per periodic-table element).
pub const MAX_INSTANCED_DRAW_CALLS: usize = 118;

/// Estimate instanced draw calls from atom data (unique elements present).
pub fn estimate_instanced_draw_calls(atom_data: &[AtomData]) -> usize {
    use std::collections::HashSet;
    atom_data
        .iter()
        .map(|a| a.element)
        .collect::<HashSet<_>>()
        .len()
}

// ============================================================================
// APP-WORLD COMPONENTS / RESOURCES / EVENTS
// ============================================================================

/// Holds all instance data for one element.  Extracted to the render world
/// when `gpu_dirty` is set so the GPU buffer can be updated incrementally.
#[derive(Component, Clone, Debug)]
pub struct InstancedAtomMesh {
    pub instances: Vec<AtomInstanceData>,
    /// Cached visualization scale (before frustum culling).
    pub mode_scale: f32,
    /// When true, instance data must be re-uploaded to the GPU.
    pub gpu_dirty: bool,
}

impl InstancedAtomMesh {
    pub fn new(instances: Vec<AtomInstanceData>, mode_scale: f32) -> Self {
        Self {
            instances,
            mode_scale,
            gpu_dirty: true,
        }
    }

    pub fn mark_gpu_dirty(&mut self) {
        self.gpu_dirty = true;
    }
}

impl ExtractComponent for InstancedAtomMesh {
    type QueryData = &'static InstancedAtomMesh;
    type QueryFilter = ();
    type Out = Self;

    fn extract_component(item: QueryItem<'_, Self::QueryData>) -> Option<Self> {
        if !item.gpu_dirty {
            return None;
        }
        Some(InstancedAtomMesh {
            instances: item.instances.clone(),
            mode_scale: item.mode_scale,
            gpu_dirty: false,
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
    mesh_pool: &mut AtomMeshPool,
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
    let instance_scale = if show_atoms {
        mode_scale.max(0.001)
    } else {
        0.0
    };

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
        let lod = mesh_pool.current_lod();
        let mesh = mesh_pool.get_atom_mesh(meshes, *element, lod);
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
                InstancedAtomMesh::new(instances, instance_scale),
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
#[allow(clippy::too_many_arguments)]
pub fn spawn_instanced_atoms_on_load(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    sim_data: Res<crate::systems::loading::SimulationData>,
    viz_config: Res<VisualizationConfig>,
    mut file_loaded_events: EventReader<crate::systems::loading::FileLoadedEvent>,
    mut topology_events: EventReader<crate::systems::loading::TopologyAppliedEvent>,
    mut instanced_entities: ResMut<InstancedAtomEntities>,
    mut atom_index: ResMut<InstancedAtomIndex>,
    mut mesh_pool: ResMut<AtomMeshPool>,
    mut pick_entities: ResMut<crate::interaction::pick_proxy::PickProxyEntities>,
    perf: Res<PerformanceSettings>,
    mut diagnostics: ResMut<crate::performance::PerformanceDiagnostics>,
    mut spawned_event: EventWriter<InstancedAtomsSpawnedEvent>,
) {
    let file_loaded = file_loaded_events.read().next().is_some();
    let topology_applied = topology_events.read().next().is_some();
    let should_spawn = (file_loaded || topology_applied)
        && sim_data.loaded
        && !sim_data.atom_data.is_empty()
        && (instanced_entities.entities.is_empty() || topology_applied);

    if should_spawn {
        if let Some(first_frame) = sim_data.get_frame(0) {
            let (new_entities, ids_by_element) = spawn_atoms_instanced_internal(
                &mut commands,
                &mut meshes,
                &mut mesh_pool,
                &first_frame,
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

            let (pick_map, selection_ok) = crate::interaction::pick_proxy::spawn_pick_proxies(
                &mut commands,
                &mut meshes,
                &mut materials,
                &sim_data.atom_data,
                &positions,
                perf.max_pick_proxies,
            );
            pick_entities.entities = pick_map;
            diagnostics.selection_disabled = !selection_ok;
            diagnostics.selection_disabled_reason = if selection_ok {
                None
            } else {
                Some(format!(
                    "Selection disabled for systems with more than {} atoms",
                    perf.max_pick_proxies
                ))
            };

            diagnostics.estimated_bytes =
                crate::performance::memory::estimate_simulation_bytes(&sim_data);
            diagnostics.memory_warning = crate::performance::memory::memory_warning(&sim_data);

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
    mut mesh_pool: ResMut<AtomMeshPool>,
    mut pick_entities: ResMut<crate::interaction::pick_proxy::PickProxyEntities>,
    mut file_loaded_events: EventReader<crate::systems::loading::FileLoadedEvent>,
    mut topology_events: EventReader<crate::systems::loading::TopologyAppliedEvent>,
) {
    let reload = file_loaded_events.read().next().is_some()
        || topology_events.read().next().is_some();
    if !reload {
        return;
    }

    mesh_pool.clear();
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

/// System: center the camera on the molecule after loading.
pub fn center_camera_on_file_load_instanced(
    mut file_loaded_events: EventReader<crate::systems::loading::FileLoadedEvent>,
    sim_data: Res<crate::systems::loading::SimulationData>,
    mut camera_query: Query<&mut bevy_panorbit_camera::PanOrbitCamera>,
) {
    if file_loaded_events.read().next().is_none() {
        return;
    }

    if let Some(frame) = sim_data.get_frame(0) {
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
    frames: Res<crate::systems::frame_cache::TimelineFrames>,
    gpu_active: Res<crate::rendering::gpu_interpolation::GpuInterpolationActive>,
    perf: Res<crate::performance::PerformanceSettings>,
    index: Res<InstancedAtomIndex>,
    mut instanced_query: Query<(&InstancedAtomEntity, &mut InstancedAtomMesh)>,
) {
    if gpu_active.0 && perf.gpu_interpolation_enabled {
        return;
    }

    if !timeline.is_changed() && !frames.is_changed() {
        return;
    }

    if !sim_data.loaded || sim_data.num_frames() == 0 {
        return;
    }

    let Some(current_frame) = frames.current.as_ref() else {
        return;
    };

    let next_frame = if timeline.interpolate && timeline.interpolation_factor > 0.0 {
        frames.next.as_ref()
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
        mesh.mark_gpu_dirty();
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
    let instance_scale = if show_atoms {
        mode_scale.max(0.001)
    } else {
        0.0
    };

    for mut mesh in instanced_query.iter_mut() {
        mesh.mode_scale = instance_scale;
        for instance in mesh.instances.iter_mut() {
            instance.scale = instance_scale;
        }
        mesh.mark_gpu_dirty();
    }
}

/// Update instance colors from the active color scheme and selection state.
pub fn update_instanced_atom_colors(
    viz_config: Res<VisualizationConfig>,
    selection: Res<SelectionState>,
    sim_data: Res<crate::systems::loading::SimulationData>,
    timeline: Res<TimelineState>,
    index: Res<InstancedAtomIndex>,
    mut instanced_query: Query<(&InstancedAtomEntity, &mut InstancedAtomMesh)>,
) {
    if !viz_config.is_changed() && !selection.is_changed() && !sim_data.is_changed() {
        return;
    }

    let ctx = sim_data.color_context(timeline.current_frame);
    let selected: std::collections::HashSet<u32> =
        selection.selected_atom_ids.iter().copied().collect();

    for (entity_info, mut mesh) in instanced_query.iter_mut() {
        let Some(atom_ids) = index.element_atom_ids.get(&entity_info.element) else {
            continue;
        };

        for (i, &atom_id) in atom_ids.iter().enumerate() {
            if i >= mesh.instances.len() {
                break;
            }

            let color = if selected.contains(&atom_id) {
                Color::srgb(1.0, 1.0, 0.0)
            } else if let Some(atom) = sim_data.atom_data.iter().find(|a| a.id == atom_id) {
                viz_config.color_scheme.atom_color(atom, &ctx)
            } else {
                let rgb = entity_info.element.cpk_color();
                Color::srgb(rgb[0], rgb[1], rgb[2])
            };

            mesh.instances[i].color = color.to_linear().to_vec4();
        }
        mesh.mark_gpu_dirty();
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
            .add_systems(ExtractSchedule, clear_instanced_gpu_dirty)
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

// --- Clear dirty flags after extract ---

fn clear_instanced_gpu_dirty(mut main_world: ResMut<MainWorld>) {
    let world = main_world.as_mut();
    let mut query = world.query::<&mut InstancedAtomMesh>();
    for mut mesh in query.iter_mut(world) {
        mesh.gpu_dirty = false;
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
    query: Query<(Entity, &InstancedAtomMesh), Changed<InstancedAtomMesh>>,
    existing: Query<&InstanceBuffer>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    for (entity, instance_data) in &query {
        if instance_data.instances.is_empty() {
            continue;
        }

        let bytes = bytemuck::cast_slice(&instance_data.instances);
        let length = instance_data.instances.len();

        if let Ok(existing_buffer) = existing.get(entity) {
            if existing_buffer.length == length {
                render_queue.write_buffer(&existing_buffer.buffer, 0, bytes);
                continue;
            }
        }

        let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("instanced_atom_buffer"),
            contents: bytes,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        });

        commands
            .entity(entity)
            .insert(InstanceBuffer { buffer, length });
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
    use crate::core::atom::AtomData;

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
    fn test_estimate_draw_calls_bounded() {
        let atoms: Vec<AtomData> = (0..100_000)
            .map(|i| {
                AtomData::new(
                    i,
                    if i % 3 == 0 {
                        Element::C
                    } else if i % 3 == 1 {
                        Element::H
                    } else {
                        Element::O
                    },
                    0,
                    "UNK".into(),
                    "A".into(),
                    format!("A{i}"),
                )
            })
            .collect();
        let draw_calls = estimate_instanced_draw_calls(&atoms);
        assert_eq!(draw_calls, 3);
        assert!(draw_calls <= MAX_INSTANCED_DRAW_CALLS);
    }

    #[test]
    fn test_instanced_atom_mesh_dirty_flag() {
        let mesh = InstancedAtomMesh::new(vec![], 1.0);
        assert!(mesh.gpu_dirty);
    }
}
