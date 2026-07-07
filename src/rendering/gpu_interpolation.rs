//! GPU compute interpolation for timeline playback at scale.
//!
//! Uploads dense frame position buffers and runs a WGSL compute shader to
//! interpolate between frames. Results are read back and applied to instanced
//! atom meshes. Falls back to CPU lerp when disabled or unavailable.

use crate::core::atom::Element;
use crate::core::trajectory::TimelineState;
use crate::rendering::atom_index::InstancedAtomIndex;
use crate::rendering::instanced::{InstancedAtomEntity, InstancedAtomMesh};
use crate::systems::frame_cache::TimelineFrames;
use crate::systems::loading::SimulationData;
use bevy::prelude::*;
use bevy::render::{
    extract_resource::{ExtractResource, ExtractResourcePlugin},
    render_graph::{self, RenderGraph, RenderLabel},
    render_resource::{
        binding_types::{storage_buffer, uniform_buffer_sized},
        *,
    },
    renderer::{RenderContext, RenderDevice, RenderQueue},
    Render, RenderApp, RenderSet,
};
use bytemuck::{Pod, Zeroable};
use crossbeam_channel::{Receiver, Sender};
use std::borrow::Cow;
use std::collections::HashMap;
use std::num::NonZeroU64;

const SHADER_PATH: &str = "shaders/atom_interpolate.wgsl";
const WORKGROUP_SIZE: u32 = 256;

/// Maps dense position array indices to instanced atom locations.
#[derive(Resource, Default, Clone, Debug)]
pub struct DenseAtomLayout {
    /// Ordered atom IDs matching `SimulationData::atom_data` (dense index → atom_id).
    pub dense_atom_ids: Vec<u32>,
    /// atom_id → dense index for O(1) scatter after GPU readback.
    pub atom_id_to_dense: HashMap<u32, u32>,
}

impl DenseAtomLayout {
    pub fn build(atom_data: &[crate::core::atom::AtomData]) -> Self {
        let dense_atom_ids: Vec<u32> = atom_data.iter().map(|a| a.id).collect();
        let atom_id_to_dense = dense_atom_ids
            .iter()
            .enumerate()
            .map(|(i, &id)| (id, i as u32))
            .collect();
        Self {
            dense_atom_ids,
            atom_id_to_dense,
        }
    }

    pub fn clear(&mut self) {
        self.dense_atom_ids.clear();
        self.atom_id_to_dense.clear();
    }
}

/// Extracted timeline data for the render-world compute pass.
#[derive(Resource, Clone, ExtractResource, Default)]
pub struct GpuInterpolationExtract {
    pub active: bool,
    pub alpha: f32,
    pub num_atoms: u32,
    pub positions_a: Vec<Vec3>,
    pub positions_b: Vec<Vec3>,
    pub frames_changed: bool,
    pub last_current_index: usize,
}

/// Positions read back from the GPU (one frame of latency).
#[derive(Resource, Deref)]
pub struct GpuInterpolationReadback(Receiver<Vec<Vec3>>);

#[derive(Resource, Deref)]
struct GpuInterpolationSender(Sender<Vec<Vec3>>);

/// Whether the GPU compute path is active this frame.
#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct GpuInterpolationActive(pub bool);

/// Build dense position arrays from resolved timeline frames.
pub fn prepare_gpu_interpolation_extract(
    sim_data: Res<SimulationData>,
    timeline: Res<TimelineState>,
    frames: Res<TimelineFrames>,
    layout: Res<DenseAtomLayout>,
    perf: Res<crate::performance::PerformanceSettings>,
    mut extract: ResMut<GpuInterpolationExtract>,
    mut gpu_active: ResMut<GpuInterpolationActive>,
) {
    gpu_active.0 = false;

    if !perf.gpu_interpolation_enabled
        || !sim_data.loaded
        || layout.dense_atom_ids.is_empty()
        || frames.current.is_none()
    {
        extract.active = false;
        return;
    }

    let Some(current) = &frames.current else {
        extract.active = false;
        return;
    };

    let alpha = if timeline.interpolate {
        timeline.interpolation_factor
    } else {
        0.0
    };

    let needs_interpolation = alpha > 0.0 && frames.next.is_some();
    if !needs_interpolation && !timeline.is_changed() && !frames.is_changed() {
        extract.active = false;
        return;
    }

    let num_atoms = layout.dense_atom_ids.len() as u32;
    let frames_changed =
        extract.positions_a.is_empty() || extract.last_current_index != frames.current_index;

    if frames_changed || extract.positions_a.len() != layout.dense_atom_ids.len() {
        extract.positions_a = dense_positions(current, &layout.dense_atom_ids);
        extract.positions_b = if let Some(next) = &frames.next {
            dense_positions(next, &layout.dense_atom_ids)
        } else {
            extract.positions_a.clone()
        };
        extract.frames_changed = true;
        extract.last_current_index = frames.current_index;
    } else if needs_interpolation {
        if let Some(next) = &frames.next {
            extract.positions_b = dense_positions(next, &layout.dense_atom_ids);
        }
        extract.frames_changed = false;
    }

    extract.active = true;
    extract.alpha = alpha;
    extract.num_atoms = num_atoms;
    gpu_active.0 = true;
}

fn dense_positions(frame: &crate::core::trajectory::FrameData, atom_ids: &[u32]) -> Vec<Vec3> {
    atom_ids
        .iter()
        .map(|&id| frame.get_position(id).unwrap_or(Vec3::ZERO))
        .collect()
}

/// Apply GPU-interpolated positions to instanced meshes.
pub fn apply_gpu_interpolated_positions(
    gpu_active: Res<GpuInterpolationActive>,
    readback: Res<GpuInterpolationReadback>,
    layout: Res<DenseAtomLayout>,
    index: Res<InstancedAtomIndex>,
    mut instanced: Query<(&InstancedAtomEntity, &mut InstancedAtomMesh)>,
) {
    if !gpu_active.0 {
        return;
    }

    let Ok(positions) = readback.try_recv() else {
        return;
    };

    if positions.len() != layout.dense_atom_ids.len() {
        return;
    }

    // Group position updates by element for a single pass over instanced entities.
    let mut by_element: HashMap<Element, Vec<(u32, Vec3)>> = HashMap::new();
    for (atom_id, &(element, instance_idx)) in &index.atom_to_instance {
        let Some(&dense_idx) = layout.atom_id_to_dense.get(atom_id) else {
            continue;
        };
        let Some(pos) = positions.get(dense_idx as usize) else {
            continue;
        };
        by_element
            .entry(element)
            .or_default()
            .push((instance_idx, *pos));
    }

    for (entity_info, mut mesh) in instanced.iter_mut() {
        let Some(updates) = by_element.get(&entity_info.element) else {
            continue;
        };
        for &(instance_idx, pos) in updates {
            if let Some(instance) = mesh.instances.get_mut(instance_idx as usize) {
                instance.position = pos;
            }
        }
        mesh.mark_gpu_dirty();
    }
}

/// Clear dense layout when a new file loads.
pub fn clear_dense_layout_on_load(
    mut layout: ResMut<DenseAtomLayout>,
    mut extract: ResMut<GpuInterpolationExtract>,
    file_loaded_events: EventReader<crate::systems::loading::FileLoadedEvent>,
    topology_events: EventReader<crate::systems::loading::TopologyAppliedEvent>,
) {
    if file_loaded_events.is_empty() && topology_events.is_empty() {
        return;
    }
    layout.clear();
    *extract = GpuInterpolationExtract::default();
}

/// Build dense layout after instanced atoms spawn.
pub fn build_dense_layout_on_spawn(
    sim_data: Res<SimulationData>,
    instanced: Res<crate::rendering::instanced::InstancedAtomEntities>,
    mut layout: ResMut<DenseAtomLayout>,
) {
    if !sim_data.loaded || instanced.total_atoms == 0 {
        return;
    }
    if layout.dense_atom_ids.is_empty() {
        *layout = DenseAtomLayout::build(&sim_data.atom_data);
    }
}

// ============================================================================
// RENDER WORLD
// ============================================================================

struct GpuInterpolationRenderPlugin;

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct GpuInterpolationLabel;

impl Plugin for GpuInterpolationRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractResourcePlugin::<GpuInterpolationExtract>::default());

        app.sub_app_mut(RenderApp)
            .add_systems(
                Render,
                (
                    prepare_interpolation_buffers
                        .in_set(RenderSet::PrepareResources)
                        .run_if(resource_exists::<InterpolationBuffers>),
                    prepare_interpolation_bind_group
                        .in_set(RenderSet::PrepareBindGroups)
                        .run_if(resource_changed::<InterpolationBuffers>),
                ),
            );

        app.sub_app_mut(RenderApp)
            .world_mut()
            .resource_mut::<RenderGraph>()
            .add_node(GpuInterpolationLabel, GpuInterpolationNode);
    }

    fn finish(&self, app: &mut App) {
        app.sub_app_mut(RenderApp)
            .init_resource::<InterpolationPipeline>();
        let (sender, receiver) = crossbeam_channel::unbounded();
        app.insert_resource(GpuInterpolationReadback(receiver))
            .insert_resource(GpuInterpolationSender(sender.clone()));

        app.sub_app_mut(RenderApp)
            .insert_resource(GpuInterpolationSender(sender))
            .init_resource::<InterpolationBuffers>();
    }
}

#[derive(Resource)]
struct InterpolationPipeline {
    bind_group_layout: BindGroupLayout,
    pipeline: CachedComputePipelineId,
}

impl FromWorld for InterpolationPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let bind_group_layout = render_device.create_bind_group_layout(
            "atom_interpolate_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    storage_buffer::<Vec3>(true),
                    storage_buffer::<Vec3>(true),
                    storage_buffer::<Vec3>(false),
                    uniform_buffer_sized(false, NonZeroU64::new(16)),
                ),
            ),
        );

        let shader = world.load_asset(SHADER_PATH);
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("atom_interpolate".into()),
            layout: vec![bind_group_layout.clone()],
            push_constant_ranges: Vec::new(),
            shader,
            shader_defs: vec![],
            entry_point: Cow::from("main"),
        });

        Self {
            bind_group_layout,
            pipeline,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct InterpolationUniformsGpu {
    alpha: f32,
    num_atoms: u32,
    _padding: [f32; 2],
}

#[derive(Resource, Default)]
struct InterpolationBuffers {
    positions_a: Option<Buffer>,
    positions_b: Option<Buffer>,
    positions_out: Option<Buffer>,
    uniforms: Option<Buffer>,
    readback: Option<Buffer>,
    capacity_atoms: usize,
}

#[derive(Resource)]
struct InterpolationBindGroup(BindGroup);

fn prepare_interpolation_buffers(
    mut commands: Commands,
    extract: Res<GpuInterpolationExtract>,
    mut buffers: ResMut<InterpolationBuffers>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    if !extract.active || extract.num_atoms == 0 {
        return;
    }

    let num_atoms = extract.num_atoms as usize;
    let byte_len = (num_atoms * std::mem::size_of::<Vec3>()) as u64;

    if buffers.capacity_atoms < num_atoms {
        let usage = BufferUsages::STORAGE | BufferUsages::COPY_SRC | BufferUsages::COPY_DST;
        buffers.positions_a = Some(render_device.create_buffer(&BufferDescriptor {
            label: Some("interp_positions_a"),
            size: byte_len,
            usage,
            mapped_at_creation: false,
        }));
        buffers.positions_b = Some(render_device.create_buffer(&BufferDescriptor {
            label: Some("interp_positions_b"),
            size: byte_len,
            usage,
            mapped_at_creation: false,
        }));
        buffers.positions_out = Some(render_device.create_buffer(&BufferDescriptor {
            label: Some("interp_positions_out"),
            size: byte_len,
            usage,
            mapped_at_creation: false,
        }));
        buffers.readback = Some(render_device.create_buffer(&BufferDescriptor {
            label: Some("interp_readback"),
            size: byte_len,
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        buffers.uniforms = Some(render_device.create_buffer(&BufferDescriptor {
            label: Some("interp_uniforms"),
            size: std::mem::size_of::<InterpolationUniformsGpu>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        buffers.capacity_atoms = num_atoms;
        commands.remove_resource::<InterpolationBindGroup>();
    }

    let Some(positions_a) = buffers.positions_a.as_ref() else {
        return;
    };
    let Some(positions_b) = buffers.positions_b.as_ref() else {
        return;
    };
    let Some(uniforms) = buffers.uniforms.as_ref() else {
        return;
    };

    render_queue.write_buffer(positions_a, 0, bytemuck::cast_slice(&extract.positions_a));
    render_queue.write_buffer(positions_b, 0, bytemuck::cast_slice(&extract.positions_b));

    let uniform_data = InterpolationUniformsGpu {
        alpha: extract.alpha,
        num_atoms: extract.num_atoms,
        _padding: [0.0; 2],
    };
    render_queue.write_buffer(uniforms, 0, bytemuck::bytes_of(&uniform_data));
}

fn prepare_interpolation_bind_group(
    mut commands: Commands,
    pipeline: Res<InterpolationPipeline>,
    buffers: Res<InterpolationBuffers>,
    render_device: Res<RenderDevice>,
) {
    if buffers.capacity_atoms == 0 {
        return;
    }

    let Some(positions_a) = buffers.positions_a.as_ref() else {
        return;
    };
    let Some(positions_b) = buffers.positions_b.as_ref() else {
        return;
    };
    let Some(positions_out) = buffers.positions_out.as_ref() else {
        return;
    };
    let Some(uniforms) = buffers.uniforms.as_ref() else {
        return;
    };

    let bind_group = render_device.create_bind_group(
        "atom_interpolate_bind_group",
        &pipeline.bind_group_layout,
        &BindGroupEntries::sequential((
            positions_a.as_entire_binding(),
            positions_b.as_entire_binding(),
            positions_out.as_entire_binding(),
            uniforms.as_entire_binding(),
        )),
    );
    commands.insert_resource(InterpolationBindGroup(bind_group));
}

#[derive(Default)]
struct GpuInterpolationNode;

impl render_graph::Node for GpuInterpolationNode {
    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let extract = world.get_resource::<GpuInterpolationExtract>();
        let Some(extract) = extract else {
            return Ok(());
        };
        if !extract.active || extract.num_atoms == 0 {
            return Ok(());
        }

        let pipeline_res = world.resource::<InterpolationPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let Some(compute_pipeline) = pipeline_cache.get_compute_pipeline(pipeline_res.pipeline)
        else {
            return Ok(());
        };

        let buffers = world.resource::<InterpolationBuffers>();
        let bind_group = world.get_resource::<InterpolationBindGroup>();
        let Some(bind_group) = bind_group else {
            return Ok(());
        };
        let Some(positions_out) = buffers.positions_out.as_ref() else {
            return Ok(());
        };
        let Some(readback) = buffers.readback.as_ref() else {
            return Ok(());
        };

        let workgroups = extract.num_atoms.div_ceil(WORKGROUP_SIZE);

        {
            let mut pass =
                render_context
                    .command_encoder()
                    .begin_compute_pass(&ComputePassDescriptor {
                        label: Some("atom_interpolate"),
                        ..default()
                    });
            pass.set_bind_group(0, &bind_group.0, &[]);
            pass.set_pipeline(compute_pipeline);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        let byte_len = (extract.num_atoms as usize * std::mem::size_of::<Vec3>()) as u64;
        render_context.command_encoder().copy_buffer_to_buffer(
            positions_out,
            0,
            readback,
            0,
            byte_len,
        );

        Ok(())
    }
}

/// Read GPU buffer after render submit (runs in Render schedule after RenderSet::Render).
fn map_interpolation_readback(
    extract: Res<GpuInterpolationExtract>,
    buffers: Res<InterpolationBuffers>,
    render_device: Res<RenderDevice>,
    sender: Res<GpuInterpolationSender>,
) {
    if !extract.active || extract.num_atoms == 0 || buffers.capacity_atoms == 0 {
        return;
    }

    let Some(readback) = buffers.readback.as_ref() else {
        return;
    };

    let byte_len = extract.num_atoms as usize * std::mem::size_of::<Vec3>();
    let buffer_slice = readback.slice(..byte_len as u64);
    let (signal_tx, signal_rx) = crossbeam_channel::unbounded::<()>();

    buffer_slice.map_async(MapMode::Read, move |result| match result {
        Ok(()) => {
            let _ = signal_tx.send(());
        }
        Err(err) => error!("GPU interpolation readback map failed: {err:?}"),
    });

    render_device.poll(Maintain::wait()).panic_on_timeout();

    if signal_rx.recv().is_err() {
        return;
    }

    let data = {
        let view = buffer_slice.get_mapped_range();
        bytemuck::cast_slice::<u8, Vec3>(&view).to_vec()
    };
    readback.unmap();

    let _ = sender.send(data);
}

struct GpuInterpolationRenderPluginWithReadback;

impl Plugin for GpuInterpolationRenderPluginWithReadback {
    fn build(&self, app: &mut App) {
        app.add_plugins(GpuInterpolationRenderPlugin);

        app.sub_app_mut(RenderApp)
            .add_systems(Render, map_interpolation_readback.after(RenderSet::Render));
    }
}

pub fn register(app: &mut App) {
    app.init_resource::<DenseAtomLayout>()
        .init_resource::<GpuInterpolationExtract>()
        .init_resource::<GpuInterpolationActive>()
        .add_plugins(GpuInterpolationRenderPluginWithReadback);

    info!("GPU interpolation module registered");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dense_layout_build() {
        use crate::core::atom::{AtomData, Element};
        let atoms = vec![
            AtomData::new(0, Element::C, 0, "UNK".into(), "A".into(), "C".into()),
            AtomData::new(1, Element::H, 0, "UNK".into(), "A".into(), "H".into()),
        ];
        let layout = DenseAtomLayout::build(&atoms);
        assert_eq!(layout.dense_atom_ids, vec![0, 1]);
    }

    #[test]
    fn test_interpolation_uniforms_size() {
        assert_eq!(std::mem::size_of::<InterpolationUniformsGpu>(), 16);
    }
}
