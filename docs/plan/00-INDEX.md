# Gumol Viz Engine — Developer Build Plan

**Purpose:** Numbered, ordered plan for building and updating the application. Start at `01` and follow dependencies. Parallel work streams are defined in `12`.

**Last audited:** 2026-07-05 (branch `master`, commit `c11d48c`)

---

## How to Use This Plan

1. Read this index to understand current state and priorities.
2. Complete **01** before anything else — it fixes architectural blockers.
3. Work through **02–03** next — the instanced rendering pipeline is the critical path for performance and for bonds/selection/export to work correctly.
4. After **03**, developers can split across **04–11** in parallel (see `12`).
5. **12** runs continuously alongside all other work.

Each document contains:
- **Status** — what exists today
- **Why it matters** — architectural rationale
- **Tasks** — numbered, with estimates, file paths, and acceptance criteria
- **Efficiency notes** — how to avoid wasted effort

---

## Current State at a Glance

| Area | Status | Blocker? |
|------|--------|----------|
| Core data structures | ✅ Done | — |
| XYZ / PDB parsers | ✅ Working | — |
| GRO / mmCIF / DCD parsers | 🟡 Implemented, not fully wired in UI | Low |
| Instanced GPU rendering | 🟡 ~60% — custom pipeline exists, needs completion | **Yes** |
| Per-atom entity path | ⚠️ Deprecated but still referenced | **Yes** |
| Bond detection & rendering | 🟡 Works on per-atom path only | **Yes** |
| Timeline & interpolation | ✅ Working (instanced path) | — |
| Selection & measurement | 🟡 Code exists, broken with instanced atoms | **Yes** |
| UI (file load, timeline, viz) | ✅ Mostly done | — |
| Export (screenshot, OBJ, glTF) | 🟡 Implemented, broken with instanced atoms | Medium |
| Video / POV-Ray export | ❌ Not started | Low |
| Advanced viz (surface, cartoon) | ❌ Enum only, no rendering | Low |
| Performance (LOD, culling, async) | ❌ Not started | Medium |
| Tests & CI | 🟡 Minimal (1 integration test) | Medium |

---

## The Core Architectural Issue

The codebase is mid-migration from **one entity per atom** to **one entity per element with GPU instancing** (`src/rendering/instanced.rs`). The instanced path is registered in `systems/mod.rs`, but these modules still assume per-atom entities:

| Module | Depends on per-atom path |
|--------|--------------------------|
| `systems/bonds.rs` | `AtomEntities`, `SpawnedAtom` |
| `interaction/selection.rs` | `SpawnedAtom`, picking on atom entities |
| `interaction/measurement.rs` | `SpawnedAtom` + `Transform` |
| `export/obj.rs`, `export/gltf_export.rs` | `AtomEntities`, `SpawnedAtom` |
| `systems/visualization.rs` | Per-atom `Visibility` / scale |

**Until 01–03 are done, bonds, selection, measurements, and export will not work in the running app.**

---

## Numbered Plan (Read in Order)

| # | Document | Summary | Est. Effort | Can Parallelize After |
|---|----------|---------|-------------|----------------------|
| **01** | [Critical Fixes & Cleanup](./01-CRITICAL-FIXES.md) | Remove dead code, fix UI bugs, unify load paths | 2–3 days | — (start here) |
| **02** | [Instanced Rendering Pipeline](./02-INSTANCED-RENDERING-PIPELINE.md) | Complete GPU instancing, material pool, benchmarks | 1 week | 01 |
| **03** | [Bonds & Topology](./03-BONDS-AND-TOPOLOGY.md) | Bond detection + rendering on instanced data | 4–5 days | 02 |
| **04** | [File I/O & Formats](./04-FILE-IO-AND-FORMATS.md) | Wire DCD/mmCIF in UI, streaming, PDB atom metadata | 1 week | 01 |
| **05** | [Timeline & Animation](./05-TIMELINE-AND-ANIMATION.md) | GPU interpolation, large trajectory support | 4–5 days | 02 |
| **06** | [Interaction: Selection & Measurement](./06-INTERACTION-SELECTION-MEASUREMENT.md) | Picking instanced atoms, inspector data | 4–5 days | 02 |
| **07** | [UI & UX](./07-UI-AND-UX.md) | Inspector panel, labels, progress, polish | 1 week | 06 |
| **08** | [Visualization Modes](./08-VISUALIZATION-MODES.md) | Wireframe, licorice tuning, points mode | 1 week | 03 |
| **09** | [Export](./09-EXPORT.md) | Fix OBJ/glTF for instanced path, video, POV-Ray | 1 week | 03 |
| **10** | [Performance Optimization](./10-PERFORMANCE-OPTIMIZATION.md) | Frustum culling, spatial index, async load, LOD | 2 weeks | 02 |
| **11** | [Advanced Features](./11-ADVANCED-FEATURES.md) | Surface, cartoon, analysis tools | 3+ weeks | 08 |
| **12** | [Testing, CI & Team Workflow](./12-TESTING-CI-AND-TEAM-WORKFLOW.md) | Tests, benchmarks, parallel dev assignments | Ongoing | 01 |

---

## Suggested Sprint Schedule (6 Developers)

```
Week 1:  Everyone → 01, then split 02 (2 devs) + 04 (1 dev) + 12 (1 dev)
Week 2:  02 finish → 03 (2 devs) + 05 (1 dev) + 06 start (1 dev) + 12
Week 3:  03 finish → 06 finish (1 dev) + 07 (1 dev) + 08 (1 dev) + 09 (1 dev) + 10 start (1 dev)
Week 4:  07–09 finish → 10 (2 devs) + 11 planning + 12
Week 5+: 11 (advanced) + 10 (LOD/culling) + polish
```

---

## Key Files Reference

```
src/
├── lib.rs                    # Plugin registration
├── main.rs                   # Binary entry, scene setup
├── core/                     # Atom, Bond, Trajectory, RenderMode
├── io/                       # xyz, pdb, gro, dcd, mmcif parsers
├── rendering/
│   ├── mod.rs                # Mesh generation
│   └── instanced.rs          # GPU instanced pipeline (critical)
├── systems/
│   ├── loading.rs            # LoadFileEvent pipeline
│   ├── bonds.rs              # Bond detection (needs rewrite)
│   ├── timeline.rs           # Playback & interpolation
│   └── visualization.rs      # Render mode switching
├── interaction/              # selection.rs, measurement.rs
├── ui/mod.rs                 # EGUI panels
└── export/                   # screenshot, obj, gltf_export
```

---

## Success Metrics (v1.0 Release)

- [ ] 100,000 atoms @ 60 FPS on mid-range GPU (instanced rendering)
- [ ] All 5 formats loadable from UI (XYZ, PDB, GRO, DCD+topology, mmCIF)
- [ ] Bonds render correctly in ball-and-stick / licorice modes
- [ ] Click-to-select atom + distance/angle/dihedral measurements
- [ ] Timeline playback with interpolation on 10,000+ frame trajectories
- [ ] Screenshot + OBJ + glTF export working
- [ ] `cargo test` passes, clippy clean, benchmark suite in CI

---

## Related Existing Docs

- [ARCHITECTURE.md](../ARCHITECTURE.md) — system diagram
- [DEVELOPMENT_PLAN.md](../DEVELOPMENT_PLAN.md) — original full spec
- [BUILD_OUT_ROADMAP.md](../BUILD_OUT_ROADMAP.md) — prior gap analysis
- [OPTIMIZATION_PROGRESS.md](../OPTIMIZATION_PROGRESS.md) — GPU optimization tracker
