# Gumol Viz Engine - Development Context

## Project Purpose
A high-performance Rust-based visualization engine for Molecular Dynamics (MD) simulations using the Bevy game engine. Designed for interactive, game-like visualization of molecular structures with GPU acceleration.

## Architecture Overview
- **Project Type**: Scientific Visualization / Molecular Dynamics
- **Development Status**: Active development — core pipeline complete, polish and advanced features in progress
- **Development Framework**: Bevy 0.14 ECS (Entity-Component-System)
- **Context Tracking**: Integrated with Vybrid development workflow

## Technology Stack
- **Language**: Rust 1.75+
- **Game Engine**: Bevy 0.14 (ECS-based, GPU-accelerated rendering)
- **UI Framework**: EGUI 0.28 (immediate-mode GUI)
- **3D Interaction**: bevy_mod_picking 0.20, bevy_panorbit_camera 0.19
- **Math**: nalgebra 0.33
- **Parsing**: nom 7.1 (parser combinators)
- **Spatial Indexing**: rstar 0.12 (bond detection)
- **Parallel Processing**: rayon 1.10 (dependency present; parallel parse not wired)
- **Error Handling**: thiserror 1.0, anyhow 1.0

## Project Structure
```
gumol-viz-engine/
├── src/
│   ├── core/           # Atom, bond, molecule, trajectory, visualization types
│   ├── io/             # XYZ, PDB, GRO, DCD, mmCIF parsers + streaming
│   ├── rendering/      # Instanced rendering, LOD, culling, wireframe, ribbon, GPU interpolation
│   ├── systems/        # Loading, timeline, bonds, visualization, frame cache
│   ├── performance/    # PerformanceSettings, diagnostics, memory budgeting
│   ├── camera/         # Focus-on-molecule / focus-on-selection
│   ├── interaction/    # Selection, measurements, pick proxies
│   ├── ui/             # EGUI panels, inspector, help, notifications
│   ├── export/         # Screenshot, OBJ, glTF
│   └── utils/          # Geometry, colors, math, spatial index
├── examples/           # basic_load, xyz_viewer, pdb_viewer
├── tests/              # Integration tests per format + load pipeline
├── benches/            # Parsing, rendering, bonds, loading benchmarks
├── docs/               # Architecture, plans, profiling guides
└── tasks/              # Development task tracking
```

## Current Implementation Status

### ✅ Completed

**Core & I/O**
- Core data structures: Atom, Element, Bond, Molecule, Trajectory, FrameData, TimelineState
- All primary and secondary parsers: XYZ, PDB, GRO, DCD, mmCIF (with unit + integration tests)
- File format detection and loadable-format gating
- DCD streaming via `FrameProvider` + LRU frame cache with prefetch

**Loading & Rendering**
- Event-driven file loading (CLI, drag-drop, file picker, async background thread)
- Instanced atom rendering pipeline (one draw call per element present)
- Material pool (CPK color per element), mesh pool, LOD, frustum culling
- Bond detection (distance-based + PDB CONECT, spatial index for large systems)
- Wireframe, Points, Cartoon/Tube/Trace ribbon modes for proteins
- GPU compute frame interpolation (WGSL shader + CPU fallback)

**Interaction & UI**
- Atom selection (click, Shift-toggle, Escape clear, highlighting)
- Distance, angle, dihedral measurements
- Inspector panel, timeline controls, visualization/bond settings, help overlay
- Pan-orbit camera + F / Shift+F focus shortcuts

**Export**
- PNG/JPEG screenshots, OBJ export, glTF export

**Testing & Tooling**
- 80+ unit tests, format-specific integration tests, benchmark suite with baseline regression script
- CI: fmt, clippy, tests, release build, benchmark smoke on main

### ⚠️ Partially Implemented

| Feature | Status |
|---------|--------|
| **Surface mode** | Enum + UI placeholder; rendering not built |
| **Color schemes** | `ColorScheme` enum defined; only CPK applied at runtime |
| **B-factor coloring** | Palette helper exists; no UI toggle or live material update |
| **Bond order visuals** | Order detected; no separate double/triple bond meshes |
| **Secondary structure** | Heuristic assignment; DSSP not integrated |
| **100K @ 60 FPS** | Architecture supports it; formal validation not complete |
| **Memory-mapped XYZ/PDB** | `memmap2` dependency present; not used in `src/` |
| **Video export** | `video` Cargo feature declared; no implementation |
| **POV-Ray export** | Documented; not implemented |

### ❌ Not Implemented

- Box / drag selection
- Atom / residue text labels
- Fly-through camera mode
- Selection manipulation (rotate/translate)
- Octree for picking at very large scale
- XYZ streaming parser
- Parallel trajectory parsing with rayon
- Volume / isosurface rendering
- Trajectory editing, RMSD/RMSF analysis
- Plugin system, Python bindings, VR

## Performance Status

The instanced rendering pipeline addresses the bottlenecks identified in the June 2025 analysis:

| Technique | Status |
|-----------|--------|
| Instanced rendering | ✅ Implemented |
| Material pooling | ✅ Implemented |
| GPU compute interpolation | ✅ Implemented (needs validation) |
| Async file loading | ✅ Implemented |
| Spatial bond detection | ✅ Implemented |
| Frustum culling | ✅ Implemented |
| Level-of-detail | ✅ Implemented |
| DCD streaming + frame cache | ✅ Implemented |
| Parallel parsing (rayon) | ❌ Not wired |
| Memory-mapped XYZ/PDB | ❌ Not implemented |

See `docs/GPU_PERFORMANCE_ANALYSIS.md`, `docs/PROFILING.md`, and `benches/baseline.json` for profiling workflow.

**Known issues:**
- `test_plugin_registration` fails in headless tests (GPU interpolation requires `RenderDevice`)
- Minor clippy/dead-code warnings in `gpu_interpolation.rs`

## Getting Started

### Prerequisites
- Rust 1.75 or higher
- Cargo (comes with Rust)
- FFmpeg (optional — video export not yet implemented)

### Installation
```bash
git clone <repository-url>
cd gumol-viz-engine
cargo build --release
```

### Running the Project
```bash
# Run with demo water molecule (no file arg)
cargo run --release

# Load a file via CLI
cargo run --release -- examples/1CRN.pdb
cargo run --release -- trajectory.xyz

# Run examples
cargo run --example basic_load
cargo run --example xyz_viewer -- input.xyz
cargo run --example pdb_viewer -- input.pdb
```

### Controls (main app)
- Mouse drag — rotate camera; scroll — zoom
- Drag file onto window — load molecular file
- Click atom — select; Shift+click — toggle; Escape — clear
- Space — play/pause timeline; ←/→ — prev/next frame
- F — focus on molecule; Shift+F — focus on selection
- F11 — fullscreen

## Development Status

### Current Phase: Polish & Advanced Features
Foundation, rendering pipeline, formats, interaction, and basic export are complete. Active work targets surface rendering, color schemes, export gaps, performance validation, and documentation sync.

### Next Priorities (see `tasks/todo.md`)
1. Fix plugin registration test + clippy warnings
2. Surface mode and runtime color scheme switching
3. Video and POV-Ray export
4. Missing examples (`timeline_demo`, `interactive_selection`)
5. 100K-atom performance validation
6. Documentation sync across README, BUILD_OUT_ROADMAP, OPTIMIZATION_PROGRESS

### Task Tracking
- `tasks/todo.md` — living task list with completed/remaining sections
- `docs/activity.md` — development timeline

## Key Context for AI Agents

### Development Workflow
- Three tracked files: `tasks/todo.md`, `docs/activity.md`, `docs/PROJECT_README.md`
- Tasks executed immediately — no approval waiting required

### Code Style & Standards
- `cargo fmt` for formatting
- `cargo clippy -- -D warnings` (CI enforced)
- Unit tests for new functionality
- Documentation on public APIs
- Follow Rust idioms and Bevy 0.14 patterns

### Bevy-Specific Guidelines
- ECS: Entities + Components processed by Systems
- Resources for global state (`TimelineState`, `SelectionState`, `SimulationData`)
- Plugin architecture via `GumolVizPlugin` and per-module `register()`
- Production atom rendering uses `rendering::instanced` (not legacy `spawning.rs` entities)
- System ordering in `systems/mod.rs` uses explicit `.chain()` groups

### Performance Targets
- 100,000+ atoms at 60 FPS (architecture in place; validation pending)
- Trajectories with 10,000+ frames via DCD streaming
- Multi-gigabyte DCD files via on-demand frame loading + LRU cache

### File Format Support
| Format | Parser | App Loading | Streaming |
|--------|--------|-------------|-----------|
| XYZ | ✅ | ✅ | ❌ (full load) |
| PDB | ✅ | ✅ | ❌ (full load) |
| GRO | ✅ | ✅ | ❌ |
| DCD | ✅ | ✅ | ✅ (with topology) |
| mmCIF | ✅ | ✅ | ❌ |

## Documentation Links
- [Task List](../tasks/todo.md) — current tasks and priorities
- [Activity Log](activity.md) — development timeline
- [Development Plan](DEVELOPMENT_PLAN.md) — full roadmap spec
- [Architecture Guide](ARCHITECTURE.md) — system layout and data flow
- [Secondary Formats](SECONDARY_FORMATS.md) — GRO, DCD, mmCIF details
- [Profiling Guide](PROFILING.md) — benchmarks and hotspot analysis
- [Setup Guide](SETUP.md) — environment setup
- [README](../README.md) — project overview and quick start

---
*Last Updated: 2026-07-06*
*Context Version: 3.0*
*Development Phase: Polish & Advanced Features*
