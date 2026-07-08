# Sprint 1 Validation Report

**Date:** 2026-07-07  
**Branch:** `master`  
**Purpose:** Prove core viewer paths before v0.2.0 feature work (surface, color schemes, video).

---

## Automated coverage

| Area | Test / check | Status |
|------|----------------|--------|
| 1CRN load pipeline | `tests/sprint1_validation.rs::test_1crn_load_pipeline` | ✅ |
| PDB CONECT + spatial bonds | `test_1crn_bond_data_and_spatial_detection` | ✅ |
| Multi-frame CPU/GPU lerp parity | `test_multi_frame_timeline_interpolation` + `gpu_interpolation` unit test | ✅ |
| 100K draw-call budget | `test_100k_instanced_draw_calls_within_budget` | ✅ |
| 100K position sync (release) | `test_100k_position_dense_sync_under_60fps_budget` | ✅ (release only) |
| 1CRN instanced spawn | `test_instanced_spawn_1crn_scale` | ✅ |
| OBJ / glTF export | `test_export_obj_and_gltf_from_snapshot` | ✅ |
| Full test suite | `cargo test --all-targets` | ✅ |
| Clippy | `cargo clippy --all-targets -- -D warnings` | ✅ |
| Benchmark regression | `python3 scripts/check_bench_regression.py` | ✅ (baseline refreshed 2026-07-07) |

Run Sprint 1 tests:

```bash
cargo test --test sprint1_validation
cargo test --release --test sprint1_validation test_100k_position_dense
python3 scripts/check_bench_regression.py
```

---

## Performance proof (release benchmarks)

Criterion group `rendering` on Linux (sample-size 20). Mean times:

| Benchmark | 1K | 10K | 100K |
|-----------|-----|-----|------|
| `instance_data_build` | 0.017 ms | 0.204 ms | 3.788 ms |
| `instanced_spawn` (one-time load) | 0.139 ms | 0.845 ms | 15.155 ms |
| `draw_call_count` | 0.016 ms | 0.168 ms | 2.009 ms |
| `timeline_position_update` | 0.047 ms | 0.561 ms | — |
| `frame_position_dense` | 0.017 ms | 0.224 ms | **3.922 ms** |
| `bond_detection_spatial` | 2.532 ms | 204.790 ms | — |

### 100K @ 60 FPS assessment

- **60 FPS frame budget:** 16.67 ms
- **Dense position sync @ 100K:** ~3.9 ms (~25% of frame budget for timeline alone)
- **Instanced draw calls @ 100K:** ≤118 batches (one per element present); count pass ~2 ms
- **Verdict:** CPU timeline position path is **within budget** for 100K atoms on this hardware. Full-frame GPU rendering + UI + bond updates still need interactive profiling (`docs/PROFILING.md`) for a complete 60 FPS proof.

---

## GPU interpolation validation

- **Shader:** `assets/shaders/atom_interpolate.wgsl` — `mix(pos_a, pos_b, alpha)`
- **CPU reference:** `rendering::gpu_interpolation::interpolate_dense_positions`
- **Parity tests:** unit test in `gpu_interpolation.rs`; integration test on `demo_trajectory.xyz` (3 frames)
- **Runtime GPU path:** requires `RenderDevice`; validated interactively (toggle `G` in app). Headless CI uses CPU fallback.

---

## Manual QA checklist (interactive — not automated)

Run: `cargo run --release -- tests/fixtures/1CRN.pdb`

- [ ] Orbit / zoom / pan camera
- [ ] Switch viz modes 1–5 (CPK, ball-and-stick, licorice, wireframe, cartoon)
- [ ] Toggle bonds on/off
- [ ] Click-select atom; Shift+click multi-select; Escape clear
- [ ] Measure distance (2 atoms), angle (3), dihedral (4)
- [ ] Timeline: scrub slider, play/pause, arrow step (multi-frame: `demo_trajectory.xyz`)
- [ ] Toggle GPU interpolation (`G` key) on multi-frame file
- [ ] Export screenshot, OBJ, glTF; open outputs externally
- [ ] Load GRO / mmCIF / DCD+topology from UI

---

## CI wiring

`.github/workflows/ci.yml` includes:

- `check` — fmt, clippy, test, release build
- `bench-smoke` — quick criterion smoke on push to main/master
- `bench-regression` — `scripts/check_bench_regression.py` (20% gate vs `benches/baseline.json`)

---

## Known gaps after Sprint 1

- Interactive 60 FPS at 100K with full scene (bonds + UI + GPU) not formally profiled in-app (Sprint 5 adds CPU budget estimate test)
- Screenshot path not covered by automated test (requires window/GPU)
- `dev_dynamic` profile not manually verified
- 1CRN PDB CONECT records are sparse; ball-and-stick on proteins relies on distance detection when CONECT absent

---

## Sprint 5 — Interaction + CPU budget (2026-07-08)

| Area | Test / feature | Status |
|------|----------------|--------|
| Box selection | Middle-mouse drag + Shift additive | ✅ |
| Atom labels | egui overlay on selected atoms (toggle in UI) | ✅ |
| Select all | Ctrl+A (warns above 10K atoms) | ✅ |
| Combined 100K CPU estimate | `tests/sprint5_validation.rs` | ✅ |
| POV-Ray export smoke | `test_povray_roundtrip_export` | ✅ |

Run Sprint 5 tests:

```bash
cargo test --test sprint5_validation
cargo test --lib box_selection
```

### 100K combined CPU estimate

Measured release benchmarks (Sprint 1) plus conservative per-frame estimates:

| Component @ 100K | Mean (ms) |
|------------------|-----------|
| Dense position sync | 3.922 |
| Draw-call / instance count pass | 2.009 |
| Color scheme update (est.) | 1.0 |
| Bond position update (est.) | 0.5 |
| **Total CPU estimate** | **~7.4 ms** |

At ~7.4 ms, core CPU systems use ~44% of a 16.67 ms (60 FPS) frame, leaving headroom for GPU rendering and UI. Full interactive proof still requires in-app profiling (`docs/PROFILING.md`).
