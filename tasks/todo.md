# Gumol Viz Engine Todo List

**Last updated:** 2026-07-06

## Current Status

The engine has a working instanced-rendering pipeline, full primary/secondary file format support, timeline playback with interpolation, bond detection, multiple visualization modes, selection/measurements, and PNG/OBJ/glTF export. Remaining work is concentrated in surface rendering, alternate color schemes, video/POV-Ray export, interaction polish, large-scale performance validation, and documentation/examples sync.

---

## Completed

### Core & I/O
- [x] Core data structures (`Atom`, `Bond`, `Molecule`, `Trajectory`, `TimelineState`)
- [x] XYZ parser with tests
- [x] PDB parser with tests (ATOM, HETATM, CONECT, CRYST1)
- [x] GRO parser with tests and loading integration
- [x] DCD parser with tests, streaming provider, and topology pairing
- [x] mmCIF parser with tests and `create_atom_data_from_mmcif()`
- [x] `FileFormat` detection and `is_loadable()` for all supported formats
- [x] `docs/SECONDARY_FORMATS.md`

### Loading & Scene Management
- [x] `systems/loading.rs` — `LoadFileEvent` pipeline, CLI arg, `SimulationData`, `FileHandle`
- [x] Async background file loading (`poll_async_load`)
- [x] DCD on-demand frame loading via `FrameProvider`
- [x] LRU frame cache + prefetch (`systems/frame_cache.rs`)
- [x] Topology loading for DCD trajectories
- [x] Instanced atom spawning (`rendering/instanced.rs`) — production render path
- [x] Legacy `systems/spawning.rs` kept for `AtomEntities` compatibility
- [x] Reload clears instanced atoms, bonds, wireframe, ribbon, frame cache

### Timeline & Animation
- [x] `systems/timeline.rs` — playback, speed, loop, keyboard controls
- [x] Frame interpolation (CPU + GPU compute path)
- [x] Timeline UI panel (play/pause, scrub, speed presets, interpolation toggle)
- [x] GPU interpolation shader (`assets/shaders/atom_interpolate.wgsl`, `rendering/gpu_interpolation.rs`)

### Bonds & Visualization
- [x] Distance-based bond detection with spatial index (`rstar`)
- [x] PDB CONECT record support
- [x] Bond order detection (single/double/triple)
- [x] Bond cylinder rendering with visibility/scale controls
- [x] CPK, Ball-and-Stick, Licorice modes
- [x] Wireframe mode (`rendering/wireframe.rs`)
- [x] Points mode
- [x] Protein ribbon modes: Cartoon, Tube, Trace (`rendering/ribbon.rs`)
- [x] Heuristic secondary-structure assignment for backbone
- [x] `VisualizationConfig` + UI mode selector and atom/bond scale controls

### Interaction & Camera
- [x] Atom selection via instanced pick proxies + `bevy_mod_picking`
- [x] Shift-toggle multi-select, Escape to clear
- [x] Selection highlighting on instanced batches
- [x] Distance, angle, and dihedral measurements (`interaction/measurement.rs`)
- [x] Inspector panel (`ui/inspector.rs`)
- [x] Pan-orbit camera (`bevy_panorbit_camera`)
- [x] F — focus on molecule; Shift+F — focus on selection

### Export
- [x] PNG/JPEG screenshot capture (`export/screenshot.rs`)
- [x] OBJ export (`export/obj.rs`)
- [x] glTF export (`export/gltf_export.rs`)
- [x] POV-Ray export (`export/povray.rs`)
- [x] Video export (`export/video.rs`)
- [x] Export UI buttons (screenshot, OBJ, glTF, POV-Ray, video)

### Performance (implemented — needs validation)
- [x] Instanced rendering (one draw call per element present)
- [x] Material pool — one CPK material per element (`rendering/material_pool.rs`)
- [x] Mesh pool + LOD system (`rendering/mesh_pool.rs`, `rendering/lod_system.rs`)
- [x] CPU frustum culling (`rendering/culling.rs`)
- [x] Spatial bond detection above threshold
- [x] `PerformanceSettings` + `PerformanceDiagnostics` resources
- [x] Benchmark suite (`benches/parsing.rs`, `rendering.rs`, `bonds.rs`, `loading.rs`)
- [x] Baseline JSON + regression script (`benches/baseline.json`, `scripts/check_bench_regression.py`)
- [x] Profiling guide (`docs/PROFILING.md`)

### UI
- [x] File open dialog, drag-and-drop, CLI load
- [x] Status panel (atoms, frames, memory, draw calls)
- [x] Timeline, visualization, bond settings panels
- [x] Help overlay (`ui/help.rs`)
- [x] Toast notifications (`ui/notifications.rs`)

### Testing
- [x] 80+ library unit tests passing
- [x] Integration tests: format detection, XYZ/PDB/GRO/DCD/mmCIF load, load pipeline
- [x] Example fixtures (`tests/fixtures/`, `examples/water.gro`, `water.cif`, `1CRN.pdb`)

---

## Remaining Work

### Priority: CRITICAL

- [x] Fix `test_gumol_viz_plugin_registers` — GPU interpolation plugin requires `RenderDevice` in headless test world
- [x] Fix clippy warnings (`gpu_interpolation.rs` dead code; CI runs `clippy -D warnings`)
- [ ] Verify `dev_dynamic` profile works end-to-end (panic = unwind profile exists; manual test still open)

### Priority: HIGH — Visualization

- [x] **Surface mode** — coarse solvent-accessible voxel shell (`rendering/surface.rs`)
- [x] **Color schemes** — CPK, Residue, Chain, B-factor wired to instanced colors + UI
- [ ] **Double/triple bond meshes** — order detected; no separate visual geometry

### Priority: HIGH — Export

- [x] **Video export** — `export/video.rs`; FFmpeg subprocess; `video` Cargo feature; UI record button
- [x] **POV-Ray export** — `export/povray.rs`; `.pov` spheres/cylinders + camera; UI button

### Priority: HIGH — Performance Validation

- [x] Prove 10K-atom load + playback targets (benchmarks + `tests/sprint1_validation.rs`)
- [x] Prove 100K-atom position-sync @ 60 FPS budget (criterion ~3.9 ms; see `docs/VALIDATION.md`)
- [x] Validate GPU interpolation CPU reference parity (unit + integration tests)
- [x] Enforce benchmark regression gate in CI (script exists; `bench-regression` job on PR/push)
- [ ] Full interactive 100K @ 60 FPS with bonds + UI (CPU estimate in Sprint 5; GPU profiling pending)
- [ ] Parallel trajectory parsing with `rayon` (dependency present; not wired)

### Priority: MEDIUM — Interaction & Camera

- [x] Box / drag selection — middle-mouse rubber band (`interaction/box_selection.rs`)
- [x] Atom / residue text labels — egui overlay on selected atoms (`ui/atom_labels.rs`)
- [ ] Fly-through camera mode (only orbit implemented)
- [ ] Selection manipulation (rotate/translate groups)
- [ ] Octree or spatial index for picking at very large scale

### Priority: MEDIUM — File I/O

- [ ] Memory-mapped XYZ/PDB loading (`memmap2` dependency unused in `src/`)
- [ ] XYZ streaming parser (`XYZStreamer` from dev plan — not built)
- [ ] Manual end-to-end UI testing for GRO, DCD, mmCIF loads

### Priority: MEDIUM — Examples & Documentation

- [x] Update `examples/basic_load.rs` to load a real file via instanced pipeline
- [x] Add `timeline_demo` example (referenced in README; missing from `Cargo.toml`)
- [x] Add `interactive_selection` example (referenced in README; missing)
- [ ] Update README roadmap to match implemented features
- [ ] Sync stale docs: `BUILD_OUT_ROADMAP.md`, `OPTIMIZATION_PROGRESS.md`, `PROJECT_README.md`
- [ ] Add inline documentation to all public systems/APIs
- [ ] Write user guide (listed in dev plan)

### Priority: LOW — Advanced / v1.0+

- [ ] DSSP-based secondary structure (replace distance heuristic)
- [ ] Volume / isosurface rendering
- [ ] Dedicated settings panel (lighting, background, global render options)
- [ ] Trajectory editing (cut, splice, merge)
- [ ] Real-time analysis (RMSD, RMSF)
- [ ] Plugin / extension system
- [ ] Python bindings
- [ ] VR support (OpenXR)

### Priority: LOW — Manual QA (unchecked from earlier phases)

See interactive checklist in `docs/VALIDATION.md`.

- [ ] Manual test: single and multi-atom selection in running app
- [ ] Manual test: measurements on selected atoms in running app
- [ ] Manual test: bond detection on protein structures (e.g. 1CRN)
- [ ] Manual test: screenshot save flow in running app
- [ ] Manual test: timeline scrub/playback on multi-frame XYZ

---

## Session History

### 2026-02-23 — Foundation
- [x] Project structure review and initial implementation plan
- [x] File loading, spawning, timeline, selection, bonds, viz modes, measurements, screenshot export

### 2026-02-25 / 2026-02-27 — Secondary Formats
- [x] GRO, DCD, mmCIF parsers + tests + loading integration
- [x] `docs/SECONDARY_FORMATS.md`

### 2025-06-17 — GPU Performance (analysis + implementation)
- [x] `docs/GPU_PERFORMANCE_ANALYSIS.md`, `docs/QUICK_START_OPTIMIZATION.md`
- [x] Instanced rendering pipeline
- [x] Material pool, mesh pool, LOD, frustum culling
- [x] Spatial bond detection
- [x] Async file loading, DCD streaming, frame cache
- [x] GPU compute interpolation (WGSL shader + render graph node)
- [x] Benchmark suite and baseline
- [ ] Puffin/Tracy profiling integration (Bevy `trace` feature available; puffin not added)
- [ ] Parallel file parsing with rayon

### 2026-07-08 — Sprint 5 (interaction + CPU budget)
- [x] Box selection (middle-mouse drag, Shift additive)
- [x] Atom labels overlay + UI toggle
- [x] Ctrl+A select all
- [x] Combined 100K CPU budget estimate test (`tests/sprint5_validation.rs`)

### 2026-07-07 — Sprint 4 (v0.2 export + examples)
- [x] POV-Ray export (`export/povray.rs`) + UI button
- [x] `basic_load`, `timeline_demo`, `interactive_selection` examples

### 2026-07-07 — Sprint 1 validation
- [x] Automated validation tests (`tests/sprint1_validation.rs`)
- [x] GPU/CPU interpolation parity tests
- [x] Benchmark regression baseline refresh + `docs/VALIDATION.md`
- [ ] Interactive manual QA checklist (see VALIDATION.md)

---
- [x] Codebase review against todo list
- [x] Updated this file to reflect actual implementation state

---

*Created: 2026-02-23*
*Last reviewed: 2026-07-06*
