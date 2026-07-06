mod common;

use bevy::prelude::*;
use common::{synthetic_atom_data, synthetic_positions, synthetic_trajectory};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use gumol_viz_engine::core::trajectory::FrameData;
use gumol_viz_engine::performance::PerformanceSettings;
use gumol_viz_engine::rendering::instanced::AtomInstanceData;
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
    bench_frame_position_sync,
    bench_bond_detection
);
criterion_main!(benches);
