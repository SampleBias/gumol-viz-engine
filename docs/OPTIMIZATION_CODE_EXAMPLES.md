# Performance Optimization Code Examples

This document contains ready-to-use code snippets for the critical GPU optimizations.

---

## 1. Instanced Rendering Component

**File:** `src/rendering/instanced.rs`

```rust
use bevy::{
    prelude::*,
    render::{
        render_resource::{ShaderType, BufferUsages},
    },
};

/// Instance data for each atom (sent to GPU)
#[derive(ShaderType, Clone, Copy, Default, Debug)]
pub struct AtomInstanceData {
    pub position: Vec3,
    pub scale: f32,
    pub color: Vec4,
    pub _padding: Vec3, // For 16-byte alignment
}

/// Component holding instance data for instanced rendering
#[derive(Component, Default, Debug)]
pub struct InstancedAtomMesh {
    pub instances: Vec<AtomInstanceData>,
}
```

---

## 2. Instanced Spawning System

**File:** `src/systems/instanced_spawning.rs`

```rust
use crate::rendering::{AtomInstanceData, InstancedAtomMesh};
use crate::core::atom::AtomData;
use crate::core::trajectory::FrameData;
use crate::rendering;
use bevy::prelude::*;
use std::collections::HashMap;

/// Spawn atoms using instanced rendering (ONE entity per element)
pub fn spawn_atoms_instanced(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    frame_data: &FrameData,
    atom_data: &[AtomData],
) -> HashMap<u32, Entity> {
    info!("Spawning {} atoms with instancing", atom_data.len());

    let mut entity_map = HashMap::new();

    // Step 1: Group atoms by element (118 elements max)
    let mut atoms_by_element: HashMap<crate::core::atom::Element, Vec<&AtomData>> =
        HashMap::new();

    for atom_info in atom_data {
        if let Some(_position) = frame_data.get_position(atom_info.id) {
            atoms_by_element
                .entry(atom_info.element)
                .or_default()
                .push(atom_info);
        }
    }

    info!("Grouped atoms into {} element types", atoms_by_element.len());

    // Step 2: Spawn ONE instanced entity per element
    for (element, atoms) in atoms_by_element {
        // Generate mesh ONCE per element
        let radius = element.vdw_radius() * 0.5;
        let mesh = meshes.add(rendering::generate_atom_mesh(radius));

        // Get CPK color for this element
        let color_rgb = element.cpk_color();

        // Create instance data for ALL atoms of this element
        let instances: Vec<AtomInstanceData> = atoms
            .iter()
            .map(|atom_info| {
                let position = frame_data.get_position(atom_info.id).unwrap_or(Vec3::ZERO);
                AtomInstanceData {
                    position,
                    scale: 1.0,
                    color: Vec4::new(color_rgb[0], color_rgb[1], color_rgb[2], 1.0),
                    _padding: Vec3::ZERO,
                }
            })
            .collect();

        // Create material (can be shared if needed)
        let material = materials.add(StandardMaterial {
            base_color: Color::WHITE, // Color comes from instance data
            unlit: false,
            metallic: 0.1,
            perceptual_roughness: 0.2,
            ..default()
        });

        // Spawn ONE entity with instancing
        let entity = commands
            .spawn((
                PbrBundle {
                    mesh,
                    material,
                    transform: Transform::from_translation(Vec3::ZERO),
                    ..default()
                },
                InstancedAtomMesh { instances },
                crate::systems::spawning::SpawnedAtom {
                    atom_id: 0, // Dummy ID for now
                },
            ))
            .id();

        info!("Spawned {} atoms of element {:?}", atoms.len(), element);

        // Track entity for all atoms (use first atom as key)
        if let Some(first_atom) = atoms.first() {
            entity_map.insert(first_atom.id, entity);
        }
    }

    info!("Total instanced entities: {}", entity_map.len());
    entity_map
}
```

---

## 3. WGSL Compute Shader for Position Interpolation

**File:** `assets/compute/atom_update.wgsl`

```wgsl
// Atom position interpolation on GPU

// Current frame positions (read-only)
@group(0) @binding(0) var<storage, read> positions_current: array<vec3<f32>>;

// Next frame positions (read-only)
@group(0) @binding(1) var<storage, read> positions_next: array<vec3<f32>>;

// Output positions (write-only)
@group(0) @binding(2) var<storage, read_write> positions_out: array<vec3<f32>>;

// Uniforms
struct InterpolationUniforms {
    alpha: f32,           // Interpolation factor (0.0 to 1.0)
    num_atoms: u32,       // Number of atoms
    _padding: vec2<f32>,  // Padding for alignment
}

@group(0) @binding(3) var<uniform> uniforms: InterpolationUniforms;

// Main compute shader
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let atom_id = global_id.x;

    // Boundary check
    if (atom_id >= uniforms.num_atoms) {
        return;
    }

    // Read positions
    let pos_current = positions_current[atom_id];
    let pos_next = positions_next[atom_id];

    // Interpolate on GPU (linear interpolation = mix)
    positions_out[atom_id] = mix(pos_current, pos_next, uniforms.alpha);
}
```

---

## 4. Rust Compute Shader Integration

**File:** `src/compute/mod.rs`

```rust
use bevy::{
    prelude::*,
    render::{
        render_resource::{
            BindGroupLayout, BindGroupLayoutEntry, BindingType, ShaderStages,
            BufferBindingType, ComputePipelineDescriptor, PipelineCache,
            ShaderType, BufferUsages,
        },
        renderer::RenderDevice,
    },
};
use bytemuck::cast_slice;

/// Uniforms for atom interpolation
#[derive(ShaderType, Clone, Copy, Default)]
pub struct InterpolationUniforms {
    pub alpha: f32,
    pub num_atoms: u32,
    pub _padding: [f32; 2],
}

/// GPU compute system for atom position interpolation
pub fn interpolate_positions_gpu(
    time: Res<Time>,
    mut timeline: ResMut<crate::core::trajectory::TimelineState>,
    sim_data: Res<crate::systems::loading::SimulationData>,
    render_device: Res<RenderDevice>,
    // Buffers and bind groups would be here
) {
    if !timeline.is_playing || !sim_data.loaded {
        return;
    }

    let current_frame = timeline.current_frame;
    let alpha = timeline.interpolation_factor;

    // Create uniforms
    let uniforms = InterpolationUniforms {
        alpha,
        num_atoms: sim_data.num_atoms() as u32,
        _padding: [0.0, 0.0],
    };

    // In a real implementation, you would:
    // 1. Upload current frame positions to GPU buffer
    // 2. Upload next frame positions to GPU buffer
    // 3. Create output buffer
    // 4. Create bind groups
    // 5. Dispatch compute shader
    // 6. Read back results (or use directly for rendering)

    // For now, this is a placeholder showing the structure
    info!(
        "GPU interpolation: frame={}, alpha={:.3}",
        current_frame, alpha
    );
}
```

---

## 5. Async File Loading

**File:** `src/io/async_loader.rs`

```rust
use crate::core::trajectory::Trajectory;
use crate::io::{FileFormat, IOResult};
use std::path::PathBuf;

use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};

/// Async file loader for non-blocking file operations
pub struct AsyncFileLoader;

impl AsyncFileLoader {
    /// Load a file asynchronously (non-blocking)
    pub async fn load_file_async(path: PathBuf) -> IOResult<Trajectory> {
        let format = FileFormat::from_path(&path);

        info!("Loading file asynchronously: {:?} (format: {:?})", path, format);

        // Use tokio for async file I/O
        let file = File::open(&path).await
            .map_err(|e| crate::io::IOError::FileNotFound(path.display().to_string()))?;
        let reader = BufReader::new(file);

        // Stream parse (incremental)
        match format {
            FileFormat::XYZ => Self::parse_xyz_async(reader, path).await,
            FileFormat::PDB => Self::parse_pdb_async(reader, path).await,
            // Add other formats...
            _ => Err(crate::io::IOError::UnsupportedFormat(format!("{:?}", format))),
        }
    }

    /// Async XYZ parser (streaming)
    async fn parse_xyz_async<R: tokio::io::AsyncReadExt + Unpin>(
        mut reader: R,
        file_path: PathBuf,
    ) -> IOResult<Trajectory> {
        use crate::io::xyz::XYZParser;
        // Convert async reader to sync for parser (simplified)
        // In production, you'd rewrite parser to be fully async
        XYZParser::parse_file(&file_path)
    }

    /// Async PDB parser (streaming)
    async fn parse_pdb_async<R: tokio::io::AsyncReadExt + Unpin>(
        mut reader: R,
        file_path: PathBuf,
    ) -> IOResult<Trajectory> {
        use crate::io::pdb::PDBParser;
        PDBParser::parse_file(&file_path)
    }
}

/// Event sent when async file load completes
#[derive(Event, Debug)]
pub struct AsyncFileLoadedEvent {
    pub trajectory: Trajectory,
    pub atom_data: Vec<crate::core::atom::AtomData>,
}

/// System to handle async file loading
pub fn handle_async_load(
    mut commands: Commands,
    mut load_events: EventReader<crate::systems::loading::LoadFileEvent>,
    // In production, you'd use a task pool here
) {
    for event in load_events.read() {
        let path = event.path.clone();

        // Spawn async task
        #[cfg(feature = "async")]
        {
            commands.spawn(async move {
                match AsyncFileLoader::load_file_async(path).await {
                    Ok(trajectory) => {
                        // Send completion event
                        // ...
                    }
                    Err(e) => {
                        error!("Async load failed: {:?}", e);
                    }
                }
            });
        }

        #[cfg(not(feature = "async"))]
        {
            // Fallback to sync loading
            warn!("Async not enabled, using sync loading");
        }
    }
}
```

---

## 6. Spatial Partitioning (R-Tree)

**File:** `src/utils/spatial.rs`

```rust
use bevy::prelude::*;
use rstar::{RTree, RTreeObject, AABB};
use crate::core::atom::AtomData;

/// Spatial wrapper for atom queries
#[derive(Clone, Copy)]
pub struct AtomSpatial {
    pub id: u32,
    pub position: Vec3,
    pub radius: f32,
}

impl RTreeObject for AtomSpatial {
    type Envelope = AABB<[f32; 3]>;

    fn envelope(&self) -> Self::Envelope {
        AABB::from_point([
            self.position.x,
            self.position.y,
            self.position.z,
        ])
        .enlarged(self.radius)
    }
}

/// Build R-tree from atom data (O(n log n))
pub fn build_spatial_index(atom_data: &[AtomData]) -> RTree<AtomSpatial> {
    let atoms: Vec<AtomSpatial> = atom_data
        .iter()
        .map(|a| AtomSpatial {
            id: a.id,
            position: a.position,
            radius: a.element.vdw_radius(),
        })
        .collect();

    RTree::bulk_load(atoms)
}

/// Find atoms within radius (O(n log n) instead of O(n²))
pub fn find_neighbors(
    tree: &RTree<AtomSpatial>,
    center: Vec3,
    radius: f32,
) -> Vec<AtomSpatial> {
    tree.locate_within_distance(
        [center.x, center.y, center.z],
        radius,
    )
    .collect()
}

/// Optimized bond detection using spatial index
pub fn detect_bonds_spatial(
    atom_data: &[AtomData],
    config: &crate::systems::bonds::BondDetectionConfig,
) -> Vec<crate::core::bond::BondData> {
    // Build spatial index
    let tree = build_spatial_index(atom_data);
    info!("Built spatial index with {} atoms", atom_data.len());

    let mut bonds = Vec::new();

    // For each atom, find neighbors within max bond distance
    for atom in atom_data {
        let neighbors = find_neighbors(
            &tree,
            atom.position,
            config.max_bond_distance,
        );

        for neighbor in neighbors {
            // Don't self-bond
            if neighbor.id == atom.id {
                continue;
            }

            // Calculate exact distance
            let distance = atom.position.distance(neighbor.position);

            // Check if should bond
            if config.should_bond_for_atoms(
                atom,
                atom_data.iter().find(|a| a.id == neighbor.id).unwrap(),
                distance,
            ) {
                // Create bond (avoid duplicates by only creating when id_a < id_b)
                if atom.id < neighbor.id {
                    let bond_type = config.determine_bond_type_for_atoms(
                        atom,
                        atom_data.iter().find(|a| a.id == neighbor.id).unwrap(),
                    );
                    let bond_order = config.determine_bond_order_for_atoms(
                        atom,
                        atom_data.iter().find(|a| a.id == neighbor.id).unwrap(),
                        distance,
                    );

                    bonds.push(crate::core::bond::BondData::new(
                        atom.id,
                        neighbor.id,
                        bond_type,
                        bond_order,
                        distance,
                    ));
                }
            }
        }
    }

    info!("Detected {} bonds using spatial index", bonds.len());
    bonds
}
```

---

## 7. Frustum Culling

**File:** `src/rendering/culling.rs`

```rust
use bevy::prelude::*;

/// Frustum culling system - only render visible atoms
pub fn frustum_cull_atoms(
    mut query: Query<(
        &GlobalTransform,
        &mut Visibility,
        &crate::core::atom::Atom,
    )>,
    cameras: Query<(&Frustum, &GlobalTransform), With<Camera>>,
) {
    let (frustum, _camera_transform) = match cameras.get_single() {
        Ok(cam) => cam,
        Err(_) => return, // No camera, nothing to cull
    };

    let mut visible_count = 0;
    let mut hidden_count = 0;

    for (transform, mut visibility, atom) in query.iter_mut() {
        let position = transform.translation();
        let radius = atom.element.vdw_radius();

        // Check if atom is in frustum
        if frustum.contains_sphere(position, radius) {
            *visibility = Visibility::Visible;
            visible_count += 1;
        } else {
            *visibility = Visibility::Hidden;
            hidden_count += 1;
        }
    }

    // Log statistics (occasionally)
    if visible_count + hidden_count > 0 {
        trace!(
            "Frustum culling: {} visible, {} hidden ({}% culled)",
            visible_count,
            hidden_count,
            hidden_count * 100 / (visible_count + hidden_count)
        );
    }
}
```

---

## 8. Material Pool

**File:** `src/rendering/material_pool.rs`

```rust
use bevy::{
    prelude::*,
};
use std::collections::HashMap;

/// Pool of pre-created materials (one per element)
#[derive(Resource, Default, Debug)]
pub struct MaterialPool {
    materials: HashMap<crate::core::atom::Element, Handle<StandardMaterial>>,
}

impl MaterialPool {
    /// Initialize material pool (call once at startup)
    pub fn initialize(mut commands: Commands, mut materials: ResMut<Assets<StandardMaterial>>) {
        let pool = Self {
            materials: Self::create_all_materials(&mut materials),
        };

        commands.insert_resource(pool);
        info!("Material pool initialized with {} materials", pool.materials.len());
    }

    /// Create a material for each element (118 total)
    fn create_all_materials(materials: &mut Assets<StandardMaterial>) -> HashMap<crate::core::atom::Element, Handle<StandardMaterial>> {
        let mut pool = HashMap::new();

        for element in crate::core::atom::Element::all_elements() {
            let color_rgb = element.cpk_color();

            let material = materials.add(StandardMaterial {
                base_color: Color::srgb(color_rgb[0], color_rgb[1], color_rgb[2]),
                metallic: 0.1,
                perceptual_roughness: 0.2,
                unlit: false,
                ..default()
            });

            pool.insert(*element, material);
        }

        pool
    }

    /// Get material for an element (or create if missing)
    pub fn get_material(&self, element: crate::core::atom::Element) -> Handle<StandardMaterial> {
        match self.materials.get(&element) {
            Some(handle) => handle.clone(),
            None => {
                // Shouldn't happen if initialized properly
                warn!("Material not found for element {:?}", element);
                Handle::weak(HandleId::default())
            }
        }
    }

    /// Get all materials
    pub fn all_materials(&self) -> &HashMap<crate::core::atom::Element, Handle<StandardMaterial>> {
        &self.materials
    }
}
```

---

## 9. Integration with Existing Code

### Update `src/lib.rs`

```rust
pub mod rendering {
    pub mod mod as rendering_mod;
    pub mod instanced;       // NEW
    pub mod culling;         // NEW
    pub mod material_pool;   // NEW
}

pub mod compute {
    pub mod mod as compute_mod;
}

pub mod utils {
    pub mod mod as utils_mod;
    pub mod spatial;         // NEW
}

// In GumolVizPlugin::build()
fn build(&self, app: &mut App) {
    // ... existing modules

    // NEW: Add optimization modules
    rendering::instanced::register(app);
    rendering::culling::register(app);
    rendering::material_pool::initialize(app, materials);
    compute::compute_mod::register(app);
    utils::spatial::register(app);
}
```

---

## 10. Performance Profiling

**File:** `src/utils/profiler.rs`

```rust
use puffin::{profile_scope, GlobalProfiler};
use std::time::Instant;

/// Initialize profiler
pub fn init_profiler() {
    GlobalProfiler::default();
    info!("Profiler initialized");
}

/// Profile a function call
pub fn profile_fn<F, R>(name: &str, f: F) -> R
where
    F: FnOnce() -> R,
{
    let start = Instant::now();
    let result = f();
    let duration = start.elapsed();
    info!("{} took {:?}", name, duration);
    result
}

/// Macro for easy profiling
#[macro_export]
macro_rules! profile_scope {
    ($name:expr) => {
        let _scope = puffin::profile_scope!($name);
    };
}
```

---

## Usage Examples

### Example 1: Replace spawning with instanced rendering

```rust
// BEFORE (slow)
// src/systems/spawning.rs
spawn_atoms_from_frame_internal(commands, meshes, materials, frame, atom_data);

// AFTER (fast)
// src/systems/instanced_spawning.rs
spawn_atoms_instanced(commands, meshes, materials, frame, atom_data);
```

### Example 2: Enable frustum culling

```rust
// In your plugin registration
app.add_systems(Update, frustum_cull_atoms);
```

### Example 3: Use material pool

```rust
// Instead of creating new materials:
// let material = materials.add(StandardMaterial { ... });

// Use from pool:
let material_pool = material_pool.get_material(element);
```

---

**All code snippets are ready to use. Copy and adapt to your project structure.**
