# 07 — UI & UX

**Priority:** P1  
**Estimated effort:** 1 developer-week  
**Dependencies:** 06 (inspector needs selection), 04 (all formats in UI)  
**Can parallelize with:** 08, 09

---

## Goal

Polished, scientist-friendly interface: file management, inspector, settings, and feedback during long operations.

---

## Current State

**Done (`ui/mod.rs`):**
- File open dialog (native, async thread)
- Drag-and-drop loading
- CLI file arg support
- Main panel: file info, atom/frame count
- Timeline controls (play, slider, speed)
- Visualization mode selector
- Bond detection toggle
- Export buttons (screenshot, OBJ, glTF)
- Error display for load failures
- Measurement display (partial)

**Missing:**
- Atom/residue inspector panel
- B-factor coloring
- Background color / lighting settings
- Load progress indicator
- Keyboard shortcut reference
- Status bar for long operations

---

## UI Layout Target

```
┌─────────────────────────────────────────────────────────┐
│ Menu: File | View | Export | Help                        │
├──────────────┬──────────────────────────────────────────┤
│  Inspector   │                                          │
│  (selected   │           3D Viewport                    │
│   atoms)     │                                          │
│              │                                          │
│  Timeline    │                                          │
│  [====●====] │                                          │
│  ▶ ⏸  1.0x  │                                          │
├──────────────┴──────────────────────────────────────────┤
│ Status: 327 atoms | Frame 12/100 | Distance: 1.42 Å     │
└─────────────────────────────────────────────────────────┘
```

---

## Tasks

### Task 7.1 — Inspector panel
**Effort:** 1 day  
**Files:** new `src/ui/inspector.rs`, `ui/mod.rs`

When atom(s) selected, show:
- Element symbol and atom name
- Residue name and ID
- Chain ID
- Position (x, y, z) in Å
- B-factor, occupancy (if from PDB/mmCIF)
- Atom ID

- [ ] Create `inspector_ui` system
- [ ] Multi-select: show count + list (scrollable, max 20 visible)
- [ ] "Zoom to selection" button (move camera focus)

---

### Task 7.2 — View settings panel
**Effort:** 4 hours  
**Files:** new `src/ui/settings.rs`

- [ ] Background color picker (dark/light/custom)
- [ ] Atom scale slider (global multiplier)
- [ ] Bond thickness slider
- [ ] Ambient light intensity
- [ ] Persist settings to `VisualizationConfig` resource

---

### Task 7.3 — Load progress and feedback
**Effort:** 4 hours  
**Files:** `ui/mod.rs`, `systems/loading.rs`

- [ ] `LoadingState` resource: `Idle | Loading { path, progress } | Error`
- [ ] Progress bar during async load (04.8)
- [ ] Toast notification: "Loaded 1CRN.pdb — 327 atoms, 1 frame"
- [ ] Disable file open button while loading

---

### Task 7.4 — B-factor coloring
**Effort:** 1 day  
**Files:** `ui/settings.rs`, `rendering/instanced.rs`, `utils/colors.rs`

- [ ] Add `ColorScheme` option: CPK | ByBFactor | ByChain | ByResidue
- [ ] B-factor → blue (low) → white → red (high) gradient
- [ ] Update instance colors when scheme changes
- [ ] Only available when B-factor data present

---

### Task 7.5 — Keyboard shortcuts
**Effort:** 4 hours  
**Files:** `ui/mod.rs`, `camera/mod.rs`, `systems/timeline.rs`

| Key | Action |
|-----|--------|
| Space | Play/pause |
| ← / → | Previous/next frame |
| Home / End | First/last frame |
| Escape | Clear selection |
| Ctrl+O | Open file |
| Ctrl+S | Screenshot |
| F | Focus on molecule |
| 1–5 | Render mode shortcuts |

- [ ] Help overlay (? key) listing shortcuts

---

### Task 7.6 — Topology prompt for DCD
**Effort:** 4 hours  
**Files:** `ui/mod.rs`  
(Depends on 4.3)

- [ ] Modal dialog when DCD loaded without topology
- [ ] "Select topology file" button
- [ ] Remember last topology directory

---

### Task 7.7 — Split UI into modules
**Effort:** 4 hours  
**Files:** `ui/mod.rs` → `ui/file.rs`, `ui/timeline.rs`, `ui/inspector.rs`, `ui/settings.rs`, `ui/export_panel.rs`

- [ ] Refactor 500+ line `mod.rs` into focused modules
- [ ] Each module registers its own systems
- [ ] Easier parallel development

**Efficiency:** Do this early in the week so other UI tasks don't conflict.

---

## Definition of Done

- [ ] Inspector shows correct data for PDB-selected atom
- [ ] Settings persist during session
- [ ] Loading spinner on 10MB+ files
- [ ] Help overlay documents all shortcuts
- [ ] UI modules split, each <200 lines
