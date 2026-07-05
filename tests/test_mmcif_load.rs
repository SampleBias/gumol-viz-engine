//! mmCIF format integration tests.

mod common;

use common::{example, require_path};
use gumol_viz_engine::core::atom::Element;
use gumol_viz_engine::io::mmcif::MmcifParser;

#[test]
fn test_load_water_cif() {
    let path = example("water.cif");
    if !require_path(&path) {
        return;
    }

    let trajectory = MmcifParser::parse_file(&path).expect("water.cif should parse");
    assert_eq!(trajectory.num_frames(), 1);
    assert_eq!(trajectory.num_atoms, 3);

    let frame = trajectory.get_frame(0).expect("frame 0");
    let o = frame.get_position(0).expect("oxygen");
    assert!((o.x - 0.0).abs() < 1e-3);
}

#[test]
fn test_mmcif_atom_site_loop_from_string() {
    let content = include_str!("../examples/water.cif");
    let trajectory = MmcifParser::parse_string(content, example("water.cif"))
        .expect("water.cif content should parse");

    assert_eq!(trajectory.num_atoms, 3);
    let frame = trajectory.get_frame(0).unwrap();
    assert!(frame.get_position(0).is_some());
    assert!(frame.get_position(1).is_some());
    assert!(frame.get_position(2).is_some());

    // Sanity check element parsing path used by mmCIF loader metadata.
    assert_eq!(Element::from_symbol("O").unwrap(), Element::O);
}
