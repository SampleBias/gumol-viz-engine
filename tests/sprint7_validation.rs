//! Sprint 7 validation — interactive 100K GPU profiling infrastructure.

use gumol_viz_engine::performance::{
    ProfilingReport, FRAME_BUDGET_MS, PLAYBACK_TARGET_FPS, TARGET_FPS,
};
use gumol_viz_engine::utils::synthetic::{
    ensure_100k_fixtures, synthetic_atom_data, synthetic_trajectory,
    SYNTHETIC_100K_PLAYBACK_XYZ, SYNTHETIC_100K_XYZ,
};

#[test]
fn test_100k_synthetic_atom_data_count() {
    assert_eq!(synthetic_atom_data(100_000).len(), 100_000);
}

#[test]
fn test_100k_synthetic_trajectory_frames() {
    let trajectory = synthetic_trajectory(100_000, 10);
    assert_eq!(trajectory.num_atoms, 100_000);
    assert_eq!(trajectory.num_frames(), 10);
}

#[test]
fn test_generate_100k_fixtures_writes_files() {
    let dir = std::env::temp_dir().join("gumol_sprint7_fixtures");
    let _ = std::fs::remove_dir_all(&dir);

    // Write into temp paths via direct API (avoid polluting repo fixtures in parallel tests).
    let static_path = dir.join("synthetic_100k.xyz");
    let playback_path = dir.join("synthetic_100k_10f.xyz");
    std::fs::create_dir_all(&dir).expect("mkdir temp");
    gumol_viz_engine::utils::synthetic::write_synthetic_xyz(&static_path, 256, 1)
        .expect("write static");
    gumol_viz_engine::utils::synthetic::write_synthetic_xyz(&playback_path, 256, 3)
        .expect("write playback");
    assert!(static_path.exists());
    assert!(playback_path.exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_100k_fixture_constants() {
    assert!(SYNTHETIC_100K_XYZ.contains("100k"));
    assert!(SYNTHETIC_100K_PLAYBACK_XYZ.contains("10f"));
}

#[test]
fn test_profiling_report_static_pass_threshold() {
    let samples = vec![FRAME_BUDGET_MS * 0.9; 300];
    let report = ProfilingReport::evaluate(
        100_000,
        1,
        3,
        false,
        120,
        300,
        &samples,
        TARGET_FPS,
    );
    assert!(report.passed, "expected pass at ~{TARGET_FPS} FPS target");
}

#[test]
fn test_profiling_report_playback_pass_threshold() {
    let frame_ms = 1000.0 / PLAYBACK_TARGET_FPS;
    let samples = vec![frame_ms * 0.95; 300];
    let report = ProfilingReport::evaluate(
        100_000,
        10,
        3,
        true,
        120,
        300,
        &samples,
        PLAYBACK_TARGET_FPS,
    );
    assert!(report.passed, "expected pass at ~{PLAYBACK_TARGET_FPS} FPS playback target");
}

#[test]
fn test_ensure_100k_fixtures_writes_both_paths_in_cwd() {
    let dir = std::env::temp_dir().join(format!(
        "gumol_sprint7_ensure_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("tests/fixtures")).expect("mkdir fixtures");

    let prev = std::env::current_dir().expect("cwd");
    std::env::set_current_dir(&dir).expect("chdir temp");
    let result = ensure_100k_fixtures();
    std::env::set_current_dir(&prev).expect("restore cwd");

    result.expect("ensure fixtures");
    assert!(dir.join(SYNTHETIC_100K_XYZ).exists());
    assert!(dir.join(SYNTHETIC_100K_PLAYBACK_XYZ).exists());
    let _ = std::fs::remove_dir_all(&dir);
}
