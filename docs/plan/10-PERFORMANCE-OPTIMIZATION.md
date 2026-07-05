# 10 — Performance Optimization

**Priority:** P2 (ongoing)  
**Estimated effort:** 2 developer-weeks  
**Dependencies:** 02 (instanced pipeline complete)  
**Can run in parallel after week 2**

---

## Goal

Hit the 100,000 atom @ 60 FPS target through GPU and CPU optimizations.

Reference: [OPTIMIZATION_PROGRESS.md](../OPTIMIZATION_PROGRESS.md)

---

## Optimization Roadmap

| Phase | Task | Impact | Effort | Status |
|-------|------|--------|--------|--------|
| 1 | Instanced rendering | 100–1000× draw calls | 1 week | 🟡 60% |
| 2 | Material pool | 10–20× state changes | 4 hours | ❌ |
| 3 | GPU compute interpolation | 10–50× animation | 2 days | ❌ |
| 4 | Async file loading | UX (no freeze) | 1 day | ❌ |
| 5 | Frustum culling | 2–10× render | 2 days | ❌ |
| 6 | Spatial partitioning | 10–100× bond detect | 2 days | ❌ |
| 7 | Level of detail | 5–20× vertices | 1 week | ❌ |

Phases 1–2 are in `02`. This document covers 3–7 and profiling infrastructure.

---

## Tasks

### Task 10.1 — Benchmark suite
**Effort:** 1 day  
**Files:** `benches/rendering.rs`, `benches/loading.rs`, `benches/bonds.rs`

- [ ] Criterion benchmarks for:
  - Atom spawn time (1K, 10K, 100K)
  - Frame update time (position sync)
  - Bond detection time
  - Draw call count (via diagnostics)
- [ ] Store results in `benches/baseline.json`
- [ ] CI regression check: fail if >20% slower

```bash
cargo bench --bench rendering
```

---

### Task 10.2 — Tracy / Chrome trace profiling
**Effort:** 4 hours  
**Files:** `Cargo.toml`, `src/main.rs`

- [ ] Add optional `trace` feature with `tracing-tracy` or Bevy's trace plugin
- [ ] Document profiling workflow in plan/12
- [ ] Profile 10K atom load + playback; identify top 3 hotspots

---

### Task 10.3 — Frustum culling for instances
**Effort:** 2 days  
**Files:** `rendering/instanced.rs`

- [ ] CPU-side: test each instance AABB against camera frustum
- [ ] Set instance scale to 0 for culled atoms (or use indirect draw count)
- [ ] GPU-side (v2): compute shader culling with `draw_indirect`

**Target:** 50% atoms off-screen → 2× FPS improvement.

---

### Task 10.4 — Spatial index for bond detection
**Effort:** 2 days  
**Files:** `systems/bonds.rs`, `Cargo.toml`

- [ ] Add `rstar = "0.12"` dependency
- [ ] Build R-tree of atom positions on load
- [ ] Neighbor query: O(log N) per atom instead of O(N)
- [ ] Rebuild tree on frame change (or update positions in tree)

**Target:** 100K atom bond detection < 5 seconds (vs minutes with O(N²)).

---

### Task 10.5 — Level of detail (LOD)
**Effort:** 1 week  
**Files:** new `src/rendering/lod.rs`

- [ ] Generate 4 sphere meshes: 32, 16, 8, 4 triangles
- [ ] Select LOD based on screen-space size (pixels on screen)
- [ ] Hysteresis to prevent popping
- [ ] Apply per element batch (all C atoms at same LOD)

**Target:** 5× vertex reduction at typical viewing distance.

---

### Task 10.6 — Struct-of-Arrays atom data
**Effort:** 2 days  
**Files:** `core/trajectory.rs`, `rendering/instanced.rs`

- [ ] Store positions as `Vec<Vec3>` indexed by atom_id (not HashMap)
- [ ] Contiguous memory for cache-friendly bulk updates
- [ ] `bytemuck` cast to GPU buffer without per-atom copy

---

### Task 10.7 — Pick proxy optimization (from 06)
**Effort:** 1 day  
**Files:** `interaction/pick_proxy.rs`

- [ ] For >50K atoms: disable pick proxies, use GPU picking or octree raycast
- [ ] Configurable: `max_pick_proxies: 50_000` in settings
- [ ] Fall back message: "Selection disabled for very large systems"

---

### Task 10.8 — Memory budget and monitoring
**Effort:** 4 hours  
**Files:** `ui/mod.rs`, `systems/loading.rs`

- [ ] Log memory usage on load (atom count × frame count × bytes)
- [ ] Warn if estimated RAM > 4 GB
- [ ] Suggest streaming mode (4.7) for large trajectories

---

## Performance Targets (Release)

| Metric | Target | Measurement |
|--------|--------|-------------|
| 100K atoms static | 60 FPS | criterion + manual |
| 100K atoms playback | 30+ FPS | with GPU interpolation |
| Draw calls (100K atoms) | ≤ 118 | Tracy / Bevy diagnostics |
| Load 10K atom PDB | < 500 ms | criterion |
| Bond detect 10K atoms | < 200 ms | with R-tree |
| UI freeze on load | 0 ms | async loading |

---

## Profiling Workflow for Developers

```bash
# 1. Baseline benchmark
cargo bench --bench rendering -- --save-baseline main

# 2. Make changes, compare
cargo bench --bench rendering -- --baseline main

# 3. Tracy (if feature enabled)
cargo run --release --features trace

# 4. GPU frame capture
RUST_LOG=bevy_render=debug cargo run --release 2>&1 | grep draw
```

---

## Definition of Done

- [ ] Benchmark suite in CI
- [ ] 100K atoms @ 30+ FPS on RTX 3060 / equivalent
- [ ] OPTIMIZATION_PROGRESS all phases marked complete
- [ ] Profiling guide in 12
