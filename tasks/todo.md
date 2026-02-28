# Gumol Viz Engine Todo List

## Project Review & Assessment
- [x] Analyze existing codebase structure
- [x] Review implemented features (parsers, core data structures)
- [x] Identify missing functionality
- [x] Create detailed next steps plan

## Phase 1: File Loading & Scene Management (Priority: HIGH)
- [x] Implement `systems/loading.rs` - File loading system for XYZ/PDB files
- [x] Implement `systems/spawning.rs` - Atom entity spawning from trajectory data
- [x] Add `SimulationData` resource to store loaded trajectory
- [x] Create `FileHandle` resource to track currently loaded file
- [x] Add system to parse file and spawn atom entities on startup
- [ ] Test loading actual XYZ and PDB files

## Phase 2: Timeline & Animation (Priority: HIGH)
- [x] Implement `systems/timeline.rs` - Timeline playback system
- [x] Add `update_timeline` system to advance frames during playback
- [x] Implement `update_atom_positions` system to update atom transforms
- [x] Add frame interpolation for smooth animation
- [x] Create timeline UI panel with play/pause/scrub controls
- [ ] Test timeline with multi-frame XYZ files

## Phase 3: Atom Selection & Interaction (Priority: MEDIUM)
- [x] Implement `interaction/selection.rs` - Atom selection via raycasting
- [x] Add `SelectionState` resource to track selected atoms
- [x] Implement selection highlighting (change material/color)
- [x] Add selection box UI in EGUI
- [ ] Test single and multi-atom selection

## Phase 4: Bond Detection & Rendering (Priority: MEDIUM)
- [x] Implement `systems/bonds.rs` - Distance-based bond detection
- [x] Create bond entity spawning system
- [x] Generate bond mesh cylinders between atoms
- [x] Add bond rendering with proper materials
- [x] Implement bond order detection (single/double/triple)
- [ ] Test bond detection on protein structures

## Phase 5: Visualization Modes (Priority: MEDIUM)
- [x] Add `VisualizationConfig` resource with render mode settings
- [x] Implement CPK mode (space-filling atoms)
- [x] Implement Ball-and-Stick mode
- [x] Implement Licorice mode
- [x] Create UI selector for visualization modes
- [x] Add atom size scaling controls

## Phase 6: Measurement Tools (Priority: LOW)
- [x] Implement `interaction/measurement.rs` - Distance calculator
- [x] Add angle measurement tool
- [x] Add dihedral angle measurement
- [x] Create measurement UI display
- [ ] Test measurements on selected atoms

## Phase 7: Export Functionality (Priority: LOW)
- [x] Implement `export/screenshot.rs` - PNG/JPEG screenshot capture
- [x] Add export UI panel
- [ ] Test screenshot functionality

## Documentation & Examples (Priority: MEDIUM)
- [x] Create example XYZ file for testing (demo_trajectory.xyz, examples/water.gro, water.cif)
- [x] Create example PDB file for testing (examples/1CRN.pdb — crambin from RCSB)
- [ ] Update examples/basic_load.rs to use actual file loading
- [x] Update examples/xyz_viewer.rs to implement XYZ viewer
- [x] Update examples/pdb_viewer.rs to implement PDB viewer
- [ ] Add inline documentation to all systems
- [ ] Update README with current feature status

## Testing & Quality
- [x] Add unit tests for loading system
- [x] Add unit tests for timeline system
- [ ] Add integration tests for full workflow
- [ ] Performance test with 10,000 atoms
- [ ] Performance test with 100,000 atoms
- [ ] Fix any clippy warnings

## Review Section
*This section will be updated upon completion with a summary of all changes made during the session.*

---
*Created: 2026-02-23 12:54*
*Last Updated: 2026-02-27*
*Review Date: 2026-02-27*

---

## New Session - 2026-02-23 13:57
- [x] Review existing todo items
- [x] Identify new requirements
- [x] Update task priorities
- [x] Add session-specific tasks

*Session started: 2026-02-23 13:57*

## New Session - 2026-02-23 15:00
- [x] Fix critical compilation errors (load_cli_file, FilePickerState Send issue)
- [x] Test compilation after fixes
- [x] Implement Timeline & Animation system (Phase 2)
- [x] Test timeline with demo_trajectory.xyz
- [x] Update activity log and PROJECT_README

*Session started: 2026-02-23 15:00*

## Critical Bug Fixes (Priority: CRITICAL)
- [ ] Fix panic runtime conflict with Bevy dynamic_linking feature
- [ ] Update Cargo.toml profile configurations
- [ ] Test compilation after fix
- [ ] Verify dynamic linking works correctly

---

## New Session - 2026-02-25 14:51
- [x] Review existing todo items
- [x] Identify new requirements
- [x] Update task priorities
- [x] Add session-specific tasks

*Session started: 2026-02-25 14:51*

## Phase 1: Secondary File Formats (Priority: HIGH)
- [x] Implement GRO format parser (`src/io/gro.rs`) - 434 lines
- [x] Implement DCD format parser (`src/io/dcd.rs`) - 280 lines
- [x] Implement mmCIF format parser (`src/io/mmcif.rs`) - 350+ lines
- [x] Update `src/io/mod.rs` to register new parsers
- [x] Update `FileFormat::is_loadable()` to include secondary formats
- [x] Add GRO format tests
- [x] Add DCD format tests
- [x] Add mmCIF format tests
- [x] Create example GRO file for testing
- [x] Create example mmCIF file for testing
- [x] Load .gro files - Full integration with file loading system
- [x] Document GroParser API in `docs/gro_parser_reference.md` (already exists)
- [ ] Fix pre-existing glTF export compilation errors (blocking testing)
- [ ] Run unit tests for all new parsers
- [ ] Test loading actual mmCIF files
- [x] Complete `create_atom_data_from_mmcif()` implementation
- [x] Update documentation for secondary formats (`docs/SECONDARY_FORMATS.md`)

---

## New Session - 2026-02-27
- [x] Complete `create_atom_data_from_mmcif()` — added `MmcifParser::parse_atom_data_from_file()`, `parse_mmcif_data()`, improved `parse_atom_data()` with alternative column names
- [x] Create `docs/SECONDARY_FORMATS.md` — comprehensive docs for GRO, DCD, mmCIF

---

## New Session - 2025-06-17 09:30 - GPU Performance Optimization
- [x] Comprehensive GPU performance analysis completed
- [x] Identified 6 critical performance bottlenecks
- [x] Created detailed performance analysis document (`docs/GPU_PERFORMANCE_ANALYSIS.md`)
- [x] Created quick start optimization guide (`docs/QUICK_START_OPTIMIZATION.md`)
- [ ] Implement instanced rendering (CRITICAL - Week 1-2)
- [ ] Implement GPU compute for position updates (CRITICAL - Week 3-4)
- [ ] Implement async file loading (CRITICAL - Week 3-4)
- [ ] Implement material pooling (CRITICAL - Week 1-2)
- [ ] Implement spatial partitioning for bond detection (HIGH - Week 7-8)
- [ ] Implement frustum culling (HIGH - Week 7-8)
- [ ] Implement level-of-detail (LOD) system (HIGH - Week 9-10)
- [ ] Implement parallel file parsing with rayon (HIGH - Week 9-10)
- [ ] Add performance profiling with puffin (CRITICAL - Week 1)
- [ ] Create benchmark suite for performance testing (CRITICAL - Week 1)
- [ ] Document baseline performance metrics (CRITICAL - Week 1)

*Session started: 2025-06-17 09:30*
