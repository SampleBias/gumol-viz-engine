//! End-to-end load pipeline tests for GRO, DCD, and mmCIF (UI-equivalent paths).

mod common;

use bevy::prelude::*;
use common::{dcd_fixture, example, fixture, require_path};
use gumol_viz_engine::core::atom::Element;
use gumol_viz_engine::io::load_topology;
use gumol_viz_engine::io::pdb::PDBParser;
use gumol_viz_engine::io::pdb_mmap;
use gumol_viz_engine::io::streaming;
use gumol_viz_engine::io::topology::validate_atom_count;
use gumol_viz_engine::systems::loading::{
    handle_load_file_events_sync, FileLoadErrorEvent, FileLoadedEvent, LoadFileEvent,
    SimulationData,
};
use std::path::PathBuf;

fn load_test_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<SimulationData>();
    app.add_event::<LoadFileEvent>();
    app.add_event::<FileLoadedEvent>();
    app.add_event::<FileLoadErrorEvent>();
    app.add_systems(Update, handle_load_file_events_sync);
    app
}

fn run_load_pipeline(path: PathBuf) -> SimulationData {
    let mut app = load_test_app();
    app.world_mut().send_event(LoadFileEvent { path });
    app.update();
    app.world().resource::<SimulationData>().clone()
}

#[test]
fn test_gro_load_pipeline() {
    let path = example("water.gro");
    if !require_path(&path) {
        return;
    }

    let sim = run_load_pipeline(path);
    assert!(sim.loaded);
    assert_eq!(sim.num_atoms(), 3);
    assert_eq!(sim.num_frames(), 1);
    assert!(sim.get_frame(0).unwrap().velocities.is_some());
}

#[test]
fn test_mmcif_load_pipeline() {
    let path = example("water.cif");
    if !require_path(&path) {
        return;
    }

    let sim = run_load_pipeline(path);
    assert!(sim.loaded);
    assert_eq!(sim.num_atoms(), 3);
    assert_eq!(sim.num_frames(), 1);
}

#[test]
fn test_pdb_mmap_matches_buffered() {
    let path = fixture("1CRN.pdb");
    if !require_path(&path) {
        return;
    }

    let buffered = PDBParser::parse_file_buffered(&path).expect("buffered");
    let mapped = pdb_mmap::parse_file_mmap(&path).expect("mmap");
    assert_eq!(buffered.0.num_atoms, mapped.0.num_atoms);
    assert_eq!(buffered.1.len(), mapped.1.len());
}

#[test]
fn test_pdb_optimized_load_pipeline() {
    let path = fixture("1CRN.pdb");
    if !require_path(&path) {
        return;
    }

    let sim = run_load_pipeline(path);
    assert!(sim.loaded);
    assert_eq!(sim.num_atoms(), 327);
    assert!(!sim.bond_data.is_empty());
    assert!(sim.atom_data.iter().any(|a| a.element == Element::S));
}

#[test]
fn test_dcd_load_pipeline_needs_topology() {
    let dir = std::env::temp_dir().join(format!("gumol_e2e_dcd_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let dcd_path = dir.join("mini.dcd");
    dcd_fixture::write_minimal_dcd(&dcd_path, 3, 2).unwrap();

    let sim = run_load_pipeline(dcd_path);
    assert!(sim.loaded);
    assert!(sim.needs_topology);
    assert_eq!(sim.num_atoms(), 3);
    assert_eq!(sim.num_frames(), 2);
    assert!(sim.atom_data.iter().all(|a| a.element == Element::Unknown));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_dcd_load_with_topology_via_load_file() {
    let gro_path = example("water.gro");
    if !require_path(&gro_path) {
        return;
    }

    let dir = std::env::temp_dir().join(format!("gumol_e2e_dcd_topo_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let dcd_path = dir.join("water.dcd");
    dcd_fixture::write_minimal_dcd(&dcd_path, 3, 2).unwrap();

    let (trajectory, frame_provider) = streaming::open_dcd(&dcd_path).expect("open dcd");
    let (atom_data, _bond_data) = load_topology(&gro_path).expect("load gro topology");
    validate_atom_count(atom_data.len(), trajectory.num_atoms).expect("atom count match");

    assert!(frame_provider.is_none());
    assert_eq!(trajectory.num_frames(), 2);
    assert_eq!(atom_data.len(), 3);
    assert_eq!(atom_data[0].element, Element::O);

    let _ = std::fs::remove_dir_all(&dir);
}
