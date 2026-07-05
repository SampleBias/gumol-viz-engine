# 01 — Critical Fixes & Cleanup

**Priority:** P0 — Do this first  
**Estimated effort:** 2–3 developer-days  
**Dependencies:** None  
**Blocks:** Everything else

---

## Why This Comes First

The app migrated to instanced rendering but left duplicate systems, stale references, and UI bugs. Fixing these prevents developers from building on broken assumptions and reduces merge conflicts during the instanced pipeline work in `02` and `03`.

---

## Current Problems

### 1.1 Dual rendering paths (per-atom vs instanced)

| Path | Location | Status |
|------|----------|--------|
| Instanced (active) | `rendering/instanced.rs`, registered in `systems/mod.rs` | Primary |
| Per-atom (legacy) | `systems/spawning.rs` | Still registered, unused in main loop |

**Problem:** `spawning.rs` still registers resources (`AtomEntities`) that bonds, export, and selection depend on — but instanced spawning never populates them.

### 1.2 UI loadable extensions mismatch

`ui/mod.rs` line 31:
```rust
const LOADABLE_EXTENSIONS: &[&str] = &["xyz", "pdb", "gro"];
```

Parsers for DCD and mmCIF exist and `loading.rs` handles them, but the UI file picker and drag-drop reject them.

### 1.3 PDB atom metadata is placeholder

`loading.rs::create_atom_data_from_pdb()` assigns every atom `Element::C` with generic names. CPK colors, bond detection, and element grouping are wrong for PDB files.

### 1.4 Deprecation warnings in main.rs

`Color::rgb` → `Color::srgb` (4 warnings). Quick fix, signals code health.

### 1.5 Clear selection UI bug

"Clear selection" removes `Selected` component but may not call `SelectionState::clear()`, causing highlight to reappear (documented in BUILD_OUT_ROADMAP).

---

## Tasks

### Task 1.1 — Audit and document the active pipeline
**Effort:** 2 hours  
**Owner:** Tech lead / senior dev

- [ ] Trace `LoadFileEvent` → spawn → render → timeline update end-to-end
- [ ] Confirm instanced path is the only atom spawn path in production
- [ ] List every reference to `SpawnedAtom`, `AtomEntities`, `spawn_atoms_on_load`
- [ ] Add findings as comments in `systems/mod.rs` (temporary, remove after 03)

**Acceptance:** Written list of all per-atom references with file:line.

---

### Task 1.2 — Fix UI loadable extensions
**Effort:** 1 hour  
**Files:** `src/ui/mod.rs`

- [ ] Update `LOADABLE_EXTENSIONS` to include `dcd`, `cif`, `mmcif`, `mcif`
- [ ] Update drag-drop and file picker warning messages
- [ ] Add note in UI status bar when DCD loaded without topology file

**Acceptance:** User can open `.cif` and `.dcd` files from dialog and drag-drop.

---

### Task 1.3 — Fix PDB atom metadata
**Effort:** 4 hours  
**Files:** `src/io/pdb.rs`, `src/systems/loading.rs`

- [ ] Have `PDBParser` return `Vec<AtomData>` alongside `Trajectory` (or store in trajectory metadata)
- [ ] Parse element from atom name (first character heuristic + lookup table)
- [ ] Preserve residue name, chain ID, B-factor, occupancy
- [ ] Remove `create_atom_data_from_pdb()` placeholder
- [ ] Add unit test with a real small PDB (e.g. water or 1CRN subset)

**Acceptance:** Loading 1CRN.pdb shows correct element colors (O red, N blue, C gray, etc.).

---

### Task 1.4 — Fix clear selection bug
**Effort:** 1 hour  
**Files:** `src/ui/mod.rs`, `src/interaction/selection.rs`

- [ ] Find "Clear selection" button handler
- [ ] Call `selection.clear()` AND remove `Selected` components
- [ ] Send `SelectionClearedEvent`
- [ ] Add test or manual test checklist item

**Acceptance:** Clear selection stays cleared; no highlight flicker.

---

### Task 1.5 — Fix deprecation warnings
**Effort:** 30 min  
**Files:** `src/main.rs`

- [ ] Replace `Color::rgb(...)` with `Color::srgb(...)`
- [ ] Run `cargo clippy -- -D warnings` — zero warnings in main binary

---

### Task 1.6 — Mark legacy spawning path
**Effort:** 2 hours  
**Files:** `src/systems/spawning.rs`, `src/systems/mod.rs`

- [ ] Add `#[deprecated(note = "Use instanced rendering path")]` on per-atom spawn functions
- [ ] Stop registering unused systems (keep `AtomEntities` resource until 03 migrates bonds)
- [ ] Add `// MIGRATION:` comments at every `SpawnedAtom` usage site

**Do NOT delete spawning.rs yet** — bonds and export still need it until `03` and `09`.

---

### Task 1.7 — Center camera only on file load
**Effort:** 1 hour  
**Files:** `src/rendering/instanced.rs`

- [ ] Verify `center_camera_on_file_load_instanced` only runs on `FileLoadedEvent`
- [ ] Remove any per-frame camera snap (check `PostUpdate` systems)
- [ ] Test: orbit camera after load — camera must not snap back

**Acceptance:** Camera stays where user puts it after initial auto-center on load.

---

## Efficiency Notes

- **Do not rewrite bonds or selection in this phase** — that's `03` and `06`. Only fix what's broken independent of architecture.
- **Batch the PDB fix with a test** — one PR, one review.
- **Run `cargo test && cargo clippy` before every PR** in this phase to keep main green.

---

## Definition of Done

- [ ] All tasks 1.1–1.7 complete
- [ ] `cargo build`, `cargo test`, `cargo clippy -- -D warnings` pass
- [ ] Manual smoke test: load XYZ, PDB, GRO, mmCIF from UI; orbit camera; clear selection
- [ ] PR merged to `master` before any developer starts `02`
