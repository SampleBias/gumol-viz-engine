//! File format detection for all supported extensions.

use gumol_viz_engine::io::FileFormat;
use std::path::Path;

#[test]
fn test_format_from_path_extensions() {
    let cases = [
        ("molecule.xyz", FileFormat::XYZ),
        ("protein.pdb", FileFormat::PDB),
        ("box.gro", FileFormat::GRO),
        ("traj.dcd", FileFormat::DCD),
        ("structure.cif", FileFormat::MmCIF),
        ("structure.mmcif", FileFormat::MmCIF),
        ("structure.mcif", FileFormat::MmCIF),
        ("unknown.dat", FileFormat::Unknown),
    ];

    for (name, expected) in cases {
        assert_eq!(
            FileFormat::from_path(Path::new(name)),
            expected,
            "extension for {name}"
        );
    }
}

#[test]
fn test_loadable_formats() {
    for format in [
        FileFormat::XYZ,
        FileFormat::PDB,
        FileFormat::GRO,
        FileFormat::MmCIF,
        FileFormat::DCD,
    ] {
        assert!(
            FileFormat::is_loadable(&format),
            "{format:?} should be loadable"
        );
    }

    assert!(!FileFormat::is_loadable(&FileFormat::Unknown));
}

#[test]
fn test_format_from_content() {
    let xyz = "3\nwater\nO 0.0 0.0 0.0\nH 0.0 0.9 0.0\nH 0.0 -0.9 0.0";
    assert_eq!(FileFormat::from_content(xyz), FileFormat::XYZ);

    let pdb = "ATOM      1  N   ALA A   1       0.000   0.000   0.000";
    assert_eq!(FileFormat::from_content(pdb), FileFormat::PDB);

    let gro = "Water\n    3\n    1SOL    OW    1   0.126   0.639   0.322   0.0001   0.0002   0.0003";
    assert_eq!(FileFormat::from_content(gro), FileFormat::GRO);
}
