# 02 — Instanced Rendering Pipeline

**Priority:** P0 — Critical path  
**Estimated effort:** 1 developer-week (2 devs can pair/split tasks)  
**Dependencies:** 01  
**Blocks:** 03, 05, 06, 09, 10

---

## Goal

Render N atoms in ~118 draw calls (one per element) using a custom Bevy 0.14 render pipeline with GPU instance buffers. Target: **100,000 atoms @ 60 FPS**.

---

## Current State (~60% Complete)

**Done:**
- `AtomInstanceData` struct (position, scale, color) — GPU-ready
- `InstancedAtomMesh` component with `ExtractComponent`
- Custom WGSL shader and render command in `instanced.rs`
- `spawn_atoms_instanced_internal()` groups atoms by element
- `update_instanced_positions_from_timeline()` updates instance buffers
- Registered in `GumolVizPlugin` via `rendering::register`

**Not done (from OPTIMIZATION_PROGRESS.md):**
- Benchmark verification (draw call count, FPS)
- Material pool (118 pre-created materials)
- Frustum culling integration
- Picking data for instance index → atom ID mapping
- Tests with 1CRN.pdb and 10K+ atom systems

---

## Architecture

```
SimulationData (CPU)
    │
    ▼
spawn_atoms_instanced_internal()
    │ groups by Element
    ▼
InstancedAtomMesh { instances: Vec<AtomInstanceData> }
    │ ExtractComponent (each frame)
    ▼
Render World → GPU vertex buffer (Instance step mode)
    │
    ▼
Custom draw command → 1 draw call per element entity
```

### Key types (existing)

| Type | File | Purpose |
|------|------|---------|
| `AtomInstanceData` | `instanced.rs:40` | 32-byte GPU instance record |
| `InstancedAtomMesh` | `instanced.rs:53` | Component holding all instances for one element |
| `InstancedAtomEntity` | `instanced.rs:71` | Marker: which element this entity represents |
| `InstancedAtomEntities` | `instanced.rs:77` | Resource: element → entity map |

---

## Tasks

### Task 2.1 — Verify and benchmark instanced rendering
**Effort:** 4 hours  
**Files:** `benches/`, `examples/basic_load.rs`

- [ ] Add `criterion` benchmark: spawn 1K, 10K, 100K atoms
- [ ] Use `bevy` diagnostic or `tracy` to count draw calls
- [ ] Document baseline in `docs/OPTIMIZATION_PROGRESS.md`
- [ ] Target: ≤118 draw calls regardless of atom count

**Acceptance:** Benchmark table filled; draw calls ≤118 for 10K atoms.

---

### Task 2.2 — Material pool
**Effort:** 4 hours  
**Files:** new `src/rendering/materials.rs`, `rendering/mod.rs`, `instanced.rs`

- [ ] Create `MaterialPool` resource
- [ ] Pre-create `StandardMaterial` (or custom) for each of 118 elements using `Element::cpk_color()`
- [ ] Replace per-spawn material allocation with pool lookup
- [ ] Unit test: pool has 118 entries, colors match CPK

**Efficiency:** Eliminates runtime material allocation on every file load (currently expensive for large systems).

---

### Task 2.3 — Atom ID → instance index mapping
**Effort:** 1 day  
**Files:** `instanced.rs`, new `src/rendering/atom_index.rs`

This is **required for selection (06) and bond detection (03)**.

- [ ] Add `InstancedAtomIndex` resource:
  ```rust
  pub struct InstancedAtomIndex {
      /// atom_id → (element, index_in_element_vec)
      pub map: HashMap<u32, (Element, u32)>,
      /// Reverse: for picking
      pub element_instances: HashMap<Element, Vec<u32>>, // instance_idx → atom_id
  }
  ```
- [ ] Build map during `spawn_atoms_instanced_internal()`
- [ ] Rebuild on file reload
- [ ] Expose `fn atom_id_at(element: Element, instance_idx: u32) -> Option<u32>`

**Acceptance:** Given any atom_id, O(1) lookup to update its instance data.

---

### Task 2.4 — Efficient instance buffer updates
**Effort:** 1 day  
**Files:** `instanced.rs`

Current approach clones entire `instances` vec each frame on timeline update.

- [ ] Track dirty atoms (changed positions this frame)
- [ ] For full frame step: bulk-update positions in `InstancedAtomMesh.instances`
- [ ] Avoid full clone in `ExtractComponent` when nothing changed (add `Dirty` flag)
- [ ] Consider `bytemuck` cast slice upload to GPU buffer directly

**Efficiency:** For 100K atoms @ 30 FPS trajectory, this avoids 100K × 32 bytes × 60 copies/sec on CPU.

---

### Task 2.5 — Visualization mode integration
**Effort:** 1 day  
**Files:** `instanced.rs`, `systems/visualization.rs`, `core/visualization.rs`

- [ ] On `RenderMode` change, update `AtomInstanceData.scale` per mode:
  - CPK: 100% VDW radius
  - Ball-and-stick: 50% VDW
  - Licorice: 25% VDW
  - Points: 10% VDW
- [ ] Hide instances (scale=0 or alpha=0) when `VisualizationConfig` toggles atoms off
- [ ] Wire existing `update_atom_scale` / `update_atom_visibility` to instanced path (or replace)

---

### Task 2.6 — Shader and render pipeline hardening
**Effort:** 1 day  
**Files:** `instanced.rs`, `assets/shaders/` (if extracted)

- [ ] Extract WGSL to `assets/shaders/instanced_atom.wgsl` for easier editing
- [ ] Support per-instance opacity (for selection highlight)
- [ ] Verify alpha blending order (transparent atoms)
- [ ] Test on Intel integrated GPU (minimum spec target)

---

### Task 2.7 — Remove per-atom spawn from hot path (soft deprecation)
**Effort:** 2 hours  
**Files:** `systems/mod.rs`, `systems/spawning.rs`

- [ ] Confirm no system in the Update chain calls `spawn_atoms_on_load`
- [ ] Gate legacy spawn behind `#[cfg(feature = "legacy_per_atom")]` if needed for debugging
- [ ] Update `AGENTS.md` to state instanced is the only production path

---

## Testing Checklist

- [ ] Load water.xyz (3 atoms) — 2–3 element entities spawned
- [ ] Load 1CRN.pdb (327 atoms) — correct colors, smooth orbit
- [ ] Play multi-frame XYZ trajectory — atoms animate smoothly
- [ ] Switch CPK ↔ ball-and-stick — scales update without reload
- [ ] Reload different file — old instanced entities despawned, new ones spawned
- [ ] Draw call count ≤118 at all atom counts

---

## Efficiency Notes

- **Do not implement picking here** — build the index (2.3) only; picking is `06`.
- **Do not implement bonds here** — bonds need atom positions from `InstancedAtomIndex` (`03`).
- **Pair 2.2 + 2.3** — same developer, same PR, avoids merge conflicts in `instanced.rs`.
- Use `--features dev_dynamic` for faster iteration during shader work.

---

## Definition of Done

- [ ] Tasks 2.1–2.7 complete
- [ ] OPTIMIZATION_PROGRESS Phase 1 marked 100%, Phase 2 (material pool) complete
- [ ] Benchmark shows ≥10× draw call reduction vs per-atom at 10K atoms
- [ ] `InstancedAtomIndex` resource available for downstream modules
