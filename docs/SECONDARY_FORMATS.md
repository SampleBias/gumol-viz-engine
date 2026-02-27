# Secondary File Formats

This document describes the secondary molecular file formats supported by Gumol Viz Engine: GRO, DCD, and mmCIF.

## Overview

| Format | Extension | Parser | Atom Metadata | Multi-Frame |
|--------|-----------|--------|---------------|-------------|
| GRO    | .gro      | GroParser | Yes (from file) | Single frame |
| DCD    | .dcd     | DcdParser | No (placeholder) | Yes |
| mmCIF  | .cif, .mmcif, .mcif | MmcifParser | Yes (from file) | Single frame |

## GRO Format (GROMACS)

**Location**: `src/io/gro.rs`

### Features
- Column-based coordinate format used by GROMACS
- Full atom metadata: residue, atom name, element
- Optional velocities
- Optional periodic box dimensions
- Read and write support

### File Structure
```
Title line
Number of atoms
resid(5) resname(5) atomname(5) atomnr(5) x(8.3) y(8.3) z(8.3) [vx(8.4) vy(8.4) vz(8.4)]
...
box_xx box_yy box_zz (optional)
```

### API
- `GroParser::parse_file(path)` → `Trajectory`
- `GroParser::parse_reader(reader, path)` → `Trajectory`
- `GroParser::parse_string(content, path)` → `Trajectory`
- `GroParser::parse_atom_line(line, line_num, atom_id)` → `ParsedAtom`
- `GroWriter::write_trajectory(path, trajectory)` → `IOResult<()>`

### Loading
GRO files are loaded via `LoadFileEvent`. The loading system calls `create_atom_data_from_gro()` to extract atom metadata (element, residue, chain) by re-parsing the file.

### Reference
See [gro_parser_reference.md](gro_parser_reference.md) for full API documentation.

---

## DCD Format (CHARMM/NAMD)

**Location**: `src/io/dcd.rs`

### Features
- Binary trajectory format from CHARMM and NAMD
- Multiple frames with coordinates only
- Little-endian byte order
- Header with frame count, atom count, timestep

### Limitations
- **No atom metadata**: DCD files contain only coordinates
- Requires a separate structure file (PDB, GRO) for atom types
- Loading uses `create_placeholder_atom_data()` — all atoms appear as "Unknown"

### File Structure
- 224-byte header
- Magic number: 84
- Per-frame: 3 × num_atoms × 4 bytes (x, y, z floats)

### API
- `DcdParser::parse_file(path)` → `Trajectory`
- `DcdParser::parse_reader(reader, path)` → `Trajectory`

### Future Work
- Pair DCD with structure file for proper atom metadata
- Memory-mapped streaming for large trajectories

---

## mmCIF Format (Macromolecular CIF)

**Location**: `src/io/mmcif.rs`

### Features
- Text-based format for macromolecular structures
- Hierarchical key-value structure
- Loop-based `atom_site` records
- Supports larger structures than PDB
- Full atom metadata: element, residue, chain

### File Structure
```
data_blockname
#
loop_
_atom_site.group_PDB
_atom_site.id
_atom_site.type_symbol
_atom_site.label_atom_id
_atom_site.label_comp_id
_atom_site.label_asym_id
_atom_site.label_seq_id
_atom_site.Cartn_x
_atom_site.Cartn_y
_atom_site.Cartn_z
ATOM 1 O O . HOH A 1 . 0.000 0.000 0.000
...
```

### Supported Column Names
The parser supports both `label_*` and `auth_*` column variants:
- Atom: `label_atom_id`, `id`, `type_symbol`
- Residue: `label_comp_id`, `auth_comp_id`
- Residue ID: `label_seq_id`, `auth_seq_id`
- Chain: `auth_asym_id`, `label_asym_id`
- Element: `type_symbol` (preferred)

### API
- `MmcifParser::parse_file(path)` → `Trajectory`
- `MmcifParser::parse_reader(reader, path)` → `Trajectory`
- `MmcifParser::parse_string(content, path)` → `Trajectory`
- `MmcifParser::parse_atom_data_from_file(path)` → `Vec<AtomData>`
- `MmcifWriter::write_trajectory(path, trajectory)` → `IOResult<()>`

### Loading
mmCIF files are loaded via `LoadFileEvent`. The loading system calls `create_atom_data_from_mmcif()` which uses `MmcifParser::parse_atom_data_from_file()` to extract full atom metadata (element, residue name, residue ID, chain ID).

---

## File Format Detection

`FileFormat::from_path()` detects format from extension:
- `.gro` → GRO
- `.dcd` → DCD
- `.cif`, `.mmcif`, `.mcif` → MmCIF

`FileFormat::is_loadable()` returns true for GRO, DCD, and MmCIF (all implemented).

---

## Integration

All formats feed into the same pipeline:
1. `LoadFileEvent` → `handle_load_file_events`
2. `load_file(path)` dispatches to appropriate parser
3. Parser returns `(Trajectory, Vec<AtomData>)`
4. `SimulationData` and `FileHandle` updated
5. `FileLoadedEvent` triggers spawning and camera centering

---

*Last updated: 2026-02-27*
