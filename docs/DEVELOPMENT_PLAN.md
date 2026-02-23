# Gumol Visualization Engine - Development Plan

## Overview
A Rust-based visualization engine for Molecular Dynamics (MD) simulations using the Bevy game engine. Engineered for high-performance, interactive rendering of molecular structures with game-like fluidity.

## Technology Stack

### Core Dependencies
```toml
[dependencies]
bevy = "0.15"              # Game engine (ECS, rendering, input)
bevy_egui = "0.29"         # UI overlay for controls
bevy_mod_picking = "0.21"  # 3D object selection
bevy_panorbit_camera = "0.20" # Camera controls
rayon = "1.10"             # Parallel processing
nalgebra = "0.33"          # Linear algebra for geometry
rustc-hash = "2.0"         # Fast hashing
memmap2 = "0.9"            # Memory-mapped files for large trajectories
nom = "7.1"                # Parser combinators
byteorder = "1.5"          # Binary file parsing
bytemuck = "1.18"          # Zero-copy casting
```

### File Format Parsers
```toml
# Format-specific crates
quick-xml = "0.36"         # mmCIF XML-like format
```

---

## Architecture

### Bevy ECS Architecture

**Systems (parallel execution):**
1. **File Loading System** - Parse trajectory files
2. **Atom Spawning System** - Create atom entities with components
3. **Bond Detection System** - Calculate bonds from distance
4. **Rendering System** - Update meshes, materials, transforms
5. **Timeline System** - Frame interpolation and playback
6. **Camera Control System** - Orbit/pan interactions
7. **Selection System** - Raycasting for atom selection
8. **Measurement System** - Distance/angle calculations
9. **Export System** - Screenshots, movies, custom outputs

**Components (data attached to entities):**
- `Atom` - atom type, residue, position
- `Bond` - bond type, order, atoms involved
- `Molecule` - collection of atoms
- `TrajectoryFrame` - frame data
- `VisualizationStyle` - rendering mode (CPK, licorice, etc.)
- `Label` - text labels
- `Selection` - currently selected atoms

**Resources (global state):**
- `SimulationData` - loaded trajectory data
- `CameraSettings` - zoom, position, target
- `TimelineState` - current frame, playback state
- `VisualizationConfig` - global rendering settings
- `SelectionState` - selected atoms list
- `FileHandle` - currently loaded file

---

## Project Structure

```
gumol-viz-engine/
├── Cargo.toml
├── assets/
│   ├── shaders/           # Custom shaders
│   └── fonts/             # UI fonts
├── src/
│   ├── main.rs            # Entry point, plugin registration
│   ├── lib.rs             # Library interface
│   │
│   ├── core/              # Core systems
│   │   ├── mod.rs
│   │   ├── atom.rs        # Atom/bond components
│   │   ├── molecule.rs    # Molecule data structures
│   │   └── trajectory.rs  # Timeline/frame management
│   │
│   ├── io/                # File I/O
│   │   ├── mod.rs
│   │   ├── xyz.rs         # XYZ format parser
│   │   ├── pdb.rs         # PDB format parser
│   │   ├── gro.rs         # GROMACS GRO format
│   │   ├── dcd.rs         # CHARMM DCD format
│   │   └── mmcif.rs       # mmCIF format parser
│   │
│   ├── rendering/         # Bevy rendering systems
│   │   ├── mod.rs
│   │   ├── atom_mesh.rs   # Sphere mesh generation
│   │   ├── bond_mesh.rs   # Cylinder mesh for bonds
│   │   ├── materials.rs   # CPK colors, custom materials
│   │   └── shaders.rs     # Custom shader definitions
│   │
│   ├── systems/           # Bevy systems
│   │   ├── mod.rs
│   │   ├── loading.rs     # File loading system
│   │   ├── spawning.rs    # Entity spawning
│   │   ├── bonds.rs       # Bond calculation
│   │   ├── timeline.rs    # Animation/playback
│   │   └── update.rs      # Per-frame updates
│   │
│   ├── camera/            # Camera controls
│   │   ├── mod.rs
│   │   ├── orbit.rs       # Orbit controls
│   │   ├── fly.rs         # Fly-through mode
│   │   └── zoom.rs        # Zoom to selection
│   │
│   ├── interaction/       # User interaction
│   │   ├── mod.rs
│   │   ├── selection.rs   # Atom selection via raycast
│   │   ├── measurement.rs # Distance/angle tools
│   │   └── manipulation.rs # Rotate/translate selections
│   │
│   ├── ui/                # GUI systems
│   │   ├── mod.rs
│   │   ├── timeline_ui.rs # Timeline controls
│   │   ├── inspector.rs   # Atom/molecule inspector
│   │   ├── settings.rs    # Rendering settings
│   │   └── export.rs      # Export UI
│   │
│   ├── export/            # Output systems
│   │   ├── mod.rs
│   │   ├── screenshot.rs  # Image capture
│   │   ├── video.rs       # Movie export (FFmpeg)
│   │   └── formats/       # Custom format export
│   │   │   ├── mod.rs
│   │   │   ├── povray.rs  # POV-Ray scene export
│   │   │   ├── obj.rs     # OBJ mesh export
│   │   │   └── gltf.rs    # glTF 3D model export
│   │
│   └── utils/             # Utilities
│       ├── mod.rs
│       ├── geometry.rs    # Mesh generation utilities
│       ├── colors.rs      # CPK color palette
│       └── math.rs        # Vector math helpers
│
└── examples/              # Example scenes
    ├── basic_load.rs
    ├── timeline_demo.rs
    └── interactive_selection.rs
```

---

## Module Specifications

### 1. File I/O Module (`src/io/`)

#### Primary Formats (Must support perfectly)

**XYZ Parser (`xyz.rs`)**
```rust
pub struct XYZParser;

impl XYZParser {
    pub fn parse<R: Read>(reader: R) -> Result<Vec<AtomFrame>>;

    pub fn parse_mmap(path: &Path) -> Result<Vec<AtomFrame>>;
}

// Streamed parser for large files
pub struct XYZStreamer;
impl XYZStreamer {
    pub fn new(path: &Path) -> Result<Self>;
    pub fn next_frame(&mut self) -> Option<Result<AtomFrame>>;
}
```

**PDB Parser (`pdb.rs`)**
```rust
pub struct PDBParser;

impl PDBParser {
    pub fn parse<R: Read>(reader: R) -> Result<PDBData>;

    pub fn parse_trajectory<R: Read>(reader: R) -> Result<Vec<AtomFrame>>;
}

// Handles ATOM, HETATM, CONECT, CRYST1 records
pub struct PDBRecord;
pub struct PDBData {
    pub atoms: Vec<Atom>,
    pub bonds: Vec<Bond>,
    pub crystal_info: Option<CrystalInfo>,
}
```

#### Secondary Formats (Good to have)

**GRO Parser (`gro.rs`)**
```rust
pub struct GroParser;

impl GroParser {
    pub fn parse<R: Read>(reader: R) -> Result<Vec<AtomFrame>>;
}
```

**DCD Parser (`dcd.rs`)**
```rust
pub struct DCDParser;

impl DCDParser {
    pub fn parse<R: Read>(reader: R) -> Result<Trajectory>;
    pub fn parse_mmap(path: &Path) -> Result<Trajectory>;
}

pub struct Trajectory {
    pub header: DCDHeader,
    pub frames: Vec<FrameData>,
    pub is_mmap: bool,
}
```

**mmCIF Parser (`mmcif.rs`)**
```rust
pub struct MMCIFParser;

impl MMCIFParser {
    pub fn parse<R: Read>(reader: R) -> Result<MMCIFData>;
}

pub struct MMCIFData {
    pub atoms: Vec<Atom>,
    pub bonds: Vec<Bond>,
    pub metadata: HashMap<String, String>,
}
```

### 2. Core Data Structures (`src/core/`)

```rust
// Atom component attached to entities
#[derive(Component, Clone)]
pub struct Atom {
    pub id: u32,
    pub element: Element,
    pub position: Vec3,
    pub residue_id: u32,
    pub residue_name: String,
    pub chain_id: String,
    pub b_factor: f32,
}

#[derive(Clone)]
pub enum Element {
    H, He, Li, Be, B, C, N, O, F, Ne, Na, Mg, Al, Si, P, S, Cl,
    // ... rest of periodic table
}

#[derive(Component)]
pub struct Bond {
    pub atom_a: Entity,
    pub atom_b: Entity,
    pub bond_type: BondType,
    pub order: u8,  // single, double, triple
}

#[derive(Component)]
pub struct Molecule {
    pub name: String,
    pub atoms: Vec<Entity>,
}

// Timeline state
#[derive(Resource)]
pub struct TimelineState {
    pub current_frame: usize,
    pub total_frames: usize,
    pub is_playing: bool,
    pub playback_speed: f32,
    pub loop_playback: bool,
}
```

### 3. Rendering Systems (`src/rendering/`)

**Atom Mesh Generation**
```rust
pub fn generate_atom_mesh(element: Element, radius: f32) -> Mesh {
    // Generate UV sphere with appropriate resolution
    // Pre-defined radii for each element (CPK)
}

pub const ATOM_RADII: [f32; 118] = [
    1.20,  // H
    1.40,  // He
    1.82,  // Li
    // ... etc
];
```

**Bond Mesh Generation**
```rust
pub fn generate_bond_mesh(pos_a: Vec3, pos_b: Vec3, radius: f32) -> Mesh {
    // Generate cylinder connecting two atoms
}

pub fn generate_double_bond_mesh(pos_a: Vec3, pos_b: Vec3, radius: f32) -> Vec<Mesh>;
pub fn generate_triple_bond_mesh(pos_a: Vec3, pos_b: Vec3, radius: f32) -> Vec<Mesh>;
```

**Materials**
```rust
pub fn get_atom_color(element: Element) -> Color {
    match element {
        Element::H => Color::rgb(0.9, 0.9, 0.9),    // white
        Element::C => Color::rgb(0.2, 0.2, 0.2),    // gray
        Element::N => Color::rgb(0.1, 0.1, 0.8),    // blue
        Element::O => Color::rgb(0.8, 0.1, 0.1),    // red
        // ... CPK colors
    }
}
```

### 4. Timeline/Animation System (`src/systems/timeline.rs`)

```rust
pub fn update_timeline(
    time: Res<Time>,
    mut timeline: ResMut<TimelineState>,
    sim_data: Res<SimulationData>,
    mut atom_query: Query<&mut Transform, With<Atom>>,
) {
    if !timeline.is_playing {
        return;
    }

    // Update current frame based on playback speed
    timeline.current_frame = (timeline.current_frame +
        (time.delta_seconds() * timeline.playback_speed) as usize)
        % timeline.total_frames;

    // Interpolate positions for smooth animation
    let t = timeline.current_frame as f32;
    let frame = &sim_data.frames[timeline.current_frame];

    for (atom, transform) in atom_query.iter_mut() {
        if let Some(pos) = frame.positions.get(&atom.id) {
            transform.translation = *pos;
        }
    }
}

pub fn interpolate_frames(
    frame_a: &FrameData,
    frame_b: &FrameData,
    alpha: f32,
) -> Vec<Vec3> {
    // Linear interpolation between frames
    // alpha is 0.0 to 1.0
}
```

### 5. Camera Control System (`src/camera/`)

```rust
#[derive(Component)]
pub struct OrbitCamera {
    pub focus: Vec3,
    pub distance: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub min_distance: f32,
    pub max_distance: f32,
}

pub fn update_orbit_camera(
    time: Res<Time>,
    mut camera_q: Query<(&mut OrbitCamera, &mut Transform)>,
    mouse: Res<Input<MouseButton>>,
    keyboard: Res<Input<KeyCode>>,
) {
    // Mouse drag to rotate
    // Scroll to zoom
    // Middle-click to pan
}
```

### 6. Selection & Interaction (`src/interaction/`)

```rust
pub fn select_atom(
    mut commands: Commands,
    mouse: Res<Input<MouseButton>>,
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    atom_q: Query<(Entity, &Transform, &Atom)>,
    mut selection: ResMut<SelectionState>,
) {
    // Raycast from camera to find clicked atom
    // Add/remove from selection
}

pub fn measure_distance(
    selection: Res<SelectionState>,
    atom_q: Query<&Atom>,
) -> Option<f32> {
    // Calculate distance between selected atoms
}

pub fn measure_angle(
    selection: Res<SelectionState>,
    atom_q: Query<&Atom>,
) -> Option<f32> {
    // Calculate angle between three selected atoms
}
```

### 7. Export System (`src/export/`)

**Screenshot Export**
```rust
pub fn capture_screenshot(
    mut images: ResMut<Assets<Image>>,
    windows: Query<&Window>,
) {
    // Capture current frame and save as PNG
}
```

**Video Export**
```rust
pub fn start_recording(settings: VideoSettings);
pub fn add_frame_recording(image: &Image);
pub fn finish_recording() -> Result<()>;

pub struct VideoSettings {
    pub format: VideoFormat,  // MP4, WebM, GIF
    pub fps: u32,
    pub quality: u32,
    pub codec: VideoCodec,
}
```

**POV-Ray Export**
```rust
pub fn export_povray(
    scene: &SceneData,
    output: &mut impl Write,
) -> Result<()> {
    // Generate POV-Ray SDL scene file
}
```

**OBJ Export**
```rust
pub fn export_obj(
    scene: &SceneData,
    output: &mut impl Write,
) -> Result<()> {
    // Export meshes as OBJ format
}
```

**glTF Export**
```rust
pub fn export_gltf(
    scene: &SceneData,
    output: &Path,
) -> Result<()> {
    // Export as glTF 2.0
}
```

---

## System Schedule

```rust
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(bevy_egui::EguiPlugin)
        .add_plugins(bevy_mod_picking::DefaultPickingPlugins)
        .add_plugins(bevy_panorbit_camera::PanOrbitCameraPlugin)

        // Resources
        .init_resource::<TimelineState>()
        .init_resource::<SelectionState>()
        .init_resource::<VisualizationConfig>()

        // Startup systems
        .add_systems(Startup, load_simulation_file)
        .add_systems(Startup, spawn_atoms)

        // Main loop systems (in order)
        .add_systems(Update,
            (
                timeline::update_timeline,
                loading::stream_trajectory_frame,
            )
            .chain()
        )
        .add_systems(Update,
            (
                spawning::update_atom_positions,
                bonds::update_bonds,
                camera::update_camera,
            )
            .chain()
        )
        .add_systems(Update,
            (
                interaction::select_atom,
                interaction::measure_distance,
            )
        )
        .add_systems(Update,
            ui::timeline_ui,
            ui::inspector_ui,
            ui::settings_ui,
        )

        .run();
}
```

---

## Development Phases

### Phase 1: Foundation (Week 1-2)
- [ ] Set up Bevy project structure
- [ ] Implement XYZ parser
- [ ] Implement PDB parser (single frame)
- [ ] Create basic atom spawning system
- [ ] Implement CPK color scheme
- [ ] Basic camera orbit controls
- [ ] Sphere mesh generation for atoms

### Phase 2: Trajectories & Animation (Week 3)
- [ ] Implement multi-frame trajectory support
- [ ] Timeline system with playback
- [ ] Frame interpolation for smooth animation
- [ ] Streamed parsing for large files (memory-mapped)
- [ ] Timeline UI controls

### Phase 3: Bonds & Visualization (Week 4)
- [ ] Bond detection from distance
- [ ] PDB CONECT record parsing
- [ ] Cylinder mesh generation for bonds
- [ ] Multiple visualization modes (CPK, licorice, ball-and-stick)
- [ ] Residue-based coloring

### Phase 4: Interaction (Week 5)
- [ ] Raycasting for atom selection
- [ ] Selection highlighting
- [ ] Distance measurement tool
- [ ] Angle measurement tool
- [ ] Dihedral angle measurement
- [ ] Selection UI

### Phase 5: Secondary File Formats (Week 6)
- [ ] GRO parser
- [ ] DCD parser (binary)
- [ ] mmCIF parser
- [ ] Format detection from extension

### Phase 6: Export (Week 7)
- [ ] Screenshot capture (PNG/JPEG)
- [ ] Video recording with FFmpeg
- [ ] POV-Ray scene export
- [ ] OBJ mesh export
- [ ] glTF 3D model export

### Phase 7: Advanced Features (Week 8)
- [ ] Surface generation (solvent accessible surface)
- [ ] Ribbon representation for proteins
- [ ] Cartoon representation
- [ ] Isosurface rendering (density maps)
- [ ] Volume rendering
- [ ] Custom shaders for advanced effects

### Phase 8: Performance & Polish (Week 9-10)
- [ ] Instanced rendering for thousands of atoms
- [ ] Level-of-detail (LOD) for large systems
- [ ] Frustum culling
- [ ] Multi-threaded trajectory loading
- [ ] GPU compute for frame interpolation
- [ ] Documentation
- [ ] Examples and tutorials

---

## Rendering Modes

### 1. CPK (Space-filling)
- Atoms as spheres with CPK radii
- Van der Waals surface representation
- No bonds shown (implied by atom overlap)

### 2. Ball-and-Stick
- Atoms as smaller spheres (50% CPK radius)
- Bonds as cylinders connecting atoms
- Clear structural representation

### 3. Licorice
- Atoms as very small spheres
- Bonds as thick cylinders
- Backbone-focused representation

### 4. Cartoon (Proteins)
- Ribbon following backbone
- Secondary structure coloring
- Smooth spline interpolation

### 5. Surface
- Solvent accessible surface
- Molecular surface (Connolly)
- Translucent rendering

### 6. Wireframe
- Lines only
- Useful for large systems

---

## Performance Optimizations

1. **Instanced Rendering**: One draw call for identical atoms
2. **GPU-Driven Animation**: Compute shaders for frame interpolation
3. **Level-of-Detail**: Low-poly meshes for distant atoms
4. **Frustum Culling**: Don't render atoms outside view
5. **Spatial Partitioning**: Octree for efficient raycasting
6. **Memory Mapping**: Stream trajectories from disk
7. **Parallel Parsing**: Multi-threaded trajectory loading
8. **Mesh Caching**: Pre-generate atom/bond meshes
9. **Texture Atlases**: Batch materials for fewer draw calls

---

## UI Features (EGUI)

### Timeline Panel
- Play/Pause button
- Frame scrubbing slider
- Playback speed control
- Frame counter display
- Loop toggle

### Inspector Panel
- Selected atom details (element, residue, coordinates)
- Measurement display (distance, angles)
- B-factor coloring toggle

### Settings Panel
- Rendering mode selector
- Atom size scaling
- Bond thickness
- Background color
- Lighting settings

### Export Panel
- Screenshot button
- Video recording controls
- Format export buttons

---

## Potential Extensions

- **VR Support**: Use OpenXR for immersive viewing
- **Remote Rendering**: Stream to web browser
- **Plugins System**: User-defined analysis tools
- **Python Bindings**: Interoperability with existing workflows
- **Real-time Analysis**: Live RMSD, RMSF calculations
- **Trajectory Editing**: Cut, splice, merge trajectories
- **Path Following**: Camera follows atom or molecule
- **Automated Movies**: Keyframe animation system

---

## Testing Strategy

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xyz_parser() {
        let data = "3\nwater\nO 0.0 0.0 0.0\nH 0.0 0.9 0.0\nH 0.0 -0.9 0.0";
        let result = XYZParser::parse(data.as_bytes()).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].atoms.len(), 3);
    }

    #[test]
    fn test_bond_detection() {
        // Test distance-based bond detection
    }

    #[test]
    fn test_frame_interpolation() {
        // Test interpolation accuracy
    }
}
```

---

## Documentation Structure

1. **User Guide**: How to use the application
2. **Developer Guide**: How to contribute
3. **API Reference**: Rust documentation
4. **File Format Specs**: Supported formats details
5. **Examples**: Interactive examples

---

## Success Criteria

- ✓ Load and visualize >100,000 atoms smoothly (60 FPS)
- ✓ Handle multi-frame trajectories with >10,000 frames
- ✓ Support all specified file formats
- ✓ Provide game-like interactive controls
- ✓ Export to multiple formats
- ✓ Extensible plugin architecture
- ✓ Comprehensive documentation
- ✓ Active development for 6+ months

---

## Questions for Approval

1. **Priority**: Should secondary formats (.gro, .dcd, mmCIF) be included in initial release or later phases?

2. **VR Support**: Is VR integration desired, or should we focus on desktop first?

3. **GPU Requirements**: What is the minimum GPU spec you want to target?

4. **Trajectory Size**: What's the largest trajectory (atoms × frames) you expect to visualize?

5. **Export Formats**: Any additional export formats beyond POV-Ray, OBJ, glTF you need?

6. **Plugin System**: How important is a user-defined plugin system?

7. **Python Integration**: Do you need Python bindings for integration with existing workflows?

8. **Timeline Features**: Do you need advanced features like multiple timelines, markers, or frame annotations?

---

*Ready for implementation upon approval. This plan provides a solid foundation for building a production-ready visualization engine.*
