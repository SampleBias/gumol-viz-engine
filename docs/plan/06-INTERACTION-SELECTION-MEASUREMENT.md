# 06 — Interaction: Selection & Measurement

**Priority:** P1  
**Estimated effort:** 4–5 developer-days  
**Dependencies:** 02 (`InstancedAtomIndex`)  
**Can parallelize with:** 04, 05

---

## Goal

Click-to-select individual atoms in the instanced pipeline; show distance, angle, and dihedral measurements.

---

## Current State

**Code exists but broken with instanced rendering:**
- `SelectionState` resource — ✅
- `handle_atom_selection` via `bevy_mod_picking` — targets per-atom entities (empty)
- `MeasurementState` + `compute_measurements` — ✅ logic correct, wrong data source
- Shift-click toggle, Escape to clear — ✅
- Selection highlighting via `Selected` component — needs instanced highlight

---

## Architecture: Picking Instanced Atoms

Per-atom entities don't exist. Options:

### Recommended: Invisible pick spheres (v1, simplest)
During spawn, create lightweight invisible per-atom entities with `PickableBundle` only (no mesh). Instanced entities handle rendering; pick entities handle interaction.

**Pros:** Works with existing `bevy_mod_picking` API.  
**Cons:** N entities for picking (memory overhead, but no draw cost if no mesh).

### Alternative: Custom ray-sphere intersection (v2)
Raycast against instance buffer on CPU or GPU. More efficient at 100K atoms but more work.

**Start with pick spheres for ≤50K atoms; optimize in `10` if needed.**

---

## Tasks

### Task 6.1 — Pick proxy entities
**Effort:** 1 day  
**Files:** new `src/interaction/pick_proxy.rs`, `rendering/instanced.rs`

- [ ] On instanced spawn, also spawn pick proxy per atom:
  ```rust
  commands.spawn((
      PickableBundle::default(),
      PickProxy { atom_id: u32 },
      Transform::from_translation(pos),
      Visibility::Hidden,  // no mesh, picking only
  ));
  ```
- [ ] Track in `PickProxyEntities` resource (atom_id → entity)
- [ ] Update pick proxy transforms in `update_instanced_positions_from_timeline`
- [ ] Despawn on file reload

---

### Task 6.2 — Rewire selection to pick proxies
**Effort:** 4 hours  
**Files:** `interaction/selection.rs`

- [ ] `handle_atom_selection` queries `PickProxy` instead of `SpawnedAtom`
- [ ] `SelectionState` stores pick proxy entities (or atom IDs — prefer atom IDs):
  ```rust
  pub selected_atom_ids: Vec<u32>,
  ```
- [ ] Emit `AtomSelectedEvent { atom_id }`

---

### Task 6.3 — Selection highlighting on instanced atoms
**Effort:** 1 day  
**Files:** `interaction/selection.rs`, `rendering/instanced.rs`

- [ ] On selection change, update `AtomInstanceData.color` or `emissive` for selected atoms
- [ ] Use bright yellow/orange highlight; restore CPK color on deselect
- [ ] Batch update: mark dirty instances, refresh GPU buffer

---

### Task 6.4 — Fix measurement system
**Effort:** 4 hours  
**Files:** `interaction/measurement.rs`

- [ ] Read positions from `InstancedAtomIndex` + `InstancedAtomMesh` (not `SpawnedAtom`)
- [ ] Use `selected_atom_ids` in order (selection order matters for dihedrals)
- [ ] Verify:
  - 2 atoms → distance (Å)
  - 3 atoms → angle at middle atom (°)
  - 4 atoms → dihedral (°)

---

### Task 6.5 — Measurement UI display
**Effort:** 2 hours  
**Files:** `ui/mod.rs`

- [ ] Show measurements in status/inspector panel:
  ```
  Distance: 1.42 Å
  Angle: 109.5°
  Dihedral: -120.3°
  ```
- [ ] Copy-to-clipboard button for values

---

### Task 6.6 — Multi-selection modes
**Effort:** 4 hours  
**Files:** `interaction/selection.rs`

- [ ] Click: replace selection
- [ ] Shift+click: toggle atom in selection
- [ ] Ctrl+A: select all (optional, warn on >10K)
- [ ] Box selection (future — `SelectionMode::Box`)

---

## Testing

- [ ] Click O atom in water — selected, highlighted yellow
- [ ] Shift+click H — both selected, distance ~0.96 Å shown
- [ ] Select 3 atoms — angle displayed
- [ ] Clear selection — highlight removed, measurements cleared
- [ ] During trajectory playback — pick proxies follow atoms

---

## Efficiency Notes

- Pick proxies add N entities but **zero draw calls** (no mesh). Acceptable for v1.
- Store `atom_id` in selection state, not entity IDs — survives reload logic better.
- Don't implement box selection until pick spheres work.

---

## Definition of Done

- [ ] Click selection works on instanced-rendered molecules
- [ ] Measurements accurate to 0.01 Å / 0.1°
- [ ] Highlight visible in all render modes
- [ ] No dependency on `SpawnedAtom` in interaction module
