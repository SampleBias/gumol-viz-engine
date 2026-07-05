# 08 — Visualization Modes

**Priority:** P2  
**Estimated effort:** 1 developer-week  
**Dependencies:** 03 (bonds), 02 (instanced scale)  
**Can parallelize with:** 07, 09

---

## Goal

Implement all advertised render modes with correct atom/bond scaling and visibility.

---

## Current State

`RenderMode` enum in `core/visualization.rs` defines 10 modes. **Only CPK and ball-and-stick partially work** via scale toggling. Others are enum-only.

| Mode | Status | Notes |
|------|--------|-------|
| CPK | 🟡 Partial | Atoms at 50% VDW — should be 100% for true CPK |
| Ball-and-stick | 🟡 Partial | Needs bonds (03) |
| Licorice | 🟡 Partial | Scale logic exists, needs bond thickening |
| Wireframe | ❌ | No line rendering |
| Surface | ❌ | No surface algorithm |
| Cartoon | ❌ | No ribbon mesh |
| Tube | ❌ | No backbone tube |
| Trace | ❌ | No backbone trace |
| Points | ❌ | Not implemented |

---

## Tasks

### Task 8.1 — Render mode switching (fix existing)
**Effort:** 4 hours  
**Files:** `systems/visualization.rs`, `core/visualization.rs`

- [ ] Centralize mode → (atom_scale, bond_scale, show_atoms, show_bonds) mapping
- [ ] Apply on mode change and on file load
- [ ] Unit test the mapping table

```rust
fn mode_params(mode: RenderMode) -> ModeParams {
    match mode {
        RenderMode::CPK => ModeParams { atom: 1.0, bond: 0.0, atoms: true, bonds: false },
        RenderMode::BallAndStick => ModeParams { atom: 0.3, bond: 0.15, atoms: true, bonds: true },
        RenderMode::Licorice => ModeParams { atom: 0.15, bond: 0.25, atoms: true, bonds: true },
        // ...
    }
}
```

---

### Task 8.2 — Points mode
**Effort:** 4 hours  
**Files:** `rendering/instanced.rs`

- [ ] Use low-poly icosphere (4 latitudes) or `PointList` topology
- [ ] Very small scale (0.05 × VDW)
- [ ] Highest performance mode for 100K+ atoms

---

### Task 8.3 — Wireframe mode
**Effort:** 1 day  
**Files:** new `src/rendering/wireframe.rs`

- [ ] Hide atom instances (scale = 0)
- [ ] Render bonds as thin lines (use `LineList` mesh or `bevy_prototype_line` equivalent)
- [ ] For atoms without bonds: draw nothing (or optional point)

**Bevy 0.14 note:** Check `Gizmos` or custom line mesh — no built-in line primitive in PBR.

---

### Task 8.4 — Licorice mode polish
**Effort:** 2 hours  
**Files:** `systems/visualization.rs`, `systems/bonds.rs`

- [ ] Atom scale: 0.1 × VDW
- [ ] Bond radius: 0.2 Å (thick sticks)
- [ ] Uniform bond color (gray) vs element-colored

---

### Task 8.5 — Ribbon / Cartoon mode (proteins only)
**Effort:** 3 days  
**Files:** new `src/rendering/ribbon.rs`, new `src/core/secondary_structure.rs`

- [ ] Extract backbone atoms (CA or P atoms for proteins/nucleic acids)
- [ ] Assign secondary structure:
  - v1: heuristic (distance-based helix/sheet/loop)
  - v2: DSSP integration (external crate or FFI)
- [ ] Generate smooth spline through backbone CA positions
- [ ] Extrude ribbon mesh (flat strip for sheet, tube for helix)
- [ ] Color by secondary structure (helix=red, sheet=yellow, loop=green)

**Efficiency:** Only activate for structures with >20 amino acids; hide option otherwise.

---

### Task 8.6 — Surface mode (defer heavy work to 11)
**Effort:** 4 hours (stub only)  
**Files:** `ui/mod.rs`

- [ ] Show "Surface mode — coming soon" in UI
- [ ] Disable or gray out button until 11 is done
- [ ] Don't leave broken surface toggle

---

## Mode Priority for v1 Release

1. ✅ CPK (fix scale)
2. ✅ Ball-and-stick
3. ✅ Licorice
4. Points (performance)
5. Wireframe
6. Cartoon (if protein detected)
7. Surface → v1.1 (see 11)

---

## Definition of Done

- [ ] Mode selector switches all v1 modes without reload
- [ ] Bonds correctly shown/hidden per mode
- [ ] Points mode achieves 60 FPS at 100K atoms
- [ ] Cartoon renders for 1CRN.pdb
