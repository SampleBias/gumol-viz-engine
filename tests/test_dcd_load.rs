//! DCD + topology integration tests.

mod common;

use common::fixture;
use gumol_viz_engine::core::atom::Element;
use gumol_viz_engine::io::load_topology;
use gumol_viz_engine::io::topology::validate_atom_count;
use gumol_viz_engine::io::FileFormat;

#[test]
fn test_topology_from_1crn_pdb() {
    let path = fixture("1CRN.pdb");
    if !common::require_path(&path) {
        return;
    }

    let (atoms, bonds) = load_topology(&path).expect("1CRN topology should load");
    assert_eq!(atoms.len(), 327);
    assert!(!bonds.is_empty(), "CONECT records should produce bonds");

    let ca = atoms.iter().find(|a| a.name == "CA").expect("CA atom");
    assert_eq!(ca.element, Element::C);
}

#[test]
fn test_topology_atom_count_validation() {
    assert!(validate_atom_count(100, 100).is_ok());
    assert!(validate_atom_count(100, 99).is_err());
}

#[test]
fn test_dcd_magic_detection() {
    assert_eq!(FileFormat::from_bytes(&84_i32.to_le_bytes()), FileFormat::DCD);
}
