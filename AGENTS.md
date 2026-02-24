# AGENTS.md

This document helps AI agents work effectively in the Gumol Viz Engine repository.

## Project Overview

Gumol Viz Engine is a high-performance Rust-based visualization engine for Molecular Dynamics simulations using the Bevy game engine. It provides GPU-accelerated rendering of molecular structures with game-like interactivity.

**Key Features:**
- Multi-format file support (XYZ, PDB primary; GRO, DCD, mmCIF planned)
- 100,000+ atoms @ 60 FPS with GPU acceleration
- Timeline animation with frame interpolation
- Multiple visualization modes (CPK, ball-and-stick, licorice, surface)
- Interactive selection, measurements, and camera controls
- Export capabilities (screenshots, videos, POV-Ray, OBJ, glTF)

## Essential Commands

### Building
```bash
# Debug build (faster compile, slower runtime)
cargo build

# Release build (optimized, slower compile)
cargo build --release

# Dynamic linking for faster dev builds (feature-flagged)
cargo build --features dev_dynamic
```

### Running
```bash
# Run main application
cargo run

# Run specific example
cargo run --example basic_load
cargo run --example xyz_viewer -- input.xyz
cargo run --example pdb_viewer -- input.pdb

# Run with logging
RUST_LOG=debug cargo run
RUST_LOG=info cargo run
```

### Testing
```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_parse_simple_xyz

# Run benchmarks
cargo bench

# Run integration tests only
cargo test --test integration_test
```

### Code Quality
```bash
# Format code
cargo fmt

# Check formatting without modifying
cargo fmt -- --check

# Run linter (treat warnings as errors)
cargo clippy -- -D warnings

# Run linter without treating warnings as errors
cargo clippy

# Generate documentation
cargo doc --open
cargo doc --no-deps --open

# Check for security vulnerabilities (requires cargo-audit)
cargo audit
```

### Development Tools
```bash
# Watch for changes and rebuild
cargo watch -x run
cargo watch -x test

# Install useful dev tools
cargo install cargo-watch
cargo install cargo-edit
cargo install cargo-expand
cargo install cargo-audit
```

## Code Organization

### Directory Structure
```
gumol-viz-engine/
├── src/
│   ├── core/              # Core data structures (atoms, bonds, molecules, trajectory)
│   ├── io/                # File format parsers (XYZ, PDB, etc.)
│   ├── rendering/         # Bevy rendering systems and mesh generation
│   ├── systems/           # Bevy ECS systems (loading, spawning, bonds, timeline)
│   ├── camera/            # Camera controls
│   ├── interaction/       # User interaction (selection, measurements)
│   ├── ui/                # EGUI interface components
│   ├── export/            # Export functionality (screenshots, videos, 3D models)
│   ├── utils/             # Utility functions (colors, geometry, math)
│   ├── lib.rs             # Library entry point, re-exports, plugin registration
│   └── main.rs            # Binary entry point
├── examples/              # Example applications
├── tests/                 # Integration tests
├── benches/               # Performance benchmarks
├── assets/                # Shaders, fonts
├── docs/                  # Project documentation
└── Cargo.toml             # Project configuration
```

### Module Pattern

Each module follows this pattern:

```rust
//! Module documentation

// Internal modules
pub mod submodule;

// Dependencies
use bevy::prelude::*;

// Public types
pub struct MyComponent;

// Public functions
pub fn do_something() -> Result<(), Error> {
    // Implementation
}

// Register function (called by main plugin)
pub fn register(app: &mut App) {
    app.init_resource::<MyResource>()
        .add_systems(Update, my_system);

    info!("Module registered");
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_something() {
        // Test implementation
    }
}
```

**Critical:** All modules must have a `register(app: &mut App)` function that registers their systems, resources, and events with the Bevy app.

### Bevy ECS Architecture

The project uses Bevy's Entity-Component-System (ECS) architecture:

- **Entities**: Unique identifiers for atoms, bonds, molecules, etc.
- **Components**: Data attached to entities (Atom, Bond, Transform, etc.)
- **Systems**: Functions that operate on queries of components
- **Resources**: Global state (TimelineState, SelectionState, etc.)
- **Events**: One-way communication between systems (FileLoadedEvent, AtomsSpawnedEvent, etc.)

**Common Queries:**
```rust
// Simple query
fn my_system(query: Query<&Atom>) {
    for atom in query.iter() {
        // Process atoms
    }
}

// Mutating query
fn my_system(mut query: Query<&mut Transform>) {
    for mut transform in query.iter_mut() {
        transform.translation += Vec3::X;
    }
}

// Filtered query
fn my_system(query: Query<&Atom, With<SpawnedAtom>>) {
    // Only atoms that have SpawnedAtom component
}

// Complex query with multiple components
fn my_system(
    query: Query<(&Atom, &Transform, &mut Visibility)>,
) {
    for (atom, transform, mut visibility) in query.iter() {
        // Access multiple components
    }
}
```

## Naming Conventions

### Types
```rust
// Structs and enums: PascalCase
pub struct AtomData;
pub enum Element;
pub struct XYZParser;
```

### Functions and Methods
```rust
// Functions: snake_case
pub fn parse_file(path: &Path) -> Result<Trajectory>;
pub fn register(app: &mut App);
```

### Constants
```rust
// Constants: SCREAMING_SNAKE_CASE
pub const MAX_ATOMS: usize = 1_000_000;
pub const DEFAULT_CAMERA_DISTANCE: f32 = 20.0;
```

### Acronyms (Non-standard!)
```rust
// IMPORTANT: Acronyms use PascalCase, not all-caps
pub struct PdbParser;      // NOT PDBParser
pub struct XyzWriter;      // NOT XYZWriter
pub struct MmCIF;        // NOT MMCIF
```

### Components, Resources, Events
```rust
// Components: PascalCase with descriptive name
#[derive(Component)]
pub struct SpawnedAtom { pub atom_id: u32 }

#[derive(Resource)]
pub struct AtomEntities { pub entities: HashMap<u32, Entity> }

#[derive(Event)]
pub struct AtomsSpawnedEvent { pub count: usize }
```

## Code Style Patterns

### Error Handling

Use `Result<T, E>` for fallible operations, never panic in production code:

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum IOError {
    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Parse error at line {line}: {message}")]
    ParseError { line: usize, message: String },
}

pub type IOResult<T> = Result<T, IOError>;

pub fn parse_file(path: &Path) -> IOResult<Trajectory> {
    let file = File::open(path)
        .map_err(|e| IOError::FileNotFound(path.display().to_string()))?;
    // Continue parsing...
    Ok(trajectory)
}
```

### Logging

Use the `tracing` crate (configured via `tracing_subscriber`):

```rust
use bevy::prelude::*;

// Info level - general information
info!("Loaded {} atoms", atom_count);

// Debug level - detailed debugging
debug!("Atom {} at position {:?}", atom_id, position);

// Warn level - warnings
warn!("Unknown element: {}, using Unknown", element_symbol);

// Error level - errors (should still be recoverable)
error!("Failed to parse file: {}", error);
```

### Bevy System Patterns

```rust
// Startup system (runs once at app start)
fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    info!("Setting up scene...");
    // Spawn entities, create resources, etc.
}

// Update system (runs every frame)
fn update_timeline(
    time: Res<Time>,
    mut timeline: ResMut<TimelineState>,
) {
    if !timeline.is_playing {
        return;
    }
    // Update timeline state
}

// Event-driven system
fn on_file_loaded(
    mut events: EventReader<FileLoadedEvent>,
    mut commands: Commands,
) {
    for event in events.read() {
        info!("File loaded: {:?}", event.path);
        // Spawn atoms, etc.
    }
}

// Register systems
pub fn register(app: &mut App) {
    app.add_systems(Startup, setup_scene)
        .add_systems(Update, update_timeline)
        .add_event::<FileLoadedEvent>();
}
```

## Testing Patterns

### Unit Tests

Place unit tests in the same module as the code:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_element_from_symbol() {
        assert_eq!(Element::from_symbol("C").unwrap(), Element::C);
        assert_eq!(Element::from_symbol("H").unwrap(), Element::H);
        assert!(Element::from_symbol("XX").is_err());
    }

    #[test]
    fn test_parse_simple_xyz() {
        let xyz_content = r#"3
water
O 0.0 0.0 0.0
H 0.757 0.0 0.0
H -0.757 0.0 0.0"#;

        let result = XYZParser::parse_string(
            xyz_content,
            PathBuf::from("test.xyz"),
        );

        assert!(result.is_ok());
        let trajectory = result.unwrap();
        assert_eq!(trajectory.num_frames(), 1);
        assert_eq!(trajectory.num_atoms, 3);
    }
}
```

### Integration Tests

Place integration tests in the `tests/` directory:

```rust
// tests/integration_test.rs
use gumol_viz_engine::GumolVizEngine;

#[test]
fn test_plugin_registration() {
    let mut app = App::new();
    app.add_plugins(GumolVizEngine);
    // Verify plugin registered correctly
}
```

### Property-Based Testing

Use `proptest` for property-based tests:

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_element_symbol_roundtrip(element_str in "[A-Za-z]{1,2}") {
        // Test that parsing and converting back to symbol works
        if let Ok(element) = Element::from_symbol(&element_str) {
            assert_eq!(element.symbol().to_uppercase(), element_str.to_uppercase());
        }
    }
}
```

## Important Gotchas

### Bevy Version
- Project uses **Bevy 0.14** (check `Cargo.toml` - some docs mention 0.15 but actual is 0.14)
- Bevy APIs can change significantly between versions
- Always check Bevy 0.14 documentation: https://docs.rs/bevy/0.14/

### Mesh Generation
- Atoms are rendered as spheres using **UV sphere** mesh generation
- Bonds are rendered as **cylinders** with caps
- Custom mesh generation in `src/rendering/mod.rs` (not using external mesh libraries)
- Mesh resolution: 16 latitudes, 32 longitudes for spheres; 16 segments for cylinders

### Atom Visualization
- Atoms are rendered at **50% of VDW radius** for visibility (see `spawning.rs:52`)
- Uses **CPK color scheme** for element coloring
- CPK colors are defined in `src/core/atom.rs` in `Element::cpk_color()`
- All common elements (H, C, N, O, etc.) have predefined colors

### Timeline System
- Frame positions are stored in `FrameData` with positions mapped by atom ID
- Timeline state tracked in `TimelineState` resource
- `update_atom_positions` system updates entity transforms from current frame
- Frame interpolation is planned but not yet fully implemented

### File Format Support
- **Primary formats (fully supported)**: XYZ, PDB
- **Secondary formats (planned)**: GRO, DCD, mmCIF
- Format detection via `FileFormat::from_path()` (extension-based)
- Format validation via `FileFormat::is_loadable()` (check if parser implemented)
- Each parser implements `parse_file()`, `parse_reader()`, and `parse_string()`

### Resource and Asset Management
- Meshes stored in `ResMut<Assets<Mesh>>` resource
- Materials stored in `ResMut<Assets<StandardMaterial>>` resource
- Always use `.clone()` when reusing meshes/materials across entities
- Assets are managed by Bevy's asset system (reference counting)

### Error Handling in Parsers
- All parsers use `IOResult<T>` type alias for `Result<T, IOError>`
- Detailed error messages include line numbers for parse errors
- Unknown elements default to `Element::Unknown` with a warning (not an error)
- Missing/invalid coordinate fields cause immediate parse failure

### Component Organization
- **`Atom` component**: Runtime atom data (position, b_factor, occupancy)
- **`AtomData` struct**: Static atom metadata (element, residue, name, mass)
- **`SpawnedAtom` component**: Marker component linking entity to atom ID
- Use `SpawnedAtom` marker to filter spawned atoms in queries

### Performance Considerations
- Target: 100,000+ atoms @ 60 FPS
- Optimization techniques planned: instanced rendering, level-of-detail, frustum culling
- Memory mapping planned for large trajectory files (multi-GB)
- Spatial partitioning (octree) planned for efficient raycasting

### Rendering Pipeline
- Uses Bevy PBR (Physically Based Rendering) materials
- Lighting: PointLight + DirectionalLight + AmbientLight
- Shadows enabled on light sources
- Present mode: `AutoVsync` for smooth rendering
- Window: 1920x1080 default, resizable

### Event Communication
- Systems communicate via Bevy's event system
- Common events: `FileLoadedEvent`, `AtomsSpawnedEvent`, `LoadFileEvent`
- Events are **fire-and-forget** (no response guaranteed)
- Use `EventReader` to read events, `EventWriter` to send events
- Events are processed in system order (not guaranteed frame order)

## Project-Specific Constants

### From `lib.rs`
```rust
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const MAX_ATOMS: usize = 1_000_000;
pub const MAX_FRAMES: usize = 100_000;
pub const DEFAULT_CAMERA_DISTANCE: f32 = 20.0;
pub const MIN_CAMERA_DISTANCE: f32 = 1.0;
pub const MAX_CAMERA_DISTANCE: f32 = 1000.0;
```

### Element Data (from `src/core/atom.rs`)
- 118 elements defined (H through Lr, plus Unknown)
- CPK colors defined for common elements
- VDW radii defined for first ~100 elements
- Atomic masses defined for first ~100 elements
- Element symbols support 1-2 character codes

## Key Dependencies

### Core Dependencies
- **bevy** 0.14 - Game engine (ECS, rendering, input)
- **bevy_egui** 0.28 - UI overlay (EGUI)
- **bevy_mod_picking** 0.20 - 3D object picking (raycasting)
- **bevy_panorbit_camera** 0.19 - Orbit camera controls

### Data Processing
- **rayon** 1.10 - Parallel processing
- **nalgebra** 0.33 - Linear algebra
- **rustc-hash** 2.0 - Fast hashing
- **ahash** 0.8 - Fast HashMap implementation

### File I/O
- **nom** 7.1 - Parser combinators
- **memmap2** 0.9 - Memory-mapped files
- **byteorder** 1.5 - Binary file parsing
- **quick-xml** 0.36 - XML parsing (mmCIF)

### Error Handling & Serialization
- **thiserror** 1.0 - Error derivation
- **anyhow** 1.0 - Error context
- **serde** 1.0 - Serialization framework
- **serde_json** 1.0 - JSON serialization

### Development Tools
- **tracing** 0.1 - Logging
- **tracing-subscriber** 0.3 - Log formatting
- **criterion** 0.5 - Benchmarking (dev-dependency)
- **proptest** 1.5 - Property-based testing (dev-dependency)

## Build Profiles

### Development (`cargo build`)
- `opt-level = 1` - Some optimization for faster builds
- Dependencies at `opt-level = 3` for performance
- No stripping (full debug symbols)

### Release (`cargo build --release`)
- `opt-level = 3` - Maximum optimization
- `lto = "thin"` - Link-time optimization
- `codegen-units = 1` - Best optimization, slower compile
- `strip = true` - Remove debug symbols
- `panic = "abort"` - Smaller binary (no unwinding code)

### Dev Dynamic (`cargo build --features dev_dynamic`)
- Bevy dynamic linking for faster recompilation
- Inherits from dev profile
- Requires `panic = "unwind"` for dynamic_linking compatibility

## Documentation Standards

### Public API Documentation
All public APIs must have doc comments:

```rust
/// Parse an XYZ file and return trajectory data
///
/// # Arguments
///
/// * `path` - Path to XYZ file
///
/// # Returns
///
/// Returns a `Result` containing `Trajectory` or an error
///
/// # Errors
///
/// Returns an error if file cannot be read or is invalid
///
/// # Examples
///
/// ```
/// use gumol_viz_engine::io::xyz::XYZParser;
///
/// let trajectory = XYZParser::parse_file("water.xyz")?;
/// println!("Loaded {} frames", trajectory.num_frames());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn parse_file(path: &Path) -> IOResult<Trajectory> {
    // Implementation
}
```

### Module Documentation
Each module must have a module-level doc comment explaining its purpose:

```rust
//! Atom component and data structures
//!
//! This module defines the `Atom` component and related types
//! for representing atomic data in the molecular visualization.
//!
//! ## Components
//!
//! - [`Atom`] - Runtime atom data attached to entities
//! - [`AtomData`] - Static atom metadata
//!
//! ## Elements
//!
//! The [`Element`] enum represents chemical elements from the
//! periodic table, with methods for properties like CPK colors,
//! VDW radii, and atomic masses.
```

## Common Workflows

### Adding a New File Format Parser

1. Create parser file: `src/io/<format>.rs`
2. Implement parser struct with `parse_file()`, `parse_reader()`, `parse_string()`
3. Register parser in `src/io/mod.rs`: `pub mod <format>;`
4. Add to `FileFormat` enum in `src/io/mod.rs`
5. Add to `FileFormat::from_path()` detection
6. Add tests in parser file
7. Update documentation

### Adding a New Visualization Mode

1. Add mode to `RenderMode` enum in `src/core/visualization.rs`
2. Implement rendering system in `src/rendering/` or `src/systems/visualization.rs`
3. Add UI controls in `src/ui/mod.rs`
4. Update documentation and examples
5. Add tests for new mode

### Adding a New System

1. Define system function with appropriate queries/resources
2. Add event types if needed
3. Register system in module's `register()` function
4. Add tests for system behavior
5. Update system scheduling in main plugin if needed

### Creating an Example

1. Create file in `examples/` directory
2. Implement Bevy app with GumolVizPlugin
3. Add example to `Cargo.toml` `[[example]]` section
4. Test with `cargo run --example <name>`
5. Document usage in README

## Development Checklist

Before committing changes, ensure:

- [ ] Code formatted: `cargo fmt`
- [ ] No clippy warnings: `cargo clippy -- -D warnings`
- [ ] All tests pass: `cargo test`
- [ ] Documentation updated for public APIs
- [ ] Examples still work
- [ ] No new `cargo build` warnings
- [ ] Follow naming conventions (especially acronyms)
- [ ] Appropriate error handling (Result types, not panics)
- [ ] Logging added for important operations
- [ ] Tests added for new functionality

## Performance Profiling

```bash
# Benchmarking
cargo bench

# Flamegraph (requires cargo-flamegraph)
cargo flamegraph --example basic_load

# Measure compile time
cargo build --timings

# Profile with standard tools
perf record --call-graph=dwarf cargo run --release
```

## Platform-Specific Notes

### Linux
- Install system dependencies: `sudo apt install build-essential libssl-dev pkg-config cmake`
- Install FFmpeg for video export: `sudo apt install ffmpeg`

### macOS
- Install via Homebrew: `brew install cmake ffmpeg`

### Windows
- Install Visual Studio Build Tools
- Install FFmpeg manually
- May need additional setup for Bevy's X11 feature on WSL

## Resources

### Internal Documentation
- `README.md` - Project overview and quick start
- `docs/ARCHITECTURE.md` - System architecture and design decisions
- `docs/DEVELOPMENT_PLAN.md` - Detailed development roadmap
- `docs/SETUP.md` - Development environment setup
- `CONTRIBUTING.md` - Contribution guidelines

### External Resources
- Bevy 0.14 Documentation: https://docs.rs/bevy/0.14/
- Bevy Learn Book: https://bevyengine.org/learn/book/
- Rust API Guidelines: https://rust-lang.github.io/api-guidelines/
- Rust Book: https://doc.rust-lang.org/book/

## Module-Specific Notes

### Core (`src/core/`)
- **`atom.rs`**: Atom, AtomData, Element enum with properties
- **`bond.rs`**: Bond component and data structures
- **`molecule.rs`**: Molecule component
- **`trajectory.rs`**: FrameData, Trajectory, TimelineState
- **`visualization.rs`**: RenderMode, VisualizationStyle

### IO (`src/io/`)
- **`xyz.rs`**: XYZ format parser and writer
- **`pdb.rs`**: PDB format parser
- All parsers support `parse_file()`, `parse_reader()`, `parse_string()`
- Streaming support for large files

### Systems (`src/systems/`)
- **`loading.rs`**: File loading system, LoadFileEvent, SimulationData resource
- **`spawning.rs`**: Atom spawning, SpawnedAtom marker, AtomEntities resource
- **`bonds.rs`**: Bond detection and rendering
- **`timeline.rs`**: Timeline playback and animation
- **`visualization.rs`**: Visualization mode switching

### Rendering (`src/rendering/`)
- **`generate_atom_mesh()`**: UV sphere generation
- **`generate_bond_mesh()`**: Cylinder with caps generation
- Manual mesh generation (no external mesh libraries)

### Interaction (`src/interaction/`)
- **`selection.rs`**: Atom selection via raycasting
- Uses `bevy_mod_picking` for 3D object picking

### Utils (`src/utils/`)
- **`colors.rs`**: Color utilities
- **`geometry.rs`**: Geometry helpers
- **`math.rs`**: Math functions
