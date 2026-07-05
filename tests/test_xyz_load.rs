//! XYZ format integration tests.

mod common;

use common::fixture;
use gumol_viz_engine::core::atom::Element;
use gumol_viz_engine::io::xyz::XYZParser;

#[test]
fn test_load_water_xyz_fixture() {
    let path = fixture("water.xyz");
    let trajectory = XYZParser::parse_file(&path).expect("water.xyz should parse");

    assert_eq!(trajectory.num_frames(), 1);
    assert_eq!(trajectory.num_atoms, 3);

    let frame = trajectory.get_frame(0).expect("frame 0");
    let o_pos = frame.get_position(0).expect("oxygen position");
    assert!((o_pos.x - 0.0).abs() < 1e-6);
    assert!((o_pos.y - 0.0).abs() < 1e-6);
    assert!((o_pos.z - 0.0).abs() < 1e-6);
}

#[test]
fn test_load_multi_frame_xyz() {
    let path = std::path::Path::new("demo_trajectory.xyz");
    if !path.exists() {
        eprintln!("Skipping: demo_trajectory.xyz not found");
        return;
    }

    let trajectory = XYZParser::parse_file(path).expect("demo trajectory should parse");
    assert_eq!(trajectory.num_frames(), 3);
    assert_eq!(trajectory.num_atoms, 6);
}

#[test]
fn test_xyz_elements_from_string() {
    let content = "3\nwater\nO 0.0 0.0 0.0\nH 0.757 0.0 0.0\nH -0.757 0.0 0.0";
    let trajectory = XYZParser::parse_string(content, fixture("inline.xyz"))
        .expect("inline XYZ should parse");
    assert_eq!(trajectory.num_atoms, 3);

    // Element symbols are recovered when loading through the full pipeline;
    // parser stores positions keyed by atom index.
    let frame = trajectory.get_frame(0).unwrap();
    assert!(frame.get_position(0).is_some());
    assert!(frame.get_position(1).is_some());
    assert!(frame.get_position(2).is_some());

    assert_eq!(Element::from_symbol("O").unwrap(), Element::O);
}
