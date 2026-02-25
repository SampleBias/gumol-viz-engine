# Gumol Viz Engine — Build-Out Roadmap

**Generated from codebase review.** This document outlines what exists, what’s missing, and what to build next to complete the engine.

---

## Current State Summary

### Implemented and working

| Module | Status | Notes |
|--------|--------|-------|
| **Core** | Done | Atom, Bond, Molecule, Trajectory, FrameData, TimelineState, RenderMode, ColorScheme |
| **IO** | Partial | XYZ and PDB parsers; format detection; `is_loadable()` |
| **Loading** | Done | LoadFileEvent pipeline, CLI arg, SimulationData, FileHandle |
| **Spawning** | Done | Atom entities, CPK colors, picking, position updates |
| **Bonds** | Done | Distance-based detection, cylinder meshes, CONECT-ready |
| **Timeline** | Done | Playback, interpolation, keyboard controls |
| **Visualization** | Done | RenderMode switching, atom/bond scale, visibility |
| **Camera** | Done | PanOrbitCamera plugin |
| **Selection** | Done | Click selection, Shift-toggle, highlighting, Escape to clear |
| **UI** | Done | File intake (CLI, drag-drop, Open), status, timeline, viz modes, bonds |
| **Rendering** | Done | Atom spheres, bond cylinders, mesh generation |

### Stubbed or missing

| Module | Status | Notes |
|--------|--------|-------|
| **Export** | Stub | `register()` only; no screenshot, video, or 3D export |
| **Secondary parsers** | Missing | GRO, DCD, mmCIF not implemented |
| **Examples** | Partial | `timeline_demo`, `interactive_selection` referenced but not present |
| **Measurement** | Missing | Distance, angle, dihedral tools |
| **Advanced viz** | Partial | Surface, Cartoon, Tube, Trace, Wireframe defined but not rendered |

---

## Priority 1: Critical gaps

### 1.1 Fix duplicate atom position updates

**Issue:** Two systems update atom positions:

- `spawning::update_atom_positions` (simple, no interpolation)
- `timeline::update_atom_positions_from_timeline` (with interpolation)

Both run in `Update` and can conflict.

**Action:** Remove `update_atom_positions` from spawning and rely on `update_atom_positions_from_timeline`. Ensure spawning only spawns atoms; timeline drives positions.

---

### 1.2 Fix “Clear selection” in UI

**Issue:** In `main_ui_panel`, the “Clear selection” button removes the `Selected` component but does not clear `SelectionState`. `update_selection_highlight` will re-add `Selected` because `selection.is_selected(entity)` stays true.

**Action:** When “Clear selection” is clicked, also call `selection.clear()` (or equivalent) so `SelectionState` matches the cleared selection.

---

### 1.3 Center camera only on load

**Issue:** `center_camera_on_molecule` runs every frame in `PostUpdate`, constantly snapping the camera to the molecule center.

**Action:** Run it only when a file is loaded (e.g. on `FileLoadedEvent`), not every frame.

---

## Priority 2: Export (high value)

### 2.1 Screenshot export

- Capture current viewport to PNG/JPEG.
- Use Bevy’s render target or `bevy_screenshot`-style approach.
- Add “Screenshot” button in UI.

### 2.2 3D export (OBJ, glTF)

- Export atom positions and bonds as meshes.
- OBJ: simple vertex/face format.
- glTF: better for web and modern tools.

### 2.3 Video export (optional)

- Requires FFmpeg.
- Record frames during playback.
- Gate behind `video` feature flag.

---

## Priority 3: Secondary file formats

### 3.1 GRO (GROMACS)

- Text format.
- Parse coordinates and box.
- Add `GROParser` in `io/gro.rs` and wire into loading.

### 3.2 DCD (CHARMM)

- Binary format.
- Use `byteorder` for endianness.
- Add `DCDParser` in `io/dcd.rs`.

### 3.3 mmCIF

- Use `quick-xml` for parsing.
- Add `MMCIFParser` in `io/mmcif.rs`.

---

## Priority 4: Measurement tools

### 4.1 Distance measurement

- When 2 atoms selected, show distance in Å.
- Use `SelectionState` and atom positions.
- Add UI display (e.g. in inspector or status).

### 4.2 Angle measurement

- When 3 atoms selected, show angle in degrees.
- Compute via dot product of vectors.

### 4.3 Dihedral angle

- When 4 atoms selected, show dihedral in degrees.
- Compute via cross products and atan2.

---

## Priority 5: UI and UX polish

### 5.1 Inspector panel

- Show details for selected atom(s): element, residue, chain, coordinates, B-factor.
- Reuse `SelectionState` and atom components.

### 5.2 Atom/residue labels

- Optional text labels for selected atoms or residues.
- Use Bevy text or egui overlay.

### 5.3 Missing examples

- Add `timeline_demo` and `interactive_selection` examples.
- Update README to match actual examples.

---

## Priority 6: Advanced visualization

### 6.1 Wireframe mode

- Replace spheres with line segments between bonded atoms.
- Use `RenderMode::Wireframe` and line rendering.

### 6.2 Licorice mode

- Smaller spheres, thicker bonds.
- Mostly scaling; verify with `VisualizationConfig`.

### 6.3 Surface mode (harder)

- Solvent-accessible surface (e.g. rolling ball).
- Or use a library (e.g. `nano-vdb` or similar).

### 6.4 Cartoon / ribbon (harder)

- Backbone ribbons with secondary structure.
- Requires secondary structure assignment (DSSP or heuristic).

---

## Priority 7: Performance

### 7.1 Instanced rendering

- Single draw call for many identical atoms.
- Use Bevy’s instancing or custom instanced mesh.

### 7.2 Despawn old atoms/bonds on reload

- When loading a new file, despawn previous atoms and bonds before spawning new ones.
- Ensure `spawn_atoms_on_load` and bond systems handle reload correctly.

### 7.3 Memory-mapped trajectory loading

- For large XYZ/PDB, use `memmap2` to avoid full load.
- Add streaming/partial-load API.

---

## Suggested build order

1. **Week 1 — Fixes**
   - Remove duplicate `update_atom_positions`.
   - Fix “Clear selection” and center-camera behavior.

2. **Week 2 — Export**
   - Screenshot (PNG).
   - OBJ export.

3. **Week 3 — Measurement**
   - Distance for 2 atoms.
   - Angle for 3 atoms.

4. **Week 4 — Formats**
   - GRO parser.
   - DCD parser (if needed).

5. **Week 5 — Polish**
   - Inspector panel.
   - Add missing examples.
   - Update README.

6. **Week 6+ — Advanced**
   - Wireframe rendering.
   - Surface mode (if feasible).
   - Instanced rendering.

---

## Architecture notes

- All file loading goes through `LoadFileEvent` → `handle_load_file_events`.
- New parsers: implement in `io/`, add to `load_file()` in `loading.rs`, extend `FileFormat`.
- New UI: add systems in `ui/mod.rs` or new `ui/*.rs` modules.
- Export: add systems in `export/` and wire to UI.

---

## References

- [DEVELOPMENT_PLAN.md](DEVELOPMENT_PLAN.md) — Full spec and phases
- [ARCHITECTURE.md](ARCHITECTURE.md) — System layout and data flow
- [README.md](../README.md) — Roadmap and feature list
