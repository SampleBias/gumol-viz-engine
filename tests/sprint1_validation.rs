//! Sprint 1 automated validation — load pipeline, bonds, interpolation, export.

mod common;

use bevy::prelude::*;
use common::{fixture, require_path};
use gumol_viz_engine::core::atom::Element;
use gumol_viz_engine::core::trajectory::{interpolate_frames, FrameData, TimelineState};
use gumol_viz_engine::core::visualization::VisualizationConfig;
use gumol_viz_engine::export::gltf_export::write_gltf_to_path;
use gumol_viz_engine::export::obj::write_obj_to_path;
use gumol_viz_engine::export::scene_snapshot::{AtomSnapshot, SceneSnapshot};
use gumol_viz_engine::io::pdb::PDBParser;
use gumol_viz_engine::performance::PerformanceSettings;
use gumol_viz_engine::rendering::gpu_interpolation::{
    interpolate_dense_positions, DenseAtomLayout,
};
use gumol_viz_engine::rendering::instanced::{
    estimate_instanced_draw_calls, spawn_atoms_instanced_internal, MAX_INSTANCED_DRAW_CALLS,
};
use gumol_viz_engine::rendering::mesh_pool::AtomMeshPool;
use gumol_viz_engine::systems::bonds::{resolve_bond_list, BondDetectionConfig};
use gumol_viz_engine::systems::loading::{
    handle_load_file_events_sync, FileLoadErrorEvent, FileLoadedEvent, LoadFileEvent,
    SimulationData,
};
use gumol_viz_engine::utils::spatial_index::AtomSpatialIndex;
use std::collections::HashMap;
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

#[test]
fn test_1crn_load_pipeline() {
    let path = fixture("1CRN.pdb");
    if !require_path(&path) {
        return;
    }

    let mut app = load_test_app();
    app.world_mut()
        .send_event(LoadFileEvent { path: path.clone() });
    app.update();

    let sim = app.world().resource::<SimulationData>();
    assert!(sim.loaded);
    assert_eq!(sim.num_atoms(), 327);
    assert_eq!(sim.num_frames(), 1);
    assert_eq!(sim.atom_data.len(), 327);
}

#[test]
fn test_1crn_bond_data_and_spatial_detection() {
    let path = fixture("1CRN.pdb");
    if !require_path(&path) {
        return;
    }

    let mut app = load_test_app();
    app.world_mut()
        .send_event(LoadFileEvent { path: path.clone() });
    app.update();

    let sim = app.world().resource::<SimulationData>();
    // 1CRN PDB includes sparse CONECT records (disulfide / termini), not full topology.
    assert!(
        !sim.bond_data.is_empty(),
        "1CRN should load CONECT bond records from PDB"
    );

    let frame = sim.get_frame(0).expect("first frame");
    let mut positions = HashMap::new();
    for atom in &sim.atom_data {
        if let Some(pos) = frame.get_position(atom.id) {
            positions.insert(atom.id, pos);
        }
    }

    // Water has no CONECT — spatial distance detection should infer O–H bonds.
    let water_path = fixture("water.xyz");
    let mut water_app = load_test_app();
    water_app.world_mut().send_event(LoadFileEvent {
        path: water_path.clone(),
    });
    water_app.update();
    let water = water_app.world().resource::<SimulationData>();
    assert!(water.bond_data.is_empty());

    let water_frame = water.get_frame(0).unwrap();
    let mut water_positions = HashMap::new();
    for atom in &water.atom_data {
        if let Some(pos) = water_frame.get_position(atom.id) {
            water_positions.insert(atom.id, pos);
        }
    }

    let spatial = AtomSpatialIndex::build(&water.atom_data, &water_positions);
    let config = BondDetectionConfig::default();
    let perf = PerformanceSettings::default();
    let water_bonds = resolve_bond_list(water, &water_positions, &config, &perf, Some(&spatial));
    assert!(
        water_bonds.len() >= 2,
        "water should infer at least two O–H bonds, got {}",
        water_bonds.len()
    );

    let _ = (sim, positions);
}

#[test]
fn test_multi_frame_timeline_interpolation() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../demo_trajectory.xyz");
    if !require_path(&path) {
        return;
    }

    let mut app = load_test_app();
    app.world_mut()
        .send_event(LoadFileEvent { path: path.clone() });
    app.update();

    let sim = app.world().resource::<SimulationData>();
    assert!(sim.num_frames() >= 2);

    let frame_a = sim.get_frame(0).expect("frame 0");
    let frame_b = sim.get_frame(1).expect("frame 1");
    let layout = DenseAtomLayout::build(&sim.atom_data);

    let timeline = TimelineState {
        interpolate: true,
        interpolation_factor: 0.4,
        current_frame: 0,
        ..Default::default()
    };

    let pos_a: Vec<Vec3> = layout
        .dense_atom_ids
        .iter()
        .map(|&id| frame_a.get_position(id).unwrap())
        .collect();
    let pos_b: Vec<Vec3> = layout
        .dense_atom_ids
        .iter()
        .map(|&id| frame_b.get_position(id).unwrap())
        .collect();

    let dense = interpolate_dense_positions(&pos_a, &pos_b, timeline.interpolation_factor);
    let expected = interpolate_frames(&frame_a, &frame_b, timeline.interpolation_factor);

    for (i, &atom_id) in layout.dense_atom_ids.iter().enumerate() {
        let exp = expected.get_position(atom_id).unwrap();
        assert!(
            dense[i].distance(exp) < 1e-5,
            "atom {atom_id}: dense={:?} expected={:?}",
            dense[i],
            exp
        );
    }
}

#[test]
fn test_100k_instanced_draw_calls_within_budget() {
    let count = 100_000usize;
    let atoms: Vec<_> = (0..count)
        .map(|i| {
            gumol_viz_engine::core::atom::AtomData::new(
                i as u32,
                if i % 3 == 0 {
                    Element::C
                } else if i % 3 == 1 {
                    Element::H
                } else {
                    Element::O
                },
                (i / 10) as u32,
                "UNK".into(),
                "A".into(),
                format!("A{i}"),
            )
        })
        .collect();

    let draw_calls = estimate_instanced_draw_calls(&atoms);
    assert!(draw_calls <= MAX_INSTANCED_DRAW_CALLS);
    assert!(draw_calls <= 118, "one batch per element present");
}

#[test]
fn test_100k_position_dense_sync_smoke() {
    let count = 100_000usize;
    let atoms: Vec<_> = (0..count)
        .map(|i| {
            gumol_viz_engine::core::atom::AtomData::new(
                i as u32,
                Element::C,
                0,
                "UNK".into(),
                "A".into(),
                format!("C{i}"),
            )
        })
        .collect();

    let mut trajectory =
        gumol_viz_engine::core::trajectory::Trajectory::new(PathBuf::from("bench.xyz"), count, 1.0);
    for f in 0..5 {
        let mut frame = FrameData::new(f, f as f32);
        for a in &atoms {
            frame.set_position(a.id, Vec3::new(a.id as f32 * 0.01, 0.0, f as f32));
        }
        trajectory.add_frame(frame);
    }

    let sim = SimulationData::new(trajectory, atoms);
    let positions = sim.frame_positions_dense(3).expect("dense positions");
    assert_eq!(positions.len(), count);
}

/// Release-only timing gate (see `benches/baseline.json` and `docs/VALIDATION.md`).
#[test]
#[cfg(not(debug_assertions))]
fn test_100k_position_dense_sync_under_60fps_budget() {
    const FPS_60_BUDGET_MS: f32 = 16.67;
    // Criterion baseline `frame_position_dense/100000` ≈ 3.27 ms; allow 2.5× headroom.
    const MAX_POSITION_SYNC_MS: f32 = 8.0;

    let count = 100_000usize;
    let atoms: Vec<_> = (0..count)
        .map(|i| {
            gumol_viz_engine::core::atom::AtomData::new(
                i as u32,
                Element::C,
                0,
                "UNK".into(),
                "A".into(),
                format!("C{i}"),
            )
        })
        .collect();

    let mut trajectory =
        gumol_viz_engine::core::trajectory::Trajectory::new(PathBuf::from("bench.xyz"), count, 1.0);
    for f in 0..5 {
        let mut frame = FrameData::new(f, f as f32);
        for a in &atoms {
            frame.set_position(a.id, Vec3::new(a.id as f32 * 0.01, 0.0, f as f32));
        }
        trajectory.add_frame(frame);
    }

    let sim = SimulationData::new(trajectory, atoms);

    let start = std::time::Instant::now();
    for _ in 0..10 {
        let _ = sim.frame_positions_dense(3);
    }
    let elapsed_ms = start.elapsed().as_secs_f32() * 1000.0 / 10.0;

    assert!(
        elapsed_ms < MAX_POSITION_SYNC_MS,
        "100K dense position sync {:.2} ms exceeds {:.2} ms budget (60 FPS frame = {:.2} ms)",
        elapsed_ms,
        MAX_POSITION_SYNC_MS,
        FPS_60_BUDGET_MS
    );
}

#[test]
fn test_export_obj_and_gltf_from_snapshot() {
    let snapshot = SceneSnapshot {
        atoms: vec![
            AtomSnapshot {
                position: Vec3::ZERO,
                radius: 0.5,
                color: [1.0, 0.0, 0.0],
            },
            AtomSnapshot {
                position: Vec3::new(1.0, 0.0, 0.0),
                radius: 0.3,
                color: [0.9, 0.9, 0.9],
            },
        ],
        bonds: vec![],
    };

    let dir = std::env::temp_dir().join("gumol_sprint1_validation");
    std::fs::create_dir_all(&dir).expect("temp dir");

    let obj_path = dir.join("test_export.obj");
    let gltf_path = dir.join("test_export.gltf");

    write_obj_to_path(&obj_path, &snapshot).expect("OBJ write");
    write_gltf_to_path(&gltf_path, &snapshot).expect("glTF write");

    let obj_text = std::fs::read_to_string(&obj_path).expect("read obj");
    assert!(obj_text.contains("# Gumol Viz Engine - OBJ Export"));
    assert!(obj_text.contains("v "));
    assert!(obj_text.contains("f "));

    let gltf_text = std::fs::read_to_string(&gltf_path).expect("read gltf");
    assert!(gltf_text.contains("\"asset\""));
    assert!(gltf_text.contains("\"meshes\""));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_instanced_spawn_1crn_scale() {
    let path = fixture("1CRN.pdb");
    if !require_path(&path) {
        return;
    }

    let (trajectory, atom_data, _bonds) =
        PDBParser::parse_file_with_atoms(&path).expect("parse 1CRN");
    let frame = trajectory.get_frame(0).unwrap().clone();
    let viz = VisualizationConfig::default();

    let mut app = App::new();
    app.init_resource::<Assets<Mesh>>();
    app.world_mut()
        .resource_scope(|world, mut meshes: Mut<Assets<Mesh>>| {
            let mut mesh_pool = AtomMeshPool::default();
            let mut commands = world.commands();
            let (entities, ids_by_element) = spawn_atoms_instanced_internal(
                &mut commands,
                &mut meshes,
                &mut mesh_pool,
                &frame,
                &atom_data,
                &viz,
            );
            let instance_count: usize = ids_by_element.values().map(|v| v.len()).sum();
            assert_eq!(instance_count, atom_data.len());
            assert_eq!(entities.len(), estimate_instanced_draw_calls(&atom_data));
            assert!(entities.len() <= MAX_INSTANCED_DRAW_CALLS);
        });
}
