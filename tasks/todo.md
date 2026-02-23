# Gumol Viz Engine Todo List

## Project Review & Assessment
- [x] Analyze existing codebase structure
- [x] Review implemented features (parsers, core data structures)
- [x] Identify missing functionality
- [x] Create detailed next steps plan

## Phase 1: File Loading & Scene Management (Priority: HIGH)
- [x] Implement `systems/loading.rs` - File loading system for XYZ/PDB files
- [x] Implement `systems/spawning.rs` - Atom entity spawning from trajectory data
- [ ] Add `SimulationData` resource to store loaded trajectory
- [ ] Create `FileHandle` resource to track currently loaded file
- [ ] Add system to parse file and spawn atom entities on startup
- [ ] Test loading actual XYZ and PDB files

## Phase 2: Timeline & Animation (Priority: HIGH)
- [ ] Implement `systems/timeline.rs` - Timeline playback system
- [ ] Add `update_timeline` system to advance frames during playback
- [ ] Implement `update_atom_positions` system to update atom transforms
- [ ] Add frame interpolation for smooth animation
- [ ] Create timeline UI panel with play/pause/scrub controls
- [ ] Test timeline with multi-frame XYZ files

## Phase 3: Atom Selection & Interaction (Priority: MEDIUM)
- [ ] Implement `interaction/selection.rs` - Atom selection via raycasting
- [ ] Add `SelectionState` resource to track selected atoms
- [ ] Implement selection highlighting (change material/color)
- [ ] Add selection box UI in EGUI
- [ ] Test single and multi-atom selection

## Phase 4: Bond Detection & Rendering (Priority: MEDIUM)
- [ ] Implement `systems/bonds.rs` - Distance-based bond detection
- [ ] Create bond entity spawning system
- [ ] Generate bond mesh cylinders between atoms
- [ ] Add bond rendering with proper materials
- [ ] Implement bond order detection (single/double/triple)
- [ ] Test bond detection on protein structures

## Phase 5: Visualization Modes (Priority: MEDIUM)
- [ ] Add `VisualizationConfig` resource with render mode settings
- [ ] Implement CPK mode (space-filling atoms)
- [ ] Implement Ball-and-Stick mode
- [ ] Implement Licorice mode
- [ ] Create UI selector for visualization modes
- [ ] Add atom size scaling controls

## Phase 6: Measurement Tools (Priority: LOW)
- [ ] Implement `interaction/measurement.rs` - Distance calculator
- [ ] Add angle measurement tool
- [ ] Add dihedral angle measurement
- [ ] Create measurement UI display
- [ ] Test measurements on selected atoms

## Phase 7: Export Functionality (Priority: LOW)
- [ ] Implement `export/screenshot.rs` - PNG/JPEG screenshot capture
- [ ] Add export UI panel
- [ ] Test screenshot functionality

## Documentation & Examples (Priority: MEDIUM)
- [ ] Create example XYZ file for testing
- [ ] Create example PDB file for testing
- [ ] Update examples/basic_load.rs to use actual file loading
- [ ] Update examples/xyz_viewer.rs to implement XYZ viewer
- [ ] Update examples/pdb_viewer.rs to implement PDB viewer
- [ ] Add inline documentation to all systems
- [ ] Update README with current feature status

## Testing & Quality
- [ ] Add unit tests for loading system
- [ ] Add unit tests for timeline system
- [ ] Add integration tests for full workflow
- [ ] Performance test with 10,000 atoms
- [ ] Performance test with 100,000 atoms
- [ ] Fix any clippy warnings

## Review Section
*This section will be updated upon completion with a summary of all changes made during the session.*

---
*Created: 2026-02-23 12:54*
*Last Updated: 2026-02-23 13:00*
*Review Date: 2026-02-23*