# 11 — Advanced Features

**Priority:** P3 (post-v1)  
**Estimated effort:** 3+ developer-weeks  
**Dependencies:** 08 (visualization foundation), 03 (bonds), 04 (formats)  
**Start after:** v1 release criteria met (see 00-INDEX)

---

## Goal

Differentiate Gumol from basic viewers with molecular surface rendering, protein cartoons, and analysis tools.

---

## Feature Overview

| Feature | Value | Complexity | Target Version |
|---------|-------|------------|----------------|
| Molecular surface (SES) | High | Very high | v1.1 |
| Cartoon/ribbon (DSSP) | High | High | v1.0 (basic) / v1.1 (DSSP) |
| Isosurface / volume | Medium | Very high | v1.2 |
| RMSD / RMSF analysis | High | Medium | v1.1 |
| Trajectory editing | Medium | Medium | v1.2 |
| Python bindings | Medium | High | v1.2 |
| Plugin system | Low | High | v2.0 |
| VR (OpenXR) | Low | High | v2.0 |

---

## 11A — Molecular Surface

### Task 11A.1 — Solvent-accessible surface (SES)
**Effort:** 2 weeks  
**Files:** new `src/rendering/surface.rs`, optional `src/rendering/surface/`

Algorithm options:
1. **Rolling ball** (standard) — implement or use `nano-shaper` via FFI
2. **GPU marching cubes** on density grid — faster for large systems
3. **External**: call MSMS or similar as subprocess (quick v1)

- [ ] Voxel grid at 0.5 Å resolution around molecule
- [ ] Expand atoms by probe radius (1.4 Å water)
- [ ] Marching cubes → triangle mesh
- [ ] Render as translucent PBR material
- [ ] Toggle in `RenderMode::Surface`

**Efficiency:** Cache surface mesh; only recompute on geometry change, not every frame.

### Task 11A.2 — Molecular surface (Connolly)
**Effort:** 1 week (after 11A.1)  
- [ ] Reentrant surface (smoother than SES)
- [ ] Optional — SES is sufficient for v1.1

---

## 11B — Protein Cartoon (Advanced)

### Task 11B.1 — DSSP secondary structure
**Effort:** 1 week  
**Files:** new `src/analysis/dssp.rs`

- [ ] Integrate `dssp` crate or port minimal DSSP algorithm
- [ ] Input: backbone N, CA, C coordinates
- [ ] Output: H (helix), E (sheet), C (coil) per residue
- [ ] Store in `SecondaryStructure` component on residue entities

### Task 11B.2 — Smooth ribbon mesh
**Effort:** 1 week  
**Files:** `rendering/ribbon.rs` (extend from 08)

- [ ] Catmull-Rom spline through CA atoms
- [ ] Flat ribbon for sheets, tube for helices, thin line for coils
- [ ] Color by chain or by secondary structure

---

## 11C — Analysis Tools

### Task 11C.1 — RMSD plot
**Effort:** 3 days  
**Files:** new `src/analysis/rmsd.rs`, `ui/analysis_panel.rs`

- [ ] Compute RMSD of all frames vs frame 0 (or reference)
- [ ] Display as egui line plot
- [ ] Click plot point → jump to frame

### Task 11C.2 — RMSF (flexibility)
**Effort:** 3 days  
- [ ] Per-atom fluctuation over trajectory
- [ ] Color atoms by RMSF (blue rigid → red flexible)
- [ ] Uses B-factor coloring infrastructure from 07

### Task 11C.3 — Radius of gyration
**Effort:** 1 day  
- [ ] Scalar value vs frame, plot in analysis panel

---

## 11D — Trajectory Tools

### Task 11D.1 — Frame range export
**Effort:** 2 days  
- [ ] Export frames 100–200 as sub-trajectory XYZ/GRO

### Task 11D.2 — Trajectory merge/splice
**Effort:** 3 days  
- [ ] Concatenate two trajectories with same topology

---

## 11E — Interoperability

### Task 11E.1 — Python bindings (PyO3)
**Effort:** 2 weeks  
**Files:** new `crates/gumol-python/`

```python
import gumol_viz
traj = gumol_viz.load("sim.xyz")
traj.export_gltf("out.glb")
gumol_viz.render_frame(traj, frame=0, output="frame.png")
```

- [ ] Expose load, export, and headless render API
- [ ] No GUI required for batch processing

### Task 11E.2 — Plugin system
**Effort:** 3 weeks  
- [ ] `GumolPlugin` trait for user-defined analysis
- [ ] Dynamic library loading (`libloading`)
- [ ] Example plugin: hydrogen bond finder

---

## 11F — VR Support

### Task 11F.1 — OpenXR integration
**Effort:** 2 weeks  
- [ ] Bevy OpenXR plugin (when available for 0.14+)
- [ ] Hand controllers for atom selection
- [ ] Scale molecule to room size

**Defer until desktop v1 is solid.**

---

## Prioritization Recommendation

```
v1.0 release → 00-INDEX success metrics
v1.1         → Surface (11A) + RMSD (11C.1) + RMSF (11C.2)
v1.2         → Python bindings (11E.1) + trajectory tools (11D)
v2.0         → Plugins (11E.2) + VR (11F)
```

---

## Definition of Done (v1.1 milestone)

- [ ] Surface mode renders translucent SES for 1CRN
- [ ] RMSD plot for 100-frame trajectory
- [ ] Cartoon with DSSP coloring for proteins
