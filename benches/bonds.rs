mod common;

use common::synthetic_trajectory;
use common::{synthetic_atom_data, synthetic_positions};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use gumol_viz_engine::performance::PerformanceSettings;
use gumol_viz_engine::systems::bonds::{resolve_bond_list, BondDetectionConfig};
use gumol_viz_engine::systems::loading::SimulationData;
use gumol_viz_engine::utils::spatial_index::AtomSpatialIndex;

fn bench_spatial_vs_naive(c: &mut Criterion) {
    let mut group = c.benchmark_group("bond_spatial_index");
    let config = BondDetectionConfig::default();

    for count in [500usize, 2_000, 5_000] {
        let atoms = synthetic_atom_data(count);
        let positions = synthetic_positions(count);
        let spatial = AtomSpatialIndex::build(&atoms, &positions);
        let sim = SimulationData::new(synthetic_trajectory(count, 1), atoms);

        let perf_spatial = PerformanceSettings {
            spatial_bond_threshold: 100,
            ..Default::default()
        };

        let perf_naive = PerformanceSettings {
            spatial_bond_detection: false,
            ..Default::default()
        };

        group.bench_with_input(BenchmarkId::new("spatial", count), &count, |b, _| {
            b.iter(|| {
                black_box(resolve_bond_list(
                    &sim,
                    &positions,
                    &config,
                    &perf_spatial,
                    Some(&spatial),
                ))
            });
        });

        group.bench_with_input(BenchmarkId::new("naive", count), &count, |b, _| {
            b.iter(|| {
                black_box(resolve_bond_list(
                    &sim,
                    &positions,
                    &config,
                    &perf_naive,
                    None,
                ))
            });
        });
    }
    group.finish();
}

fn bench_rtree_build(c: &mut Criterion) {
    let mut group = c.benchmark_group("rtree_build");

    for count in [1_000usize, 10_000, 50_000] {
        let atoms = synthetic_atom_data(count);
        let positions = synthetic_positions(count);

        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, _| {
            b.iter(|| black_box(AtomSpatialIndex::build(&atoms, &positions)));
        });
    }
    group.finish();
}

criterion_group!(benches, bench_spatial_vs_naive, bench_rtree_build);
criterion_main!(benches);
