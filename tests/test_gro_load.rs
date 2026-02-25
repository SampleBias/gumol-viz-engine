//! GRO parser integration test
//!
//! This test module verifies that GRO files can be loaded
//! via the file loading system.

use gumol_viz_engine::io::{FileFormat, IOResult};
use gumol_viz_engine::io::gro::GroParser;
use std::path::Path;

#[test]
fn test_gro_file_load_via_file_format_detection() {
    // Test that FileFormat detects .gro files correctly
    let path = Path::new("examples/water.gro");
    assert_eq!(FileFormat::from_path(path), FileFormat::GRO);
    assert_eq!(FileFormat::is_loadable(&FileFormat::GRO), true);
}

#[test]
fn test_load_actual_gro_file() {
    // Test loading the actual example file
    let path = Path::new("examples/water.gro");

    if !path.exists() {
        eprintln!("Warning: examples/water.gro not found - skipping test");
        return;
    }

    // Parse the file
    let result = GroParser::parse_file(path);

    assert!(result.is_ok(), "Should successfully parse water.gro");

    let trajectory = result.unwrap();

    // Verify basic properties
    assert_eq!(trajectory.num_frames(), 1, "Should have 1 frame");
    assert_eq!(trajectory.num_atoms, 3, "Should have 3 atoms");

    // Verify metadata
    assert!(trajectory.metadata.title.contains("Water"), "Title should contain 'Water'");
    assert_eq!(trajectory.metadata.software, "GROMACS");

    // Get frame data
    let frame = trajectory.get_frame(0).expect("Should have frame 0");

    // Verify positions
    let pos_0 = frame.get_position(0).expect("Should have atom 0 position");
    assert!((pos_0.x - 0.126).abs() < 0.001, "Oxygen X coordinate");
    assert!((pos_0.y - 0.639).abs() < 0.001, "Oxygen Y coordinate");
    assert!((pos_0.z - 0.322).abs() < 0.001, "Oxygen Z coordinate");

    // Verify velocities are present
    assert!(frame.velocities.is_some(), "Should have velocities");

    let velocities = frame.velocities.as_ref().unwrap();
    assert_eq!(velocities.len(), 3, "Should have 3 velocities");

    let vel_0 = velocities.get(&0).expect("Should have atom 0 velocity");
    assert!((vel_0.x - 0.0001).abs() < 0.0001, "Oxygen X velocity");
    assert!((vel_0.y - 0.0002).abs() < 0.0001, "Oxygen Y velocity");
    assert!((vel_0.z - 0.0003).abs() < 0.0001, "Oxygen Z velocity");

    // Verify box dimensions
    assert!(frame.box_size.is_some(), "Should have box dimensions");

    let box_size = frame.box_size.unwrap();
    assert_eq!(box_size[0], 0.0, "Box X should be 0.0");
    assert_eq!(box_size[1], 0.0, "Box Y should be 0.0");
    assert_eq!(box_size[2], 0.0, "Box Z should be 0.0");

    println!("✅ GRO file load test passed!");
}

#[test]
fn test_gro_format_from_content_detection() {
    // Test that FileFormat can detect GRO from content
    let gro_content = "    1SOL    OW    1   0.126   0.639   0.322";
    let format = FileFormat::from_content(gro_content);
    assert_eq!(format, FileFormat::GRO, "Should detect GRO format from content");
}
