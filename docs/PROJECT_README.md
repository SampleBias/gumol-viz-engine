# Gumol Viz Engine - Development Context

## Project Purpose
A high-performance Rust-based visualization engine for Molecular Dynamics (MD) simulations using the Bevy game engine. Designed for interactive, game-like visualization of molecular structures with GPU acceleration.

## Architecture Overview
- **Project Type**: Scientific Visualization / Molecular Dynamics
- **Development Status**: Early Development - Foundation complete, systems implementation needed
- **Development Framework**: Bevy 0.14 ECS (Entity-Component-System)
- **Context Tracking**: Integrated with Vybrid development workflow

## Technology Stack
- **Language**: Rust 1.75+
- **Game Engine**: Bevy 0.14 (ECS-based, GPU-accelerated rendering)
- **UI Framework**: EGUI 0.28 (immediate-mode GUI)
- **3D Interaction**: bevy_mod_picking 0.20, bevy_panorbit_camera 0.19
- **Math**: nalgebra 0.33
- **Parsing**: nom 7.1 (parser combinators)
- **Parallel Processing**: rayon 1.10
- **Error Handling**: thiserror 1.0, anyhow 1.0

## Project Structure
```
gumol-viz-engine/
├── src/
│   ├── core/           # Core data structures (atom, bond, molecule, trajectory)
│   ├── io/             # File format parsers (xyz.rs, pdb.rs implemented)
│   ├── rendering/      # Rendering systems (mesh generation)
│   ├── systems/        # Bevy ECS systems (stubs, needs implementation)
│   ├── camera/         # Camera controls (stubs)
│   ├── interaction/    # User interaction (stubs)
│   ├── ui/             # EGUI systems (stubs)
│   ├── export/         # Export functionality (stubs)
│   └── utils/          # Utility functions (geometry, colors, math)
├── examples/           # Example applications
├── docs/               # Documentation (DEVELOPMENT_PLAN.md, ARCHITECTURE.md, etc.)
└── tasks/              # Development task tracking
```

## Current Implementation Status

### ✅ Completed Features
- **Core Data Structures**: Complete implementations of Atom, Element, Bond, Molecule, Trajectory, FrameData, TimelineState
- **XYZ Parser**: Fully functional with streaming support for large files
- **PDB Parser**: Complete with ATOM, HETATM, CONECT record parsing
- **Secondary Parsers**: GRO, DCD, and mmCIF formats fully implemented
- **Element System**: All 118 elements with CPK colors, van der Waals radii, atomic masses
- **Mesh Generation**: Basic sphere (atom) and cylinder (bond) mesh generation
- **Bevy Plugin Structure**: GumolVizPlugin with module registration
- **File Loading System**: Complete event-driven loading system with CLI, drag-drop, and file picker support
- **Atom Spawning System**: Entity spawning from trajectory data with position updates and picking support
- **Timeline & Animation**: Complete playback system with interpolation, speed control, keyboard/UI controls
- **UI System**: EGUI-based interface with file loading, status display, timeline controls, and selection info
- **Atom Selection**: Complete selection system with raycasting, highlighting, and multi-select support
- **Bond Detection**: Distance-based bond detection with automatic spawning
- **Visualization Modes**: CPK, Ball-and-Stick, Licorice, Wireframe, Surface, Cartoon, Tube, Trace, Points
- **Export Systems**: Screenshot, OBJ, and glTF export functionality

### ⚠️ Performance Issues Identified (June 2025)
**CRITICAL:** Comprehensive GPU performance analysis revealed major bottlenecks:

- **No Instanced Rendering**: Each atom = separate draw call (10,000 atoms = 10,000 draw calls)
- **GPU Utilization**: <10% (should be >80%)
- **CPU Position Updates**: Timeline interpolation done on CPU, transferred to GPU every frame
- **Synchronous File Loading**: Blocks UI thread (100K atom PDB = 5-10 second freeze)
- **O(N²) Bond Detection**: Checks every atom against every other atom
- **No Spatial Acceleration**: No octree/BVH for queries
- **No Frustum Culling**: All atoms rendered regardless of camera view
- **High-Poly Meshes Everywhere**: No level-of-detail system

**Impact:**
- 10K atoms: 10-30 FPS (unusable for smooth interaction)
- 100K atoms: <1 FPS (completely unusable)
- Load times: 500ms - 5s with UI freeze

**Solution Path:** See `docs/GPU_PERFORMANCE_ANALYSIS.md` and `docs/QUICK_START_OPTIMIZATION.md`

**Expected Improvements:**
- Instanced rendering: 100-1000x performance gain
- GPU compute updates: 10-50x faster animation
- Async loading: Eliminates UI freezes
- 10K atoms: 10 FPS → 200+ FPS
- 100K atoms: <1 FPS → 60+ FPS

### 🔨 In Progress / Stubs
- **Camera Controls**: Using bevy_panorbit_camera, custom controls stubbed
- **Atom Selection**: Module exists but raycasting selection not implemented
- **Bond Rendering**: Bond detection system not yet implemented
- **Visualization Modes**: Only basic CPK rendering implemented
- **Measurement Tools**: Distance/angle/dihedral measurement not implemented
- **Export Systems**: Screenshot/video export not implemented

### ❌ Not Implemented
- Bond detection and rendering
- Atom interaction (raycasting, selection highlighting)
- Multiple visualization modes (ball-and-stick, licorice)
- Measurement tools (distance, angle, dihedral)
- Export functionality (screenshots, videos)
- Secondary file formats (GRO, DCD, mmCIF)
- Surface generation
- Cartoon representation

## Getting Started

### Prerequisites
- Rust 1.75 or higher
- Cargo (comes with Rust)
- For video export: FFmpeg (optional)

### Installation
```bash
git clone <repository-url>
cd gumol-viz-engine
cargo build --release
```

### Running the Project
```bash
# Run the main demo (water molecule)
cargo run --release

# Run examples (need to be implemented)
cargo run --example basic_load
cargo run --example xyz_viewer
cargo run --example pdb_viewer
```

### Current Demo
The main application currently shows:
- A 3D scene with a water molecule (H2O)
- Orbit camera controls (mouse drag to rotate, scroll to zoom)
- Red oxygen atom with two white hydrogen atoms
- Cylindrical bonds connecting atoms
- Point and directional lighting
- F11 toggles fullscreen

## Development Status

### Current Phase: Atom Selection Complete
- ✅ Project structure established
- ✅ Dependencies configured in Cargo.toml
- ✅ Core data structures implemented
- ✅ Primary file parsers (XYZ, PDB) working
- ✅ Basic mesh generation functional
- ✅ Bevy app and demo scene working
- ✅ File loading system (CLI, drag-drop, file picker)
- ✅ Atom spawning system with position updates and picking
- ✅ Timeline & Animation system with interpolation
- ✅ UI system with file loading, timeline, and selection controls
- ✅ Atom selection system with raycasting and highlighting

### Next Priority Phase: Secondary File Formats (Phase 1 - Week 1-2)
According to `tasks/todo.md`, the next priorities are:
1. **GRO Parser** - GROMACS coordinate format parser
2. **DCD Parser** - CHARMM trajectory format parser
3. **mmCIF Parser** - macromolecular Crystallographic Information File parser
4. **Testing** - Unit tests and integration tests for all formats

### Task Tracking
- See `tasks/todo.md` for detailed task breakdown and progress
- Tasks organized by priority (HIGH/MEDIUM/LOW)
- 7 major phases identified

### Activity History
- See `docs/activity.md` for detailed development timeline

## Key Context for AI Agents

### Development Workflow
- This project follows the Vybrid development methodology
- Three mandatory files maintained: `tasks/todo.md`, `docs/activity.md`, `docs/PROJECT_README.md`
- All development activities tracked and documented systematically
- Tasks executed immediately - no approval waiting required

### Code Style & Standards
- Use `cargo fmt` for formatting
- Pass `cargo clippy -- -D warnings`
- Comprehensive unit tests for new functionality
- Add documentation to public APIs
- Follow Rust idioms and Bevy best practices

### Bevy-Specific Guidelines
- Use ECS pattern: Entities with Components processed by Systems
- Resources for global state (TimelineState, SelectionState, SimulationData)
- Plugin architecture for modular organization
- Component derive macros (Component, Reflect, Default)
- System ordering using `.chain()` for dependencies

### Performance Targets
- 100,000+ atoms at 60 FPS
- Handle trajectories with 10,000+ frames
- Support multi-gigabyte files via streaming
- GPU-accelerated rendering with instancing

### File Format Priority
1. **Primary** (Complete): XYZ, PDB
2. **Secondary** (Not started): GRO, DCD, mmCIF

## Project Evolution
- **Initial Setup**: 2026-02-23 12:54 (project structure files)
- **Codebase Review**: 2026-02-23 13:00 (comprehensive analysis)
- **Context Update**: 2026-02-23 13:05 (this update)
- **Major Changes**: Foundation complete, transitioning to system implementation

## Documentation Links
- [Task List](../tasks/todo.md) - Current development tasks with priorities
- [Activity Log](activity.md) - Detailed timeline of all development activities
- [Development Plan](DEVELOPMENT_PLAN.md) - Full 10-week development roadmap
- [Architecture Guide](ARCHITECTURE.md) - System architecture diagrams
- [Setup Guide](SETUP.md) - Environment setup instructions
- [README](../README.md) - Project overview and quick start

## Important Notes
- Project compiles successfully with only minor warnings
- Parsers are well-tested and functional
- The main gap is between data loading and visualization (the "glue" systems)
- Focus on connecting existing data structures to Bevy rendering pipeline
- Test with real molecular files as features are implemented

---
*Last Updated: 2026-02-23 13:05*
*Context Version: 2.0*
*Development Phase: System Implementation*


---

## Session Update - 2026-02-23 13:57
- **Session Started**: 2026-02-23 13:57
- **Context Status**: Verified and up-to-date

*Context automatically updated for new development session*


---

## Session Update - 2026-02-25 14:51
- **Session Started**: 2026-02-25 14:51
- **Context Status**: Verified and up-to-date

*Context automatically updated for new development session*


---

## Session Update - 2026-02-28 09:21
- **Session Started**: 2026-02-28 09:21
- **Context Status**: Verified and up-to-date

*Context automatically updated for new development session*
