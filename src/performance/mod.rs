//! Performance settings, memory budgeting, and runtime diagnostics.

pub mod fps;
pub mod memory;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
pub use fps::{
    FrameStats, ProfilingPhase, ProfilingReport, ProfilingSession, FRAME_BUDGET_MS,
    PLAYBACK_TARGET_FPS, TARGET_FPS,
};

/// Grouped performance resources for UI systems (keeps system param count low).
#[derive(SystemParam)]
pub struct PerformanceUiState<'w> {
    pub frame_stats: Res<'w, FrameStats>,
    pub profiling: Res<'w, ProfilingSession>,
    pub diagnostics: Res<'w, PerformanceDiagnostics>,
}

/// Global performance toggles and limits.
#[derive(Resource, Clone, Debug)]
pub struct PerformanceSettings {
    /// CPU-side frustum culling for instanced atoms.
    pub frustum_culling_enabled: bool,
    /// Level-of-detail mesh selection.
    pub lod_enabled: bool,
    /// Use R-tree neighbor search for bond detection.
    pub spatial_bond_detection: bool,
    /// Atom count above which spatial bond detection is used.
    pub spatial_bond_threshold: usize,
    /// Maximum pick-proxy entities (selection disabled above this).
    pub max_pick_proxies: usize,
    /// GPU compute interpolation for timeline playback (falls back to CPU if unavailable).
    pub gpu_interpolation_enabled: bool,
}

impl Default for PerformanceSettings {
    fn default() -> Self {
        Self {
            frustum_culling_enabled: true,
            lod_enabled: true,
            spatial_bond_detection: true,
            spatial_bond_threshold: 500,
            max_pick_proxies: 50_000,
            gpu_interpolation_enabled: true,
        }
    }
}

/// Runtime load / memory diagnostics surfaced to the UI.
#[derive(Resource, Default, Debug, Clone)]
pub struct PerformanceDiagnostics {
    pub estimated_bytes: u64,
    pub memory_warning: Option<String>,
    pub selection_disabled: bool,
    pub selection_disabled_reason: Option<String>,
    pub last_bond_detection_ms: f32,
    pub culled_instance_count: usize,
    pub visible_instance_count: usize,
    pub current_lod: crate::rendering::lod::AtomLod,
    pub profiling_report: Option<ProfilingReport>,
}

pub fn register(app: &mut App) {
    app.init_resource::<PerformanceSettings>()
        .init_resource::<PerformanceDiagnostics>()
        .init_resource::<FrameStats>()
        .add_systems(Update, (fps::update_frame_stats, fps::run_profiling_validation));
    info!("Performance module registered");
}
