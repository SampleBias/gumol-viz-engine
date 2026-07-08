//! Sprint 6 validation — parallel XYZ I/O and bond order visuals.

use gumol_viz_engine::core::bond::BondOrder;
use gumol_viz_engine::io::xyz::XYZParser;
use gumol_viz_engine::io::xyz_parallel::{parse_file_optimized, parse_lines_parallel, PARALLEL_FRAME_THRESHOLD};
use gumol_viz_engine::systems::bonds::{bond_cylinder_count, bond_cylinder_local_offsets};
use std::path::PathBuf;

fn synthetic_trajectory(frame_count: usize) -> String {
    let mut out = String::new();
    for f in 0..frame_count {
        out.push_str("2\n");
        out.push_str(&format!("frame {f}\n"));
        out.push_str(&format!("C {} 0.0 0.0\n", f as f32 * 0.05));
        out.push_str("H 1.09 0.0 0.0\n");
    }
    out
}

#[test]
fn test_parallel_xyz_matches_buffered_parser() {
    let content = synthetic_trajectory(PARALLEL_FRAME_THRESHOLD + 4);
    let path = PathBuf::from("sprint6_parallel.xyz");
    let lines: Vec<String> = content.lines().map(str::to_string).collect();

    let parallel = parse_lines_parallel(&lines, path.clone()).unwrap();
    let buffered = XYZParser::parse_string(&content, path).unwrap();

    assert_eq!(parallel.num_frames(), buffered.num_frames());
    assert_eq!(parallel.num_atoms, buffered.num_atoms);
}

#[test]
fn test_optimized_parser_on_small_fixture() {
    let path = PathBuf::from("demo_trajectory.xyz");
    if !path.exists() {
        return;
    }
    let optimized = parse_file_optimized(&path).expect("optimized parse");
    let buffered = XYZParser::parse_file_buffered(&path).expect("buffered parse");
    assert_eq!(optimized.num_frames(), buffered.num_frames());
    assert_eq!(optimized.num_atoms, buffered.num_atoms);
}

#[test]
fn test_bond_order_cylinder_counts() {
    assert_eq!(bond_cylinder_count(BondOrder::Single), 1);
    assert_eq!(bond_cylinder_count(BondOrder::Double), 2);
    assert_eq!(bond_cylinder_count(BondOrder::Triple), 3);
    assert_eq!(bond_cylinder_local_offsets(BondOrder::Double, 0.2).len(), 2);
}
