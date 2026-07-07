# Profiling Guide — 10K Atom Load + Playback

Manual profiling workflow for Phase 10 (Task 10.2). Complements `benches/baseline.json` and CI regression checks.

## Quick commands

```bash
# CPU benchmarks + draw-call verification
cargo bench --bench rendering

# Compare against committed baseline (20% gate)
python3 scripts/check_bench_regression.py

# Chrome / Bevy trace (frame + render diagnostics)
RUST_LOG=info,bevy_render=debug cargo run --release --features trace -- path/to/10k.pdb

# CPU flamegraph (requires cargo-flamegraph)
cargo flamegraph --release -- path/to/10k.pdb
```

## 10K load + playback — top hotspots (release, instanced path)

Profiled via criterion benches and code-path analysis on the instanced pipeline:

| Rank | Hotspot | Location | Notes |
|------|---------|----------|-------|
| 1 | Bond detection (spatial) | `systems/bonds.rs`, `resolve_bond_list` | ~166 ms @ 10K atoms in bench; dominates load when bonds enabled |
| 2 | Instanced spawn (group + mesh alloc) | `rendering/instanced.rs`, `spawn_atoms_instanced_internal` | ~726 µs @ 10K; one entity per element (~3 draw calls for C/H/O test data) |
| 3 | Timeline position sync | `update_instanced_positions_from_timeline` | ~515 µs @ 10K/frame; O(atoms) CPU writes, GPU upload skipped when static (`gpu_dirty`) |
| 4 | Dense frame positions | `SimulationData::frame_positions_dense` | ~186 µs @ 10K; HashMap → Vec copy for bonds/pick proxies |
| 5 | GPU instance upload | `prepare_instance_buffers` | Only runs when `gpu_dirty`; uses `write_buffer` reuse instead of full buffer recreate |

## Draw call target

Instanced atoms use **one draw call per element present** (max 118):

- `estimate_instanced_draw_calls()` — CPU-side count from atom data
- `draw_call_count/*` criterion group asserts `≤ MAX_INSTANCED_DRAW_CALLS` (118)
- UI shows `~N` from `InstancedAtomEntities.entities.len()`

For Tracy/GPU verification: enable `--features trace` and inspect transparent 3D phase item count in Bevy diagnostics.

## When playback is slow

1. Confirm timeline updates set `gpu_dirty` only when positions/colors/scales change
2. Disable pick proxies above `PerformanceSettings.max_pick_proxies` (default 50K)
3. Enable frustum culling (`PerformanceSettings.frustum_culling_enabled`)
4. Check bond detection is using spatial index (`AtomSpatialIndex`)

## Updating baseline

After intentional performance changes:

```bash
cargo bench --bench rendering -- --noplot --sample-size 20 2>&1 | tee /tmp/bench.txt
# Update benches/baseline.json mean_ns values from criterion output
python3 scripts/check_bench_regression.py
```
