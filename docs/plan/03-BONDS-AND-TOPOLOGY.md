# 03 — Bonds & Topology

**Priority:** P0  
**Estimated effort:** 4–5 developer-days  
**Dependencies:** 02 (needs `InstancedAtomIndex`)  
**Blocks:** 08 (visualization modes), 09 (export)

---

## Goal

Detect and render bonds using atom positions from the instanced pipeline — without per-atom entities.

---

## Current State

**Working (legacy path only):**
- Distance-based bond detection in `systems/bonds.rs`
- VdW radius multiplier, min/max distance thresholds
- Cylinder mesh generation via `rendering::generate_bond_mesh`
- Bond position sync from atom `Transform`

**Broken:**
- `spawn_bonds` queries `AtomEntities` + `SpawnedAtom` — empty with instanced rendering
- `update_bond_positions` reads per-atom transforms that don't exist
- No CONECT record parsing from PDB (topology from file ignored)

---

## Architecture Decision

### Option A — Instanced bonds (recommended for performance)
Group bonds by length bucket or material; one draw call per bond-type batch. Harder to implement.

### Option B — Per-bond entities (recommended for v1)
Keep one entity per bond (typical count: 0.5–1.5× atom count). Acceptable draw call count for ≤50K bonds. **Start here.**

### Data flow (Option B)

```
InstancedAtomIndex + SimulationData
        │
        ▼
detect_bonds()  ── reads positions from InstancedAtomMesh.instances
        │
        ▼
Vec<BondData>  (atom_a_id, atom_b_id, order, length)
        │
        ▼
spawn_bond_entities()  ── one PbrBundle cylinder per bond
        │
        ▼
update_bond_positions()  ── reads from InstancedAtomMesh, not Transform
```

---

## Tasks

### Task 3.1 — Position lookup from instanced data
**Effort:** 4 hours  
**Files:** `systems/bonds.rs`, `rendering/instanced.rs`

- [ ] Add helper:
  ```rust
  fn get_atom_position(
      atom_id: u32,
      index: &InstancedAtomIndex,
      instanced: &Query<&InstancedAtomMesh, With<InstancedAtomEntity>>,
  ) -> Option<Vec3>
  ```
- [ ] Replace all `AtomEntities` / `SpawnedAtom` position reads with this helper

---

### Task 3.2 — Rewrite bond detection input
**Effort:** 1 day  
**Files:** `systems/bonds.rs`

- [ ] `detect_bonds()` takes `SimulationData` + instanced positions
- [ ] O(N²) naive detection OK for ≤5K atoms; flag for spatial index (see `10`)
- [ ] For >5K atoms: use `BondDetectionConfig` to skip or use spatial index
- [ ] Parse PDB CONECT records when available (`io/pdb.rs`) — use as authoritative topology, fall back to distance detection

**Acceptance:** 1CRN.pdb shows correct backbone bonds; water shows 2 O–H bonds.

---

### Task 3.3 — Rewrite bond spawn and update
**Effort:** 1 day  
**Files:** `systems/bonds.rs`

- [ ] `spawn_bonds` triggered on `FileLoadedEvent` (after instanced atoms spawn)
- [ ] Store `BondData` (atom IDs, not entity refs) on bond component:
  ```rust
  pub struct Bond {
      pub atom_a_id: u32,
      pub atom_b_id: u32,
      // remove atom_a: Entity, atom_b: Entity
  }
  ```
- [ ] `update_bond_positions` reads positions via Task 3.1 helper each frame

---

### Task 3.4 — Bond visibility per render mode
**Effort:** 4 hours  
**Files:** `systems/visualization.rs`, `systems/bonds.rs`

| RenderMode | Atoms | Bonds |
|------------|-------|-------|
| CPK | visible | hidden |
| Ball-and-stick | visible (small) | visible |
| Licorice | visible (tiny) | visible (thick) |
| Wireframe | hidden | visible (as lines) |
| Surface | hidden | hidden |

- [ ] Wire `update_bond_visibility` and `update_bond_scale` to instanced path
- [ ] Licorice: bond radius ×2, atom scale ×0.25

---

### Task 3.5 — Bond detection performance (basic)
**Effort:** 1 day  
**Files:** `systems/bonds.rs`

- [ ] Skip bond detection when `BondDetectionConfig.enabled == false`
- [ ] Skip when `RenderMode::CPK` or `RenderMode::Surface`
- [ ] Run detection in `rayon` parallel chunk for >1K atoms
- [ ] Show progress in UI for >10K atom detection ("Detecting bonds…")

**Efficiency:** Don't run O(N²) on 100K atoms without spatial index — gate with warning.

---

### Task 3.6 — Clear bonds on reload
**Effort:** 2 hours  
**Files:** `systems/bonds.rs`

- [ ] Verify `clear_bonds_on_load` runs before new bond spawn
- [ ] Despawn all bond entities; clear `BondEntities` map
- [ ] Test: load file A → load file B → no orphan bonds

---

## Testing

- [ ] Water.xyz: 2 bonds, correct length ~1.0 Å
- [ ] 1CRN.pdb: peptide backbone bonds visible in ball-and-stick
- [ ] Trajectory playback: bonds follow atoms during animation
- [ ] CPK mode: bonds hidden
- [ ] Reload: bonds replaced, not duplicated

---

## Efficiency Notes

- **Don't build spatial index here** unless bond detection exceeds 2s on 10K atoms — that's `10`.
- **Keep bond entities** for v1; instanced bonds are a v2 optimization.
- **PDB CONECT first** — avoids false positives from distance detection on proteins.

---

## Definition of Done

- [ ] Bonds visible in ball-and-stick for XYZ, PDB, GRO files
- [ ] No references to `SpawnedAtom` in `bonds.rs`
- [ ] Bond count logged on spawn; no duplicates on reload
