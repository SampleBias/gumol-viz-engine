use criterion::{criterion_group, criterion_main, Criterion};

fn bench_xyz_parse_placeholder(c: &mut Criterion) {
    c.bench_function("parsing_placeholder", |b| {
        b.iter(|| 1 + 1);
    });
}

criterion_group!(benches, bench_xyz_parse_placeholder);
criterion_main!(benches);
