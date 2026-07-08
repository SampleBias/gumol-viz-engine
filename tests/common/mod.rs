//! Shared helpers for integration tests.

#![allow(dead_code)]

pub mod dcd_fixture;

use std::path::{Path, PathBuf};

/// Path to a fixture file under `tests/fixtures/`.
pub fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

/// Path to an example file under `examples/`.
pub fn example(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join(name)
}

/// Skip the test when a path is missing (e.g. optional large fixtures).
pub fn require_path(path: &Path) -> bool {
    if path.exists() {
        true
    } else {
        eprintln!("Skipping: {} not found", path.display());
        false
    }
}
