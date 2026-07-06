//! PDB format integration tests.

mod common;

use common::fixture;
use gumol_viz_engine::core::atom::Element;
use gumol_viz_engine::io::pdb::PDBParser;

#[test]
fn test_load_mini_pdb_atom_count() {
    let path = fixture("mini.pdb");
    let (trajectory, atoms, bonds) =
        PDBParser::parse_file_with_atoms(&path).expect("mini.pdb should parse");

    assert_eq!(trajectory.num_frames(), 1);
    assert_eq!(atoms.len(), 5);
    assert!(!bonds.is_empty(), "CONECT records should produce bonds");

    let elements: Vec<Element> = atoms.iter().map(|a| a.element).collect();
    assert!(elements.contains(&Element::N));
    assert!(elements.contains(&Element::C));
    assert!(elements.contains(&Element::O));
}

#[test]
fn test_pdb_parse_atom_hetatm_conect_cryst1() {
    let content = r#"CRYST1   10.000   10.000   10.000  90.00  90.00  90.00 P 1
ATOM      1  N   ALA A   1       0.000   0.000   0.000  1.00  0.00           N
HETATM    2  O   HOH A   2       1.000   0.000   0.000  1.00  0.00           O
CONECT    1    2
END
"#;
    let (_trajectory, atoms, bonds) =
        PDBParser::parse_string(content, fixture("inline.pdb")).expect("inline PDB should parse");

    assert_eq!(atoms.len(), 2);
    assert_eq!(atoms[0].element, Element::N);
    assert_eq!(atoms[1].element, Element::O);
    assert!(!bonds.is_empty());
}

#[test]
fn test_load_1crn_pdb_metadata() {
    let path = fixture("1CRN.pdb");
    if !common::require_path(&path) {
        return;
    }

    let (trajectory, atoms, _bonds) =
        PDBParser::parse_file_with_atoms(&path).expect("1CRN.pdb should parse");

    assert_eq!(trajectory.num_atoms, 327);
    assert_eq!(atoms.len(), 327);
    assert_eq!(trajectory.num_frames(), 1);

    let elements: Vec<Element> = atoms.iter().map(|a| a.element).collect();
    assert!(elements.contains(&Element::N));
    assert!(elements.contains(&Element::C));
    assert!(elements.contains(&Element::O));
    assert!(elements.contains(&Element::S), "1CRN has disulfide (SG)");

    let ca_atoms: Vec<_> = atoms.iter().filter(|a| a.name == "CA").collect();
    assert!(!ca_atoms.is_empty());
    assert!(ca_atoms.iter().all(|a| a.element == Element::C));

    let thr1_n = atoms.iter().find(|a| a.residue_name == "THR" && a.name == "N");
    assert_eq!(thr1_n.map(|a| a.element), Some(Element::N));
}
