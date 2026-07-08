//! Minimal binary DCD writer for integration tests.

use byteorder::{LittleEndian, WriteBytesExt};
use std::io::Write;
use std::path::Path;

/// Write a tiny NAMD/CHARMM-style DCD with `num_atoms` and `num_frames`.
///
/// Coordinates are generated deterministically: atom `i` at frame `f` is
/// `(i + f * 0.1, 0, 0)`.
pub fn write_minimal_dcd(path: &Path, num_atoms: u32, num_frames: u32) -> std::io::Result<()> {
    let mut file = std::fs::File::create(path)?;

    file.write_i32::<LittleEndian>(84)?;
    file.write_all(b"CORD")?;
    file.write_i32::<LittleEndian>(num_frames as i32)?;
    file.write_i32::<LittleEndian>(0)?;
    file.write_i32::<LittleEndian>(1)?;
    file.write_i32::<LittleEndian>(num_frames as i32)?;
    for _ in 0..5 {
        file.write_i32::<LittleEndian>(0)?;
    }
    file.write_i32::<LittleEndian>(0)?;
    file.write_f64::<LittleEndian>(0.02)?;
    for _ in 0..9 {
        file.write_i32::<LittleEndian>(0)?;
    }
    file.write_i32::<LittleEndian>(0)?;
    file.write_i32::<LittleEndian>(0)?;
    file.write_i32::<LittleEndian>(0)?; // n_title = 0

    file.write_i32::<LittleEndian>(4)?;
    file.write_i32::<LittleEndian>(num_atoms as i32)?;
    file.write_i32::<LittleEndian>(4)?;

    for frame in 0..num_frames {
        let shift = frame as f32 * 0.1;
        let xs: Vec<f32> = (0..num_atoms).map(|i| i as f32 + shift).collect();
        let ys = vec![0.0f32; num_atoms as usize];
        let zs = vec![0.0f32; num_atoms as usize];
        write_coord_record(&mut file, &xs)?;
        write_coord_record(&mut file, &ys)?;
        write_coord_record(&mut file, &zs)?;
    }

    Ok(())
}

fn write_coord_record(file: &mut std::fs::File, coords: &[f32]) -> std::io::Result<()> {
    let size = (coords.len() * 4) as i32;
    file.write_i32::<LittleEndian>(size)?;
    for value in coords {
        file.write_f32::<LittleEndian>(*value)?;
    }
    file.write_i32::<LittleEndian>(size)?;
    Ok(())
}
