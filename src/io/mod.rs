//! File I/O and format parsers
//!
//! This module provides parsers for various molecular file formats.

pub mod xyz;
pub mod pdb;

use bevy::prelude::*;
use thiserror::Error;

/// Register all IO systems
pub fn register(app: &mut App) {
    xyz::register(app);
    pdb::register(app);

    info!("IO module registered");
}

/// Error types for file I/O
#[derive(Error, Debug)]
pub enum IOError {
    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error at line {line}: {message}")]
    ParseError { line: usize, message: String },

    #[error("Invalid format: {0}")]
    InvalidFormat(String),

    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),
}

/// Result type for IO operations
pub type IOResult<T> = Result<T, IOError>;

/// File format detection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileFormat {
    XYZ,
    PDB,
    GRO,
    DCD,
    MmCIF,
    Unknown,
}

impl FileFormat {
    /// Detect file format from file extension
    pub fn from_path(path: &std::path::Path) -> Self {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("xyz") => FileFormat::XYZ,
            Some("pdb") => FileFormat::PDB,
            Some("gro") => FileFormat::GRO,
            Some("dcd") => FileFormat::DCD,
            Some("cif") | Some("mmcif") | Some("mcif") => FileFormat::MmCIF,
            _ => FileFormat::Unknown,
        }
    }

    /// Check if this format is currently loadable (parser implemented)
    pub fn is_loadable(&self) -> bool {
        matches!(self, FileFormat::XYZ | FileFormat::PDB)
    }

    /// Detect file format from content
    pub fn from_content(content: &str) -> Self {
        let first_line = content.lines().next().unwrap_or("");

        // XYZ format: first line is number of atoms
        if first_line.trim().parse::<u32>().is_ok() {
            return FileFormat::XYZ;
        }

        // PDB format: starts with ATOM, HETATM, HEADER, etc.
        let first_word = first_line.split_whitespace().next().unwrap_or("");
        if matches!(
            first_word,
            "ATOM" | "HETATM" | "HEADER" | "TITLE" | "CRYST1" | "REMARK" | "MODEL"
        ) {
            return FileFormat::PDB;
        }

        FileFormat::Unknown
    }
}
