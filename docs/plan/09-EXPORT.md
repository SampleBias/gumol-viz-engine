# 09 — Export

**Priority:** P2  
**Estimated effort:** 1 developer-week  
**Dependencies:** 03 (bonds), 02 (instanced positions)  
**Can parallelize with:** 07, 08

---

## Goal

Working export of screenshots, 3D models, and (optionally) video from the instanced rendering pipeline.

---

## Current State

| Export | File | Status |
|--------|------|--------|
| Screenshot PNG | `export/screenshot.rs` | ✅ Implemented via Bevy `ScreenshotManager` |
| OBJ | `export/obj.rs` | 🟡 Implemented, reads `SpawnedAtom` (broken) |
| glTF | `export/gltf_export.rs` | 🟡 Implemented, reads `SpawnedAtom` (broken) |
| Video MP4/WebM | — | ❌ Not started |
| POV-Ray | — | ❌ Not started |

UI has export buttons with async save dialogs — wiring exists.

---

## Tasks

### Task 9.1 — Fix OBJ export for instanced pipeline
**Effort:** 4 hours  
**Files:** `export/obj.rs`, `export/mesh_export.rs`

- [ ] Collect atom positions from `InstancedAtomMesh` + `InstancedAtomIndex`
- [ ] Collect bond positions from bond entities (03)
- [ ] Remove dependency on `SpawnedAtom` / `AtomEntities`
- [ ] Test: export water.xyz → import in Blender

---

### Task 9.2 — Fix glTF export for instanced pipeline
**Effort:** 4 hours  
**Files:** `export/gltf_export.rs`

- [ ] Same data source as 9.1
- [ ] Export as glTF 2.0 binary (.glb)
- [ ] Include element colors as vertex colors
- [ ] Test: view in https://gltf-viewer.donmccurdy.com/

---

### Task 9.3 — Screenshot enhancements
**Effort:** 4 hours  
**Files:** `export/screenshot.rs`, `ui/mod.rs`

- [ ] PNG (default) and JPEG options
- [ ] Resolution override (1×, 2×, 4× viewport) — render to offscreen target
- [ ] Include filename timestamp: `gumol_2026-07-05_143022.png`
- [ ] Success/error toast in UI

---

### Task 9.4 — Video export (FFmpeg)
**Effort:** 2 days  
**Files:** new `src/export/video.rs`, `Cargo.toml`

- [ ] Add optional feature flag `video` in Cargo.toml
- [ ] Depends on `ffmpeg-next` or pipe raw frames to `ffmpeg` subprocess
- [ ] `VideoExportSettings`: fps, format (MP4/WebM/GIF), quality, frame range
- [ ] UI: "Record" button → plays timeline → captures frames → encodes
- [ ] Progress bar during encode

```toml
[features]
video = ["ffmpeg-next"]  # optional
```

**Efficiency:** Subprocess to `ffmpeg` CLI is simpler than binding; document `sudo apt install ffmpeg`.

---

### Task 9.5 — POV-Ray export
**Effort:** 1 day  
**Files:** new `src/export/povray.rs`

- [ ] Generate `.pov` scene file with spheres at atom positions
- [ ] CPK colors as `pigment { rgb }`
- [ ] Bonds as cylinders
- [ ] Camera block from current viewport
- [ ] UI button + save dialog

---

### Task 9.6 — Export current frame only vs trajectory
**Effort:** 4 hours  
**Files:** `export/mod.rs`, UI

- [ ] Checkbox: "Export current frame" (default) vs "Export all frames"
- [ ] All frames: create numbered sequence or multi-frame glTF animation
- [ ] Warn on large exports (>1000 frames)

---

## Export Data Collection Pattern

Centralize to avoid duplicating instanced lookup in every exporter:

```rust
// export/scene_snapshot.rs
pub struct SceneSnapshot {
    pub atoms: Vec<AtomSnapshot>,  // id, pos, radius, color
    pub bonds: Vec<BondSnapshot>,  // pos_a, pos_b, radius
    pub camera: CameraSnapshot,
}

pub fn capture_scene(world: &World) -> SceneSnapshot { ... }
```

All exporters call `capture_scene()` — **build this first**, then 9.1 and 9.2 become trivial.

---

## Definition of Done

- [ ] Screenshot saves PNG from UI button
- [ ] OBJ and glTF export correctly from instanced scene
- [ ] `SceneSnapshot` helper used by all exporters
- [ ] Video export records 10-frame trajectory (with `video` feature)
- [ ] No references to `SpawnedAtom` in export module
