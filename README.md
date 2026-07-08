# Gumol Viz Engine

A high-performance Rust visualization engine for molecular dynamics simulations.

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![Bevy](https://img.shields.io/badge/Bevy-0.14-purple.svg)](https://bevyengine.org/)

Documentation: [docs/](docs/) | [Architecture](docs/ARCHITECTURE.md) | [Roadmap](docs/DEVELOPMENT_PLAN.md) | [Validation](docs/VALIDATION.md)

---

## Overview

Gumol Viz Engine provides interactive visualization of molecular structures and trajectories using GPU-accelerated instanced rendering. It is built on [Bevy](https://bevyengine.org/) 0.14 and targets smooth rendering of 100,000+ atoms at 60 FPS.

### Features

| Area | Capabilities |
|------|----------------|
| **Performance** | Instanced rendering, LOD, frustum culling, GPU frame interpolation, spatial bond detection, async loading, runtime FPS profiling |
| **File formats** | XYZ, PDB, GRO, DCD (with topology), mmCIF |
| **Visualization** | CPK, ball-and-stick, licorice, wireframe, points, surface, cartoon/tube/trace ribbons |
| **Color schemes** | CPK, residue, chain, B-factor |
| **Interaction** | Orbit camera, atom selection, box selection, distance/angle/dihedral measurements, atom labels |
| **Timeline** | Playback, scrubbing, speed control, frame interpolation |
| **Export** | PNG/JPEG screenshots, OBJ, glTF, POV-Ray, video (MP4/WebM/GIF via FFmpeg) |

---

## Table of Contents

- [Quick Start](#quick-start)
- [Running the Application](#running-the-application)
- [Library Usage](#library-usage)
- [File Formats](#file-formats)
- [Visualization Modes](#visualization-modes)
- [Project Structure](#project-structure)
- [Documentation](#documentation)
- [Development](#development)
- [Performance](#performance)
- [Roadmap](#roadmap)
- [Contributing](#contributing)
- [License](#license)

---

## Quick Start

### Prerequisites

- Rust 1.75 or newer
- System build tools (see [docs/SETUP.md](docs/SETUP.md))
- FFmpeg on `PATH` (optional; required for video export)

**Ubuntu/Debian**

```bash
sudo apt update
sudo apt install build-essential libssl-dev pkg-config cmake
# Optional: video export
sudo apt install ffmpeg
```

**macOS**

```bash
brew install cmake ffmpeg
```

### Build

```bash
git clone https://github.com/yourusername/gumol-viz-engine.git
cd gumol-viz-engine
cargo build --release
```

Faster iteration during development:

```bash
cargo build --features dev_dynamic
```

---

## Running the Application

The binary is `gumol-viz` (see `src/main.rs`).

```bash
# Open the UI; loads the first available default file if no path is given
cargo run --release

# Load a structure or trajectory from the command line
cargo run --release -- tests/fixtures/1CRN.pdb
cargo run --release -- trajectory.xyz

# DCD trajectories require a topology file
cargo run --release -- trajectory.dcd --topology structure.pdb
```

Default file search order (when no CLI path is provided):

1. `demo_trajectory.xyz`
2. `tests/fixtures/water.xyz`
3. `tests/fixtures/1CRN.pdb`

### Keyboard shortcuts

| Key | Action |
|-----|--------|
| `1`–`5` | CPK, ball-and-stick, licorice, wireframe, points |
| Click | Select atom |
| Shift+Click | Toggle atom in selection |
| Middle-mouse drag | Box selection |
| Ctrl+A | Select all atoms |
| Escape | Clear selection |
| F | Focus camera on molecule |
| Shift+F | Focus camera on selection |
| F11 | Toggle fullscreen |
| Shift+/ or F1 | Help overlay |

Full shortcut list is available in the in-app help panel.

---

## Library Usage

### Embed the plugin

```rust
use bevy::prelude::*;
use gumol_viz_engine::GumolVizPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(GumolVizPlugin)
        .run();
}
```

### Parse a file directly

```rust
use gumol_viz_engine::io::xyz::XYZParser;
use std::path::Path;

let trajectory = XYZParser::parse_file(Path::new("trajectory.xyz"))?;
println!("Loaded {} frames, {} atoms", trajectory.num_frames(), trajectory.num_atoms);
```

### Format detection

```rust
use gumol_viz_engine::io::{FileFormat, pdb::PDBParser, xyz::XYZParser};
use std::path::Path;

let path = Path::new("structure.pdb");
let trajectory = match FileFormat::from_path(path) {
    FileFormat::PDB => PDBParser::parse_file(path)?,
    FileFormat::XYZ => XYZParser::parse_file(path)?,
    _ => return Err("Unsupported format".into()),
};
```

---

## File Formats

| Format | Extension | Status | Notes |
|--------|-----------|--------|-------|
| XYZ | `.xyz` | Supported | Multi-frame trajectories; mmap + parallel parse for large files |
| PDB | `.pdb` | Supported | ATOM, HETATM, CONECT, CRYST1 |
| GRO | `.gro` | Supported | GROMACS coordinates |
| DCD | `.dcd` | Supported | Binary trajectories; requires topology (PDB/GRO) |
| mmCIF | `.cif`, `.mmcif` | Supported | Macromolecular structures |

**XYZ example**

```
3
water molecule
O 0.0 0.0 0.0
H 0.757 0.0 0.0
H -0.757 0.0 0.0
```

See [docs/SECONDARY_FORMATS.md](docs/SECONDARY_FORMATS.md) for parser details.

---

## Visualization Modes

| Mode | Description |
|------|-------------|
| CPK | Space-filling van der Waals spheres |
| Ball-and-stick | Reduced atoms with bond cylinders |
| Licorice | Small atoms with thick bonds |
| Wireframe | Line bonds between atoms |
| Points | Small point sprites at atom positions |
| Surface | Coarse solvent-accessible voxel shell |
| Cartoon / Tube / Trace | Protein backbone ribbons (requires sufficient CA atoms) |

Color schemes (CPK, residue, chain, B-factor) apply to instanced atom batches and update from the UI.

---

## Project Structure

```
gumol-viz-engine/
├── assets/                 # Shaders (e.g. atom_interpolate.wgsl), fonts
├── benches/                # Criterion benchmarks (parsing, rendering, bonds, loading)
├── docs/                   # Architecture, setup, validation, development plans
├── examples/               # Standalone example applications
├── scripts/                # Benchmark regression checker
├── src/
│   ├── main.rs             # Application entry point (gumol-viz binary)
│   ├── lib.rs              # GumolVizPlugin and public re-exports
│   ├── core/               # Atoms, bonds, molecules, trajectory, visualization types
│   ├── io/                 # Format parsers, streaming, topology, xyz_parallel
│   ├── rendering/          # Instanced pipeline, GPU interpolation, LOD, culling,
│   │                       # wireframe, ribbon, surface, material/mesh pools
│   ├── systems/            # Loading, timeline, bonds, frame cache, visualization
│   ├── performance/        # PerformanceSettings, diagnostics, memory estimates
│   ├── camera/             # Camera focus shortcuts
│   ├── interaction/        # Selection, measurements, pick proxies, box selection
│   ├── ui/                 # EGUI panels, help, notifications, atom labels
│   ├── export/             # Screenshot, OBJ, glTF, POV-Ray, video
│   └── utils/              # Colors, geometry, math, spatial index
├── tests/                  # Integration and sprint validation tests
│   └── fixtures/           # Sample structures (1CRN.pdb, water.xyz, etc.)
├── tasks/                  # Implementation todo list (tasks/todo.md)
├── AGENTS.md               # Agent/developer conventions for this repository
└── Cargo.toml
```

### Module responsibilities

| Module | Role |
|--------|------|
| `core/` | Data model: `Atom`, `Bond`, `Trajectory`, `RenderMode`, `TimelineState` |
| `io/` | Parsers for XYZ, PDB, GRO, DCD, mmCIF; async load support; `xyz_parallel` mmap path |
| `rendering/` | Production render path via `instanced.rs`; auxiliary modes in wireframe, ribbon, surface |
| `systems/` | ECS orchestration: load events, spawn/clear on reload, timeline, bonds |
| `interaction/` | Picking, selection state, measurements, middle-mouse box select |
| `ui/` | Side panel (file, timeline, visualization, export), inspector, help overlay |
| `export/` | File export pipelines and save-dialog polling |
| `performance/` | Runtime tuning knobs and diagnostic readouts for the status panel |

The instanced rendering pipeline in `rendering/instanced.rs` is the production atom draw path. Legacy per-entity spawning in `systems/spawning.rs` remains for compatibility during migration.

---

## Documentation

| Document | Description |
|----------|-------------|
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | System design and data flow |
| [docs/DEVELOPMENT_PLAN.md](docs/DEVELOPMENT_PLAN.md) | Full feature roadmap |
| [docs/SETUP.md](docs/SETUP.md) | Development environment setup |
| [docs/SECONDARY_FORMATS.md](docs/SECONDARY_FORMATS.md) | GRO, DCD, mmCIF parser notes |
| [docs/PROFILING.md](docs/PROFILING.md) | Benchmarks and profiling |
| [docs/VALIDATION.md](docs/VALIDATION.md) | Automated and manual QA checklist |
| [tasks/todo.md](tasks/todo.md) | Current implementation status |
| [AGENTS.md](AGENTS.md) | Conventions for contributors and AI agents |

Generate API docs locally:

```bash
cargo doc --no-deps --open
```

---

## Development

### Examples

```bash
cargo run --example basic_load
cargo run --example timeline_demo
cargo run --example interactive_selection
cargo run --example perf_100k -- --profile --generate-100k --profile-exit
cargo run --example xyz_viewer -- input.xyz
cargo run --example pdb_viewer -- input.pdb
```

### Tests

```bash
cargo test                    # All tests
cargo test -- --nocapture     # With output
cargo test --test sprint1_validation
cargo test --test sprint7_validation
```

Integration tests cover format loading, the instanced pipeline, plugin registration, and sprint validation suites (`tests/sprint1_validation.rs`, `tests/sprint5_validation.rs`, `tests/sprint6_validation.rs`, `tests/sprint7_validation.rs`).

### Quality checks

```bash
cargo fmt
cargo fmt -- --check
cargo clippy -- -D warnings
cargo test
cargo bench
```

Benchmark regression check:

```bash
cargo bench --bench rendering
python3 scripts/check_bench_regression.py
```

See [AGENTS.md](AGENTS.md) for naming conventions, ECS patterns, and module registration requirements.

---

## Performance

| Technique | Location |
|-----------|----------|
| Instanced rendering (one draw call per element) | `src/rendering/instanced.rs` |
| Material pooling | `src/rendering/material_pool.rs` |
| Mesh pool and LOD | `src/rendering/mesh_pool.rs`, `src/rendering/lod_system.rs` |
| Frustum culling | `src/rendering/culling.rs` |
| GPU frame interpolation | `src/rendering/gpu_interpolation.rs`, `assets/shaders/atom_interpolate.wgsl` |
| Spatial bond detection | `src/systems/bonds.rs`, `src/utils/spatial_index.rs` |
| DCD streaming and frame cache | `src/io/streaming.rs`, `src/systems/frame_cache.rs` |
| Async file loading | `src/systems/loading.rs` |
| Parallel / mmap XYZ parsing | `src/io/xyz_parallel.rs` |
| Runtime FPS + profiling validation | `src/performance/fps.rs`, UI status panel |

CPU-side 100K-atom position sync and draw-call budgets are covered by `tests/sprint1_validation.rs` and Criterion benches. Interactive GPU profiling at 100K atoms (bonds + UI + full frame) is validated in-app via CLI flags and helper scripts.

### 100K @ 60 FPS validation

Generate synthetic fixtures and run automated profiling (warmup + sampling, JSON report, pass/fail exit code):

```bash
# Static scene — 60 FPS target
./scripts/profile_100k_static.sh

# Trajectory playback — 30+ FPS target
./scripts/profile_100k_playback.sh
```

Or use CLI flags directly:

```bash
cargo run --release -- \
  --profile --profile-exit --generate-100k \
  --profile-output=target/profile_100k_static.json
```

The status panel shows live FPS (current, average, min) and profiling progress. See [docs/PROFILING.md](docs/PROFILING.md) and [docs/VALIDATION.md](docs/VALIDATION.md) for pass criteria, Chrome trace workflow, and benchmark regression gates.

---

## Roadmap

### v0.1.0 — Foundation (complete)

- Core data structures and parsers (XYZ, PDB, GRO, DCD, mmCIF)
- Instanced rendering pipeline
- Timeline playback with interpolation
- Atom selection and measurements
- Bond detection and rendering
- Screenshot, OBJ, and glTF export
- EGUI interface

### v0.2.0 — Polish (in progress)

- [x] Surface rendering mode
- [x] Runtime color schemes (CPK, residue, chain, B-factor)
- [x] Video export (FFmpeg)
- [x] POV-Ray export
- [x] Box selection and atom labels
- [x] `timeline_demo` and `interactive_selection` examples
- [x] Interactive 100K-atom @ 60 FPS validation (GPU profiling)

### v0.3.0 — Scale and I/O (in progress)

- [x] Memory-mapped and parallel XYZ loading
- [x] Double/triple bond visual meshes
- [ ] DSSP secondary structure assignment
- [ ] XYZ streaming parser
- [ ] Memory-mapped PDB loading
- [ ] Manual end-to-end QA for GRO, DCD, mmCIF in the UI

### v1.0.0 — Production

- [ ] Comprehensive user guide
- [ ] Proven large-system performance benchmarks
- [ ] Plugin system for custom visualizations

---

## Contributing

1. Read [docs/DEVELOPMENT_PLAN.md](docs/DEVELOPMENT_PLAN.md) and [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).
2. Check [tasks/todo.md](tasks/todo.md) for current priorities.
3. Fork the repository and create a feature branch.
4. Add tests where behavior changes.
5. Run `cargo fmt`, `cargo clippy -- -D warnings`, and `cargo test`.
6. Open a pull request.

---

## License

MIT License. See [LICENSE](LICENSE).

---

## Acknowledgments

- [Bevy Engine](https://bevyengine.org/) — ECS and rendering foundation
- [VMD](https://www.ks.uiuc.edu/Research/vmd/) — visualization inspiration
- [PyMOL](https://pymol.org/) — rendering mode inspiration
- [MDAnalysis](https://www.mdanalysis.org/) — file format reference
