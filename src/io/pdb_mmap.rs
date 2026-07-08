//! Memory-mapped PDB parsing for large structure files.

use crate::core::atom::AtomData;
use crate::core::bond::BondData;
use crate::core::trajectory::Trajectory;
use crate::io::pdb::PDBParser;
use crate::io::{IOError, IOResult};
use memmap2::Mmap;
use std::fs::File;
use std::path::Path;

/// Use mmap when the PDB file is at least this large (512 KiB).
pub const MMAP_THRESHOLD_BYTES: u64 = 512 * 1024;

/// Parse a PDB file via memory map (avoids an extra kernel read buffer copy).
pub fn parse_file_mmap(path: &Path) -> IOResult<(Trajectory, Vec<AtomData>, Vec<BondData>)> {
    let file = File::open(path).map_err(|_| IOError::FileNotFound(path.display().to_string()))?;
    let mmap = unsafe { Mmap::map(&file) }.map_err(IOError::Io)?;
    let content = std::str::from_utf8(&mmap)
        .map_err(|_| IOError::InvalidFormat(format!("{} is not valid UTF-8", path.display())))?;
    PDBParser::parse_string(content, path.to_path_buf())
}

/// Auto-select mmap parsing for large PDB files.
pub fn parse_file_optimized(path: &Path) -> IOResult<(Trajectory, Vec<AtomData>, Vec<BondData>)> {
    let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    if size >= MMAP_THRESHOLD_BYTES {
        parse_file_mmap(path)
    } else {
        PDBParser::parse_file_buffered(path)
    }
}
