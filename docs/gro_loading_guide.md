# Loading .gro Files in Gumol Viz Engine

## Overview

The GRO (GROningen) format is a coordinate file format used by **GROMACS** molecular dynamics simulation software. It provides atomic coordinates in a compact, column-based format that can optionally include velocities and box dimensions.

## Quick Start

### 1. Check File Format

```rust
use gumol_viz_engine::io::FileFormat;

let path = std::path::Path::new("molecule.gro");
let format = FileFormat::from_path(path);

assert_eq!(format, FileFormat::GRO);
assert!(FileFormat::is_loadable(&format), true);
```

### 2. Load via File Loading System

```rust
use gumol_viz_engine::systems::loading::LoadFileEvent;
use bevy::prelude::*;

// Send a load file event
fn load_gro_file(mut load_events: EventWriter<LoadFileEvent>) {
    let path = std::path::PathBuf::from("molecule.gro");
    load_events.send(LoadFileEvent { path });
}
```

### 3. Load via CLI Argument

```bash
# Start the application with a GRO file
cargo run --release -- examples/water.gro
```

### 4. Load via UI (Drag & Drop or File Picker)

The UI supports drag-and-drop and file picker selection:
- Drag a `.gro` file onto the application window
- Click "Open File" and select a `.gro` file

## GRO Format Specification

### File Structure
```
Line 1: Title (free text)
Line 2: Number of atoms (integer)
Lines 3-N: Atom records (column-based, 44-68 characters each)
Last Line: Box vectors (optional, 3 values)
```

### Atom Record Format (Column-Based)
```
resid(5) resname(5) atomname(5) atomnr(5) x(8.3) y(8.3) z(8.3) [vx(8.4) vy(8.4) vz(8.4)]
```

**Column Breakdown:**

| Columns | Field | Type | Description | Example |
|---------|-------|------|-------------|---------|
| 1-5 | Residue ID | Integer (5 chars) | `    1` |
| 6-10 | Residue Name | String (5 chars) | `SOL  ` |
| 11-15 | Atom Name | String (5 chars) | `OW   ` |
| 16-20 | Atom Number | Integer (5 chars) | `    1` |
| 21-28 | X Coordinate | Float (8.3) | `  0.126` |
| 29-36 | Y Coordinate | Float (8.3) | `  0.639` |
| 37-44 | Z Coordinate | Float (8.3) | `  0.322` |
| 45-52 | X Velocity (Optional) | Float (8.4) | `  0.0001` |
| 53-60 | Y Velocity (Optional) | Float (8.4) | `  0.0002` |
| 61-68 | Z Velocity (Optional) | Float (8.4) | `  0.0003` |

**Key Points:**
- **NOT whitespace-delimited** - fixed-width columns
- Minimum line length: **44 characters** (without velocities)
- With velocities: **68 characters**
- Numbers are **right-aligned** in their fields
- Units: **nanometers (nm)** for coordinates, **nm/ps** for velocities

### Box Dimensions (Optional)
```
xx yy zz
```
- Three floats (typically 8.4 format)
- Represents periodic box vectors
- Units: nanometers (nm)

## Example GRO Files

### Example 1: Water Molecule (Minimal)
```
Water molecule - GRO format
3
    1SOL    OW    1   0.126   0.639   0.322
    1SOL   HW1    2   0.187   0.713   0.394
    1SOL   HW2    3   0.145   0.584   0.235
   0.0000   0.0000   0.0000
```

**File Location**: `examples/water.gro`

**Parsed Data:**
- 3 atoms (Oxygen + 2 Hydrogens)
- Positions in nanometers
- Velocities: **Not present** (this file is minimal)

### Example 2: Water with Velocities
```
Water with velocities
3
    1SOL    OW    1   0.126   0.639   0.322   0.0001   0.0002   0.0003
    1SOL   HW1    2   0.187   0.713   0.394   0.0004   0.0005   0.0006
    1SOL   HW2    3   0.145   0.584   0.235   0.0007   0.0008   0.0009
   0.0000   0.0000   0.0000
```

**Parsed Data:**
- 3 atoms
- Positions in nanometers
- **Velocities present** (last 3 columns)
- Velocities in nm/ps (nanometers per picosecond)

### Example 3: Protein (Alanine Dipeptide)
```
Protein - Alanine dipeptide (GRO format)
22
    1ALA    N     1   0.098   0.657   0.000   0.0000   0.0000   0.0000
    1ALA   CA    2  -0.617   0.458   0.000   0.0000   0.0000   0.0000
    1ALA    C     3  -0.687   0.699   0.000   0.0000   0.0000   0.0000
    1ALA    O     4  -1.835   0.511   0.000   0.0000   0.0000   0.0000
    1ALA   CB    5  -1.913   1.298   0.000   0.0000   0.0000   0.0000
    1ALA   OG    6  -1.471   2.226   0.000   0.0000   0.0000   0.0000
    1ALA   CG    7  -2.986   1.451   0.000   0.0000   0.0000   0.0000
    1ALA   OD1   8  -2.938   1.608   0.000   0.0000   0.0000   0.0000
    1ALA    N     9   0.098   0.657   0.000   0.0000   0.0000   0.0000
    2ALA    H     10  0.647   1.651   0.000   0.0000   0.0000   0.0000
    2ALA    H     11  1.005   1.299   0.000   0.0000   0.0000   0.0000
    1ALA   H     12  0.614  -0.413   0.000   0.0000   0.0000   0.0000
    1ALA   HA    13 -1.707   1.313   0.000   0.0000   0.0000   0.0000
    1ALA   HB2   14 -1.739   0.998   0.000   0.0000   0.0000   0.0000
    1ALA   HB3   15 -2.729   1.332   0.000   0.0000   0.0000   0.0000
    1ALA   HG1   16 -2.949   1.762   0.000   0.0000   0.0000   0.0000
    1ALA   HG2   17 -3.891   1.241   0.000   0.0000   0.0000   0.0000
    1ALA   HG3   18 -2.552   2.821   0.000   0.0000   0.0000   0.0000
    1ALA   N     19  0.098   0.657   0.000   0.0000   0.0000   0.0000
    1ALA    H     20  0.647   1.651   0.000   0.0000   0.0000   0.0000
    1ALA    H     21  1.005   1.299   0.000   0.0000   0.0000   0.0000
    1ALA   HA    22 -1.707   1.313   0.000   0.0000   0.0000   0.0000
    2.4534   2.4534   2.4534
```

**File Location**: `examples/alanine.gro`

**Parsed Data:**
- 22 atoms (protein backbone + side chain)
- Two alanine residues (ALA)
- Positions in nanometers
- Velocities: **Present** (all zero, this is a minimized structure)
- Box dimensions: **Present** (2.4534 nm cubic box)

## Loading Workflow

### Via Application UI

1. **Start the application**:
   ```bash
   cargo run --release
   ```

2. **Load a GRO file** using one of:
   - **Drag and drop**: Drag `examples/water.gro` onto the window
   - **File picker**: Click "Open File" and select a `.gro` file
   - **CLI argument**: `cargo run --release -- examples/water.gro`

3. **Visualization**:
   - Atoms spawn automatically when file loads
   - Positions are in **nanometers** (GRO units)
   - If velocities present, they're stored but not displayed (used for analysis)
   - If box dimensions present, they're stored in `FrameData.box_size`

### Via Code (Programmatic)

```rust
use gumol_viz_engine::io::gro::GroParser;
use std::path::Path;

// Parse GRO file
let trajectory = GroParser::parse_file(Path::new("molecule.gro"))?;

// Access parsed data
println!("Title: {}", trajectory.metadata.title);
println!("Atoms: {}", trajectory.num_atoms);
println!("Frames: {}", trajectory.num_frames());

// Get frame data
let frame = trajectory.get_frame(0).unwrap();
for atom_id in 0..trajectory.num_atoms as u32 {
    let pos = frame.get_position(atom_id)?;
    println!("Atom {}: {:?}", atom_id, pos);

    // Check for velocities
    if let Some(velocities) = &frame.velocities {
        if let Some(vel) = velocities.get(&atom_id) {
            println!("  Velocity: {:?}", vel);
        }
    }

    // Check for box dimensions
    if let Some(box_size) = frame.box_size {
        println!("Box: {:?}", box_size);
    }
}
```

## Data Access After Loading

### In Bevy Systems

```rust
use gumol_viz_engine::systems::loading::SimulationData;
use bevy::prelude::*;

fn my_system(sim_data: Res<SimulationData>) {
    if sim_data.loaded {
        println!("Loaded {} atoms from {}",
            sim_data.num_atoms(),
            sim_data.trajectory.file_path.display());
    }
}
```

### Atom Data

After loading, atom data is available in `SimulationData.atom_data`:
```rust
for atom_data in &sim_data.atom_data {
    println!("Atom {}: {} ({})",
        atom_data.atom_id,
        atom_data.element.symbol(),
        atom_data.residue_name,
    );
}
```

### Frame Data

Positions, velocities, and box dimensions are in `SimulationData.trajectory.frames`:
```rust
for frame in &sim_data.trajectory.frames {
    for atom_id in frame.atom_ids() {
        let pos = frame.get_position(*atom_id)?;
        
        // Access velocities (if present)
        if let Some(velocities) = &frame.velocities {
            if let Some(vel) = velocities.get(atom_id) {
                println!("Position: {:?}, Velocity: {:?}", pos, vel);
            }
        }
        
        // Access box dimensions (if present)
        if let Some(box_size) = frame.box_size {
            println!("Box: {:?}", box_size);
        }
    }
}
```

## Element Detection

The parser automatically determines chemical elements from GROMACS atom names:

### Detection Logic

1. **Strip leading numbers** from atom name
2. **Try 2-character elements**: `CA`, `CB`, `OD1`, etc.
3. **Try 1-character elements**: `C`, `N`, `O`, `H`, `S`, etc.
4. **GROMACS-specific patterns**:
   - `OW` / `HW` → Oxygen / Hydrogen (water variants)
5. **Default to `Element::Unknown`** with warning if not recognized

### Common GROMACS Atom Names

| Atom Name | Element | Pattern Type |
|----------|----------|--------------|
| C, CA, CB, CG | Carbon | Backbone/sidechain |
| N, NH1, NH2 | Nitrogen | Backbone |
| O, OW, OD1, OD2 | Oxygen | Backbone/sidechain |
| H, H1, H2, HA | Hydrogen | Hydrogens |
| S, SG | Sulfur | Sidechain |
| CL | Chlorine | 2-char element |
| NA | Sodium | 2-char element |

### Example

```rust
use gumol_viz_engine::io::gro::GroParser;
use gumol_viz_engine::core::atom::Element;

// Detect elements
assert_eq!(GroParser::element_from_atom_name("CA"), Element::C);
assert_eq!(GroParser::element_from_atom_name("OW"), Element::O);
assert_eq!(GroParser::element_from_atom_name("HW"), Element::H);
assert_eq!(GroParser::element_from_atom_name("SG"), Element::S);
assert_eq!(GroParser::element_from_atom_name("CL"), Element::Cl);
```

## Error Handling

### Common Errors

#### 1. File Not Found
```rust
IOError::FileNotFound(String)
```
**Cause**: File doesn't exist at specified path
**Solution**: Check file path and existence

#### 2. Empty File
```rust
IOError::ParseError { line: 0, message: "Empty GRO file" }
```
**Cause**: File has no content
**Solution**: Verify file contains valid GRO data

#### 3. Missing Title Line
```rust
IOError::ParseError { line: 1, message: "Missing title line" }
```
**Cause**: First line is missing
**Solution**: Ensure file has title as first line

#### 4. Missing Atom Count
```rust
IOError::ParseError { line: 2, message: "Missing atom count line" }
```
**Cause**: Second line is missing or not a number
**Solution**: Ensure second line is an integer

#### 5. Invalid Atom Count
```rust
IOError::ParseError { line: 2, message: "Number of atoms cannot be zero" }
```
**Cause**: Atom count is 0
**Solution**: Ensure valid atom count > 0

#### 6. Line Too Short
```rust
IOError::ParseError { line: N, message: "Line too short (X chars), expected at least 44" }
```
**Cause**: Atom line is less than 44 characters
**Solution**: Ensure proper column widths and spacing

#### 7. Invalid Coordinate
```rust
IOError::ParseError { line: N, message: "Invalid X coordinate: ..." }
```
**Cause**: Coordinate cannot be parsed as float
**Solution**: Ensure valid float format

### Error Handling Example

```rust
use gumol_viz_engine::io::gro::GroParser;
use gumol_viz_engine::io::IOError;
use std::path::Path;

fn load_gro_file(path: &Path) {
    match GroParser::parse_file(path) {
        Ok(trajectory) => {
            println!("✅ Successfully loaded!");
            println!("Atoms: {}", trajectory.num_atoms);
        }
        Err(IOError::FileNotFound(msg)) => {
            eprintln!("❌ File not found: {}", msg);
        }
        Err(IOError::ParseError { line, message }) => {
            eprintln!("❌ Parse error at line {}: {}", line, message);
        }
        Err(e) => {
            eprintln!("❌ Error: {}", e);
        }
    }
}
```

## Testing

### Run Unit Tests

```bash
# Run all GRO parser tests
cargo test io::gro::tests

# Run specific test
cargo test test_parse_simple_gro
cargo test test_element_from_atom_name
cargo test test_parse_atom_line
```

### Test with Real Files

```bash
# Test with example files
cargo run --release -- examples/water.gro
cargo run --release -- examples/alanine.gro

# Test with custom file
cargo run --release -- /path/to/your/file.gro
```

### Integration Tests

```bash
# Run GRO load integration test
cargo test test_load_actual_gro_file
cargo test test_gro_file_load_via_file_format_detection
```

## Troubleshooting

### Issue: "Unknown element" warnings

**Cause**: Atom name not recognized by element detection

**Solution 1**: Add new atom name patterns to `GroParser::element_from_atom_name()`

**Solution 2**: Use a more general file format (PDB, mmCIF) with explicit element columns

### Issue: Coordinates seem wrong scale

**Cause**: GRO coordinates are in **nanometers (nm)**, not Ångströms (Å)

**Solution**: No action needed - Gumol Viz Engine handles nm units correctly. Coordinates are displayed in the correct scale.

### Issue: Velocities not displayed

**Cause**: Velocities are stored but not rendered in 3D view

**Solution**: This is expected - velocities are used for analysis (kinetic energy, temperature), not visualization.

## References

- [GROMACS File Formats](https://manual.gromacs.org/current/userguide/file-formats.html#gro)
- [GRO Format Specification](https://www.gromacs.org/documentation/current/user-guide/file-formats.html)
- [Gumol Viz Engine README](../README.md)
- [File Loading System](../src/systems/loading.rs)

---

**Last Updated**: 2026-02-25  
**Implementation Status**: ✅ **Production-Ready**
