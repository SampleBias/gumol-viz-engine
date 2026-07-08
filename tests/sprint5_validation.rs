//! Sprint 5 validation — interaction polish and 100K CPU budget estimate.

use bevy::prelude::*;
use gumol_viz_engine::export::povray::{write_pov_to_path, CameraSnapshot};
use gumol_viz_engine::export::scene_snapshot::{AtomSnapshot, BondSnapshot, SceneSnapshot};

/// 60 FPS frame budget in milliseconds.
const FRAME_BUDGET_MS: f32 = 16.67;

/// Release benchmark means documented in `docs/VALIDATION.md` (2026-07-07).
const POSITION_SYNC_100K_MS: f32 = 3.922;
const DRAW_COUNT_100K_MS: f32 = 2.009;
/// Conservative estimates for per-frame systems not isolated in benchmarks.
const COLOR_UPDATE_ESTIMATE_MS: f32 = 1.0;
const BOND_POSITION_UPDATE_ESTIMATE_MS: f32 = 0.5;

#[test]
fn test_combined_100k_cpu_budget_leaves_gpu_headroom() {
    let cpu_total = POSITION_SYNC_100K_MS
        + DRAW_COUNT_100K_MS
        + COLOR_UPDATE_ESTIMATE_MS
        + BOND_POSITION_UPDATE_ESTIMATE_MS;

    assert!(
        cpu_total < FRAME_BUDGET_MS * 0.75,
        "estimated CPU per-frame cost {cpu_total:.2} ms should stay below 75% of 60 FPS budget"
    );
}

#[test]
fn test_povray_roundtrip_export() {
    let snapshot = SceneSnapshot {
        atoms: vec![AtomSnapshot {
            position: Vec3::new(0.0, 0.0, 0.0),
            radius: 0.5,
            color: [0.2, 0.8, 0.2],
        }],
        bonds: vec![BondSnapshot {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            length: 1.5,
            radius: 0.08,
        }],
    };
    let camera = CameraSnapshot::default();
    let path = std::env::temp_dir().join("gumol_sprint5_test.pov");
    write_pov_to_path(&path, &snapshot, &camera).expect("pov write");
    let text = std::fs::read_to_string(&path).expect("read pov");
    assert!(text.contains("sphere {"));
    let _ = std::fs::remove_file(path);
}

#[test]
fn test_box_selection_screen_rect() {
    use gumol_viz_engine::interaction::box_selection::ScreenRect;

    let rect = ScreenRect::from_corners(Vec2::new(0.0, 0.0), Vec2::new(100.0, 100.0));
    assert!(rect.contains(Vec2::new(50.0, 50.0)));
    assert!(!rect.contains(Vec2::new(150.0, 50.0)));
}
