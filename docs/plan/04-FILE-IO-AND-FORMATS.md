# 04 — File I/O & Formats

**Priority:** P1  
**Estimated effort:** 1 developer-week  
**Dependencies:** 01 (UI extensions fix)  
**Can parallelize with:** 02, 03

---

## Goal

All five formats load correctly with accurate atom metadata, multi-frame support, and a path toward streaming large trajectories.

---

## Current State

| Format | Parser | Loading wired | UI loadable | Atom metadata | Multi-frame |
|--------|--------|---------------|-------------|---------------|-------------|
| XYZ | ✅ | ✅ | ✅ | ✅ | ✅ |
| PDB | ✅ | ✅ | ✅ | ⚠️ placeholder | ✅ |
| GRO | ✅ | ✅ | ✅ | ✅ | ✅ (single) |
| mmCIF | ✅ | ✅ | ❌ (UI) | ✅ | ✅ (single) |
| DCD | ✅ | ✅ | ❌ (UI) | ❌ placeholder | ✅ |

---

## Tasks

### Task 4.1 — Enable all formats in UI
**Effort:** 1 hour (may overlap 01.2)  
**Files:** `src/ui/mod.rs`

- [ ] `LOADABLE_EXTENSIONS` includes all formats
- [ ] Status bar shows format name and frame count on load
- [ ] Error toast on unsupported/parse failure

---

### Task 4.2 — Fix PDB atom metadata (if not done in 01)
**Effort:** 4 hours  
**Files:** `src/io/pdb.rs`, `src/systems/loading.rs`

- [ ] Return `Vec<AtomData>` from parser with element, residue, chain, B-factor
- [ ] Parse CONECT records into `Vec<BondData>` stored in trajectory metadata
- [ ] Tests: 1CRN, water.pdb

---

### Task 4.3 — DCD + topology file pairing
**Effort:** 1 day  
**Files:** `src/systems/loading.rs`, `src/ui/mod.rs`, new `TopologyFile` resource

DCD contains only coordinates — needs a structure file.

- [ ] Add `TopologyState` resource:
  ```rust
  pub struct TopologyState {
      pub path: Option<PathBuf>,
      pub atom_data: Vec<AtomData>,
  }
  ```
- [ ] UI flow: load DCD → prompt "Load topology (PDB/GRO/mmCIF)?"
- [ ] Validate atom count matches DCD header
- [ ] CLI: `cargo run -- traj.dcd --topology struct.pdb`

**Acceptance:** DCD trajectory animates with correct element colors.

---

### Task 4.4 — GRO multi-frame trajectories
**Effort:** 1 day  
**Files:** `src/io/gro.rs`

GRO trajectories are concatenated frames in one file.

- [ ] Detect multiple frames (frame size = 3 + num_atoms + 1 lines)
- [ ] Parse all frames into `Trajectory.frames`
- [ ] Preserve box vectors per frame in `FrameData.metadata`
- [ ] Test with `examples/alanine.gro` if multi-frame sample available

---

### Task 4.5 — mmCIF robustness
**Effort:** 1 day  
**Files:** `src/io/mmcif.rs`

- [ ] Handle `_atom_site.*` loop completely (altLoc, occupancy, B-factor)
- [ ] Handle `_struct_conn` for bonds (mmCIF equivalent of CONECT)
- [ ] Test with `examples/water.cif`
- [ ] Add integration test `tests/test_mmcif_load.rs`

---

### Task 4.6 — Format auto-detection improvement
**Effort:** 4 hours  
**Files:** `src/io/mod.rs`

- [ ] When extension is ambiguous, use `FileFormat::from_content()`
- [ ] DCD: check binary magic number (first 4 bytes == 84)
- [ ] Log detected format vs extension mismatch as warning

---

### Task 4.7 — Streaming / memory-mapped loading (foundation)
**Effort:** 2 days  
**Files:** new `src/io/streaming.rs`, `src/systems/loading.rs`

For trajectories >1 GB, loading all frames into RAM is not viable.

- [ ] Define trait:
  ```rust
  pub trait FrameProvider {
      fn num_frames(&self) -> usize;
      fn get_frame(&self, index: usize) -> IOResult<FrameData>;
  }
  ```
- [ ] Implement `MmapXyzProvider` using `memmap2`
- [ ] Implement `DcdFrameProvider` (seek per frame, don't load all)
- [ ] `SimulationData` holds `Box<dyn FrameProvider>` or enum
- [ ] Timeline loads frames on demand (cache last N frames)

**Efficiency:** Start with DCD only (binary, fixed record size) — highest ROI for MD users.

---

### Task 4.8 — Async file loading
**Effort:** 1 day  
**Files:** `src/systems/loading.rs`, `src/ui/mod.rs`

- [ ] Parse file on background thread (use `bevy` `AsyncComputeTaskPool` or `std::thread`)
- [ ] Send result via channel; main thread inserts `SimulationData`
- [ ] UI shows spinner + "Loading…" during parse
- [ ] Cancel in-flight load if user opens new file

**Efficiency:** Prevents UI freeze on 100MB+ files. Depends on 4.7 for very large files.

---

## Parser File Reference

```
src/io/
├── mod.rs       # FileFormat, IOError
├── xyz.rs       # ✅ complete
├── pdb.rs       # needs AtomData + CONECT
├── gro.rs       # needs multi-frame
├── dcd.rs       # needs streaming
└── mmcif.rs     # needs struct_conn
```

---

## Testing Requirements

Each format needs:
1. Unit test: `parse_string()` with minimal valid content
2. Integration test: load example file, assert atom count + frame count
3. Example in `examples/` if not present

```bash
cargo test test_parse_simple_xyz
cargo test --test test_gro_load
cargo test --test test_mmcif_load  # add
cargo test --test test_dcd_load      # add
```

---

## Definition of Done

- [ ] All 5 formats load from UI and CLI
- [ ] DCD requires and uses topology file
- [ ] PDB/mmCIF bonds available for bond system (03)
- [ ] No UI freeze on 50MB XYZ file (async load)
- [ ] Integration test per format
