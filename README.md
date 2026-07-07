# Gumol Viz Engine

<div align="center">

A high-performance Rust-based visualization engine for Molecular Dynamics simulations

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![Bevy](https://img.shields.io/badge/Bevy-0.14-purple.svg)](https://bevyengine.org/)

[Documentation](docs/) • [Examples](examples/) • [Architecture](docs/ARCHITECTURE.md) • [Roadmap](docs/DEVELOPMENT_PLAN.md)

</div>

## 🎯 Overview

Gumol Viz Engine provides interactive, game-like visualization of molecular structures and trajectories using GPU-accelerated instanced rendering. Built on the [Bevy](https://bevyengine.org/) game engine, it targets smooth rendering of 100,000+ atoms at 60 FPS.

### Key Features

- **🚀 High Performance**: Instanced rendering, LOD, frustum culling, GPU interpolation, spatial bond detection
- **📁 File Formats** (all loadable in the app):
  - `.xyz`, `.pdb` — primary formats
  - `.gro`, `.dcd`, `.cif` / `.mmcif` — secondary formats (DCD supports streaming)
- **🎮 Interactivity**: Orbit camera, atom selection, distance/angle/dihedral measurements
- **🎬 Timeline**: Playback with frame interpolation, scrubbing, speed control
- **🎨 Visualization Modes** (implemented):
  - CPK, Ball-and-stick, Licorice, Wireframe, Points
  - Cartoon, Tube, Trace (protein backbone ribbons)
  - Surface — planned (UI shows "coming soon")
- **📤 Export** (implemented):
  - Screenshots (PNG, JPEG)
  - 3D models (OBJ, glTF)
  - Video (MP4/WebM/GIF) via FFmpeg subprocess
- **🔬 Analysis** (implemented):
  - Distance, angle, and dihedral measurements from selected atoms
  - B-factor coloring — palette defined, runtime toggle not yet wired

## 📋 Table of Contents

- [Quick Start](#quick-start)
- [Installation](#installation)
- [Basic Usage](#basic-usage)
- [Documentation](#documentation)
- [File Formats](#file-formats)
- [Visualization Modes](#visualization-modes)
- [Development](#development)
- [Contributing](#contributing)
- [License](#license)

## 🚀 Quick Start

### Installation

```bash
git clone https://github.com/yourusername/gumol-viz-engine.git
cd gumol-viz-engine
cargo build --release
```

### Run

```bash
# Demo water molecule
cargo run --release

# Load a structure or trajectory
cargo run --release -- examples/1CRN.pdb
cargo run --release -- trajectory.xyz
```

### Basic Usage (library)

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

### Loading a File (parser API)

```rust
use gumol_viz_engine::io::xyz::XYZParser;
use std::path::Path;

let trajectory = XYZParser::parse_file(Path::new("trajectory.xyz"))?;
println!("Loaded {} frames, {} atoms", trajectory.num_frames(), trajectory.num_atoms);
```

## 📦 Installation

### Prerequisites

- **Rust** 1.75 or higher
- **Cargo** (comes with Rust)
- **FFmpeg** (optional — video export not yet implemented)

### System Dependencies

#### Ubuntu/Debian
```bash
sudo apt update
sudo apt install build-essential libssl-dev pkg-config cmake
```

#### macOS
```bash
brew install cmake
```

#### Windows
- Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/)

### Building from Source

```bash
cargo build              # debug
cargo build --release    # optimized
cargo test               # unit + integration tests
cargo bench              # performance benchmarks

# Faster dev rebuilds (dynamic linking)
cargo build --features dev_dynamic
```

## 🎓 Basic Usage

### Loading a Trajectory

```rust
use gumol_viz_engine::io::{FileFormat, xyz::XYZParser, pdb::PDBParser};
use std::path::Path;

let path = Path::new("simulation.pdb");
let format = FileFormat::from_path(path);

let trajectory = match format {
    FileFormat::PDB => PDBParser::parse_file(path)?,
    FileFormat::XYZ => XYZParser::parse_file(path)?,
    FileFormat::GRO => gumol_viz_engine::io::gro::GroParser::parse_file(path)?,
    _ => return Err("Unsupported format".into()),
};
```

### Visualization Settings

```rust
use gumol_viz_engine::core::visualization::{RenderMode, VisualizationStyle};

let style = VisualizationStyle::new(RenderMode::BallAndStick);
```

## 📚 Documentation

- **[Architecture Guide](docs/ARCHITECTURE.md)** — system design and data flow
- **[Development Plan](docs/DEVELOPMENT_PLAN.md)** — full roadmap spec
- **[Secondary Formats](docs/SECONDARY_FORMATS.md)** — GRO, DCD, mmCIF details
- **[Profiling Guide](docs/PROFILING.md)** — benchmarks and hotspot analysis
- **[Setup Guide](docs/SETUP.md)** — development environment
- **[Task List](tasks/todo.md)** — current implementation status
- **[Validation Report](docs/VALIDATION.md)** — Sprint 1 automated + manual QA checklist

```bash
cargo doc --no-deps --open
```

## 📁 File Formats

| Format | Extension | Status | Notes |
|--------|-----------|--------|-------|
| XYZ | `.xyz` | ✅ Full | Multi-frame trajectories |
| PDB | `.pdb` | ✅ Full | ATOM, HETATM, CONECT, CRYST1 |
| GRO | `.gro` | ✅ Full | GROMACS coordinates |
| DCD | `.dcd` | ✅ Full | Binary trajectories; requires topology (PDB/GRO) |
| mmCIF | `.cif`, `.mmcif` | ✅ Full | Macromolecular structures |

#### XYZ example
```
3
water molecule
O 0.0 0.0 0.0
H 0.757 0.0 0.0
H -0.757 0.0 0.0
```

## 🎨 Visualization Modes

| Mode | Status | Description |
|------|--------|-------------|
| CPK | ✅ | Space-filling van der Waals spheres |
| Ball-and-stick | ✅ | Reduced atoms + bond cylinders |
| Licorice | ✅ | Small atoms, thick bonds |
| Wireframe | ✅ | Line bonds between atoms |
| Points | ✅ | Small point sprites |
| Cartoon / Tube / Trace | ✅ | Protein backbone ribbons (≥20 CA residues) |
| Surface | 🔜 | Solvent-accessible surface — not yet implemented |

## 📊 Performance

### Optimization Techniques (implemented)

| Technique | Module |
|-----------|--------|
| Instanced rendering | `rendering/instanced.rs` |
| Material pooling | `rendering/material_pool.rs` |
| LOD mesh selection | `rendering/lod_system.rs` |
| Frustum culling | `rendering/culling.rs` |
| GPU frame interpolation | `rendering/gpu_interpolation.rs` |
| Spatial bond detection | `systems/bonds.rs`, `utils/spatial_index.rs` |
| DCD streaming + frame cache | `io/streaming.rs`, `systems/frame_cache.rs` |
| Async file loading | `systems/loading.rs` |

### Benchmarks

Criterion benchmarks are in `benches/`. Run and compare against baseline:

```bash
cargo bench --bench rendering
python3 scripts/check_bench_regression.py
```

Formal 100K-atom @ 60 FPS validation is tracked in `tasks/todo.md` (architecture in place; proof pending).

## 💻 Development

### Project Structure

```
gumol-viz-engine/
├── docs/                  # Documentation
├── examples/              # Example applications
├── src/
│   ├── core/             # Data structures
│   ├── io/               # File parsers + streaming
│   ├── rendering/        # Instanced pipeline, LOD, wireframe, ribbon
│   ├── systems/          # Loading, timeline, bonds, frame cache
│   ├── performance/      # Settings and diagnostics
│   ├── camera/           # Focus shortcuts
│   ├── interaction/      # Selection, measurements
│   ├── ui/               # EGUI interface
│   ├── export/           # Screenshot, OBJ, glTF
│   └── utils/            # Geometry, colors, spatial index
├── tests/                # Integration tests
├── benches/              # Performance benchmarks
└── Cargo.toml
```

### Running Examples

```bash
cargo run --example basic_load
cargo run --example xyz_viewer -- input.xyz
cargo run --example pdb_viewer -- input.pdb
```

> `timeline_demo` and `interactive_selection` examples are planned but not yet added.

### Quality Checks

```bash
cargo fmt
cargo clippy -- -D warnings
cargo test
```

## 🤝 Contributing

Read the [Development Plan](docs/DEVELOPMENT_PLAN.md) and [Architecture Guide](docs/ARCHITECTURE.md) before starting. See [tasks/todo.md](tasks/todo.md) for current priorities.

1. Fork the repository
2. Create a feature branch
3. Make changes with tests
4. Run `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test`
5. Open a Pull Request

## 🗺️ Roadmap

### Version 0.1.0 — Foundation ✅
- [x] Core data structures
- [x] XYZ, PDB, GRO, DCD, mmCIF parsers
- [x] Instanced rendering pipeline
- [x] Timeline playback with interpolation
- [x] Atom selection and measurements
- [x] Bond detection and rendering
- [x] Multiple visualization modes (except Surface)
- [x] Screenshot, OBJ, glTF export
- [x] EGUI interface

### Version 0.2.0 — Polish (in progress)
- [x] Surface rendering mode (coarse SAS voxel shell)
- [x] Runtime color schemes (CPK, Residue, Chain, B-factor)
- [x] Video export (FFmpeg; `cargo run` — requires `ffmpeg` on PATH)
- [ ] POV-Ray export
- [ ] Box selection and atom labels
- [ ] `timeline_demo` and `interactive_selection` examples
- [ ] 100K-atom performance validation
- [ ] Fix headless plugin registration test

### Version 0.3.0 — Scale & I/O
- [ ] Memory-mapped / streamed XYZ loading
- [ ] Parallel trajectory parsing (rayon)
- [ ] DSSP secondary structure
- [ ] Double/triple bond visual meshes

### Version 1.0.0 — Production
- [ ] All planned features complete
- [ ] Comprehensive user guide
- [ ] Proven performance benchmarks
- [ ] Plugin system

## 📄 License

MIT License — see [LICENSE](LICENSE).

## 🙏 Acknowledgments

- **[Bevy Engine](https://bevyengine.org/)** — rendering and ECS foundation
- **[VMD](https://www.ks.uiuc.edu/Research/vmd/)** — visualization inspiration
- **[PyMOL](https://pymol.org/)** — rendering mode inspiration
- **[MDAnalysis](https://www.mdanalysis.org/)** — file format reference

---

<div align="center">

Built with ❤️ and 🦀 Rust

[⬆ Back to top](#gumol-viz-engine)

</div>
