# Profiling Guide — 10K / 100K Load + Playback

Manual and scripted profiling workflow for Phase 10 (Task 10.2). Complements `benches/baseline.json`, CI regression checks, and in-app FPS validation.

## Quick commands

```bash
# CPU benchmarks + draw-call verification
cargo bench --bench rendering

# Compare against committed baseline (20% gate)
python3 scripts/check_bench_regression.py

# Generate 100K synthetic fixtures (static + 10-frame playback)
./scripts/generate_100k_fixture.sh

# Interactive 100K static validation @ 60 FPS (auto-exit with pass/fail code)
./scripts/profile_100k_static.sh

# Interactive 100K playback validation @ 30+ FPS
./scripts/profile_100k_playback.sh

# Chrome / Bevy trace (frame + render diagnostics)
RUST_LOG=info,bevy_render=debug cargo run --release --features trace -- path/to/10k.pdb

# CPU flamegraph (requires cargo-flamegraph)
cargo flamegraph --release -- path/to/10k.pdb
```

## 100K interactive GPU validation

The app tracks rolling FPS in the status panel and supports automated profiling via CLI flags.

### Static scene (60 FPS target)

```bash
cargo run --release -- \
  --profile \
  --profile-exit \
  --generate-100k \
  --profile-output=target/profile_100k_static.json
```

Or use the dedicated example:

```bash
cargo run --release --example perf_100k -- \
  --profile --profile-exit --generate-100k
```

### Playback (30+ FPS target)

```bash
cargo run --release -- \
  --profile \
  --profile-playback \
  --profile-exit \
  --generate-100k \
  --profile-output=target/profile_100k_playback.json
```

### CLI flags

| Flag | Description |
|------|-------------|
| `--profile` | Enable automated warmup + sampling |
| `--profile-exit` | Exit with code 0 (pass) or 1 (fail) when done |
| `--profile-playback` | Auto-play timeline during sampling (30 FPS target) |
| `--generate-100k` | Generate/load `tests/fixtures/synthetic_100k*.xyz` |
| `--profile-warmup=N` | Warmup frames before sampling (default 120) |
| `--profile-frames=N` | Sample frames (default 300) |
| `--profile-output=PATH` | Write JSON report to file |

Environment variables (optional): `GUMOL_PROFILE`, `GUMOL_PROFILE_EXIT`, `GUMOL_PROFILE_PLAYBACK`, `GUMOL_PROFILE_OUTPUT`.

### Pass criteria

| Mode | Target | Metrics |
|------|--------|---------|
| Static @ 100K | ≥ 60 FPS avg | p95 frame time ≤ 1.5× frame budget (~25 ms) |
| Playback @ 100K | ≥ 30 FPS avg | p95 frame time ≤ 1.5× frame budget (~50 ms) |

Reports include atom count, draw calls, avg/min FPS, p95 frame time, and pass/fail. JSON is logged to stdout and written when `--profile-output` is set.

### What is measured

Full interactive frame time including:

- Instanced atom rendering (≤ 118 draw calls)
- EGUI overlay
- Bond meshes (spatial detection runs once at load)
- Frustum culling + LOD
- Timeline position sync (and GPU interpolation during playback mode)

Selection pick proxies are disabled above 50K atoms by design.

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

## Automated tests

```bash
cargo test --test sprint7_validation
cargo test --test sprint1_validation test_100k
cargo test --test sprint5_validation
```

Sprint 7 covers profiling report math and synthetic 100K fixture generation. CPU budget tests remain in Sprint 1/5; full GPU proof requires the interactive scripts above on target hardware.
