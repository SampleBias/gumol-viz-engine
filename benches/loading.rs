mod common;

use common::{synthetic_atom_data, synthetic_trajectory};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use gumol_viz_engine::performance::memory::{estimate_simulation_bytes, format_bytes};
use gumol_viz_engine::systems::loading::SimulationData;

fn bench_memory_estimate(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_estimate");

    for count in [1_000usize, 10_000, 100_000] {
        let sim = SimulationData::new(synthetic_trajectory(count, 100), synthetic_atom_data(count));

        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, _| {
            b.iter(|| {
                let bytes = estimate_simulation_bytes(&sim);
                black_box(format_bytes(bytes))
            });
        });
    }
    group.finish();
}

fn bench_dense_positions(c: &mut Criterion) {
    let mut group = c.benchmark_group("dense_positions");

    for count in [1_000usize, 10_000, 100_000] {
        let sim = SimulationData::new(synthetic_trajectory(count, 50), synthetic_atom_data(count));

        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, _| {
            b.iter(|| black_box(sim.frame_positions_dense(25)));
        });
    }
    group.finish();
}

criterion_group!(benches, bench_memory_estimate, bench_dense_positions);
criterion_main!(benches);
