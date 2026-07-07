mod common;

use bevy::prelude::*;
use common::{synthetic_atom_data, synthetic_positions, synthetic_trajectory};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use gumol_viz_engine::core::atom::Element;
use gumol_viz_engine::core::trajectory::FrameData;
use gumol_viz_engine::core::visualization::VisualizationConfig;
use gumol_viz_engine::performance::PerformanceSettings;
use gumol_viz_engine::rendering::instanced::{
    estimate_instanced_draw_calls, spawn_atoms_instanced_internal, AtomInstanceData,
    MAX_INSTANCED_DRAW_CALLS,
};
use gumol_viz_engine::rendering::mesh_pool::AtomMeshPool;
use gumol_viz_engine::systems::bonds::{resolve_bond_list, BondDetectionConfig};
use gumol_viz_engine::systems::loading::SimulationData;
use gumol_viz_engine::utils::spatial_index::AtomSpatialIndex;

fn bench_instance_data_build(c: &mut Criterion) {
    let mut group = c.benchmark_group("instance_data_build");

    for count in [1_000usize, 10_000, 100_000] {
        group.throughput(Throughput::Elements(count as u64));
        let atoms = synthetic_atom_data(count);
        let mut frame = FrameData::new(0, 0.0);
        for a in &atoms {
            frame.set_position(a.id, Vec3::new(a.id as f32, 0.0, 0.0));
        }

        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, _| {
            b.iter(|| {
                let mut instances = Vec::with_capacity(atoms.len());
                for a in &atoms {
                    if let Some(pos) = frame.get_position(a.id) {
                        instances.push(AtomInstanceData {
                            position: pos,
                            scale: 1.0,
                            color: Vec4::ONE,
                        });
                    }
                }
                black_box(instances.len())
            });
        });
    }
    group.finish();
}

fn bench_instanced_spawn(c: &mut Criterion) {
    let mut group = c.benchmark_group("instanced_spawn");
    let viz = VisualizationConfig::default();

    for count in [1_000usize, 10_000, 100_000] {
        group.throughput(Throughput::Elements(count as u64));
        let atoms = synthetic_atom_data(count);
        let mut frame = FrameData::new(0, 0.0);
        for a in &atoms {
            frame.set_position(a.id, Vec3::new(a.id as f32 * 0.1, 0.0, 0.0));
        }

        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, _| {
            b.iter(|| {
                let mut app = App::new();
                app.init_resource::<Assets<Mesh>>();
                app.world_mut()
                    .resource_scope(|world, mut meshes: Mut<Assets<Mesh>>| {
                        let mut mesh_pool = AtomMeshPool::default();
                        let mut commands = world.commands();
                        let (entities, _) = spawn_atoms_instanced_internal(
                            &mut commands,
                            &mut meshes,
                            &mut mesh_pool,
                            &frame,
                            &atoms,
                            &viz,
                        );
                        black_box((entities.len(), estimate_instanced_draw_calls(&atoms)))
                    })
            });
        });
    }
    group.finish();
}

fn bench_draw_call_count(c: &mut Criterion) {
    let mut group = c.benchmark_group("draw_call_count");

    for count in [1_000usize, 10_000, 100_000] {
        let atoms = synthetic_atom_data(count);
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, _| {
            b.iter(|| {
                let draw_calls = estimate_instanced_draw_calls(&atoms);
                assert!(draw_calls <= MAX_INSTANCED_DRAW_CALLS);
                black_box(draw_calls)
            });
        });
    }
    group.finish();
}

fn bench_timeline_position_update(c: &mut Criterion) {
    let mut group = c.benchmark_group("timeline_position_update");

    for count in [1_000usize, 10_000] {
        group.throughput(Throughput::Elements(count as u64));
        let atoms = synthetic_atom_data(count);
        let trajectory = synthetic_trajectory(count, 10);
        let sim = SimulationData::new(trajectory, atoms.clone());
        let frame = sim.trajectory.get_frame(5).unwrap().clone();

        let mut ids_by_element = std::collections::HashMap::new();
        for atom in &atoms {
            ids_by_element
                .entry(atom.element)
                .or_insert_with(Vec::new)
                .push(atom.id);
        }

        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, _| {
            b.iter(|| {
                let mut instances_by_element: std::collections::HashMap<
                    Element,
                    Vec<AtomInstanceData>,
                > = std::collections::HashMap::new();
                for atom in &atoms {
                    if let Some(pos) = frame.get_position(atom.id) {
                        instances_by_element.entry(atom.element).or_default().push(
                            AtomInstanceData {
                                position: pos,
                                scale: 1.0,
                                color: Vec4::ONE,
                            },
                        );
                    }
                }
                for (element, ids) in &ids_by_element {
                    if let Some(instances) = instances_by_element.get_mut(element) {
                        for (i, &atom_id) in ids.iter().enumerate() {
                            if let Some(pos) = frame.get_position(atom_id) {
                                instances[i].position = pos;
                            }
                        }
                    }
                }
                black_box(instances_by_element.len())
            });
        });
    }
    group.finish();
}

fn bench_frame_position_sync(c: &mut Criterion) {
    let mut group = c.benchmark_group("frame_position_dense");

    for count in [1_000usize, 10_000, 100_000] {
        let atoms = synthetic_atom_data(count);
        let trajectory = synthetic_trajectory(count, 10);
        let sim = SimulationData::new(trajectory, atoms);

        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, _| {
            b.iter(|| black_box(sim.frame_positions_dense(5)));
        });
    }
    group.finish();
}

fn bench_bond_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("bond_detection_spatial");
    let config = BondDetectionConfig::default();
    let perf = PerformanceSettings::default();

    for count in [1_000usize, 10_000] {
        let atoms = synthetic_atom_data(count);
        let positions = synthetic_positions(count);
        let spatial = AtomSpatialIndex::build(&atoms, &positions);
        let sim = SimulationData::new(synthetic_trajectory(count, 1), atoms);

        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, _| {
            b.iter(|| {
                black_box(resolve_bond_list(
                    &sim,
                    &positions,
                    &config,
                    &perf,
                    Some(&spatial),
                ))
            });
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_instance_data_build,
    bench_instanced_spawn,
    bench_draw_call_count,
    bench_timeline_position_update,
    bench_frame_position_sync,
    bench_bond_detection
);
criterion_main!(benches);
