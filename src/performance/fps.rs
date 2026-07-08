//! Runtime FPS tracking and interactive 100K validation profiling.

use crate::core::trajectory::TimelineState;
use crate::rendering::instanced::{InstancedAtomEntities, InstancedAtomsSpawnedEvent};
use crate::systems::loading::{AsyncLoadState, ProfileCliArgs, SimulationData};
use bevy::prelude::*;
use std::fs;
use std::path::Path;

/// Target interactive frame rate for large static scenes.
pub const TARGET_FPS: f32 = 60.0;

/// Minimum acceptable FPS during trajectory playback at 100K atoms.
pub const PLAYBACK_TARGET_FPS: f32 = 30.0;

/// 60 FPS frame budget in milliseconds.
pub const FRAME_BUDGET_MS: f32 = 1000.0 / TARGET_FPS;

/// Rolling frame-time statistics updated every frame.
#[derive(Resource, Debug, Clone)]
pub struct FrameStats {
    pub current_fps: f32,
    pub avg_fps: f32,
    pub min_fps: f32,
    pub frame_time_ms: f32,
    pub avg_frame_time_ms: f32,
    pub max_frame_time_ms: f32,
    pub samples: u64,
}

impl Default for FrameStats {
    fn default() -> Self {
        Self {
            current_fps: 0.0,
            avg_fps: 0.0,
            min_fps: f32::MAX,
            frame_time_ms: 0.0,
            avg_frame_time_ms: 0.0,
            max_frame_time_ms: 0.0,
            samples: 0,
        }
    }
}

/// Phase of an automated profiling run (`--profile`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfilingPhase {
    WaitingForLoad,
    Warmup,
    Sampling,
    Complete,
}

/// Automated profiling session state.
#[derive(Resource, Debug, Clone)]
pub struct ProfilingSession {
    pub active: bool,
    pub phase: ProfilingPhase,
    pub warmup_frames: u32,
    pub sample_frames: u32,
    pub playback: bool,
    pub exit_on_complete: bool,
    pub min_fps_target: f32,
    pub output_path: Option<std::path::PathBuf>,
    pub frames_in_phase: u32,
    pub sample_times_ms: Vec<f32>,
    pub report: Option<ProfilingReport>,
}

impl ProfilingSession {
    pub fn from_cli(args: &ProfileCliArgs) -> Self {
        Self {
            active: args.enabled,
            phase: if args.enabled {
                ProfilingPhase::WaitingForLoad
            } else {
                ProfilingPhase::Complete
            },
            warmup_frames: args.warmup_frames,
            sample_frames: args.sample_frames,
            playback: args.playback,
            exit_on_complete: args.exit_on_complete,
            min_fps_target: if args.playback {
                args.min_fps_playback
            } else {
                args.min_fps_static
            },
            output_path: args.output_path.clone(),
            frames_in_phase: 0,
            sample_times_ms: Vec::new(),
            report: None,
        }
    }
}

/// JSON-serializable profiling result.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProfilingReport {
    pub atom_count: usize,
    pub frame_count: usize,
    pub playback: bool,
    pub draw_calls: usize,
    pub warmup_frames: u32,
    pub sample_frames: u32,
    pub avg_fps: f32,
    pub min_fps: f32,
    pub p95_frame_time_ms: f32,
    pub max_frame_time_ms: f32,
    pub target_fps: f32,
    pub passed: bool,
}

impl ProfilingReport {
    pub fn evaluate(
        atom_count: usize,
        frame_count: usize,
        draw_calls: usize,
        playback: bool,
        warmup_frames: u32,
        sample_frames: u32,
        sample_times_ms: &[f32],
        min_fps_target: f32,
    ) -> Self {
        let avg_frame_ms = if sample_times_ms.is_empty() {
            f32::MAX
        } else {
            sample_times_ms.iter().sum::<f32>() / sample_times_ms.len() as f32
        };
        let max_frame_ms = sample_times_ms.iter().copied().fold(0.0_f32, f32::max);
        let min_fps = if max_frame_ms > 0.0 {
            1000.0 / max_frame_ms
        } else {
            0.0
        };
        let avg_fps = if avg_frame_ms > 0.0 {
            1000.0 / avg_frame_ms
        } else {
            0.0
        };
        let p95_frame_time_ms = percentile(sample_times_ms, 0.95);
        let frame_budget_ms = 1000.0 / min_fps_target;
        let passed = avg_fps >= min_fps_target && p95_frame_time_ms <= frame_budget_ms * 1.5;

        Self {
            atom_count,
            frame_count,
            playback,
            draw_calls,
            warmup_frames,
            sample_frames,
            avg_fps,
            min_fps,
            p95_frame_time_ms,
            max_frame_time_ms: max_frame_ms,
            target_fps: min_fps_target,
            passed,
        }
    }
}

fn percentile(values: &[f32], pct: f32) -> f32 {
    if values.is_empty() {
        return f32::MAX;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let idx = ((sorted.len() - 1) as f32 * pct).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

/// Update rolling FPS statistics from Bevy's frame delta.
pub fn update_frame_stats(time: Res<Time>, mut stats: ResMut<FrameStats>) {
    let dt = time.delta_seconds();
    if dt <= 0.0 {
        return;
    }

    let frame_ms = dt * 1000.0;
    let fps = 1.0 / dt;
    stats.frame_time_ms = frame_ms;
    stats.current_fps = fps;
    stats.samples += 1;

    let n = stats.samples as f32;
    stats.avg_frame_time_ms += (frame_ms - stats.avg_frame_time_ms) / n;
    stats.avg_fps = if stats.avg_frame_time_ms > 0.0 {
        1000.0 / stats.avg_frame_time_ms
    } else {
        0.0
    };
    stats.max_frame_time_ms = stats.max_frame_time_ms.max(frame_ms);
    stats.min_fps = stats.min_fps.min(fps);
}

/// Drive automated profiling once atoms are loaded and rendered.
pub fn run_profiling_validation(
    time: Res<Time>,
    mut session: ResMut<ProfilingSession>,
    mut timeline: ResMut<TimelineState>,
    sim_data: Res<SimulationData>,
    async_load: Res<AsyncLoadState>,
    instanced: Res<InstancedAtomEntities>,
    mut spawned_events: EventReader<InstancedAtomsSpawnedEvent>,
    mut frame_stats: ResMut<FrameStats>,
    mut exit: EventWriter<AppExit>,
    mut diagnostics: ResMut<crate::performance::PerformanceDiagnostics>,
) {
    if !session.active || session.phase == ProfilingPhase::Complete {
        return;
    }

    if spawned_events.read().next().is_some() && session.phase == ProfilingPhase::WaitingForLoad {
        session.phase = ProfilingPhase::Warmup;
        session.frames_in_phase = 0;
        if session.playback && sim_data.num_frames() > 1 {
            timeline.is_playing = true;
            timeline.interpolate = true;
            timeline.loop_playback = true;
        }
        info!(
            "Profiling: atoms spawned ({}), starting {}-frame warmup",
            instanced.total_atoms, session.warmup_frames
        );
    }

    if session.phase == ProfilingPhase::WaitingForLoad
        && (!sim_data.loaded
            || async_load.in_progress
            || instanced.total_atoms == 0
            || instanced.total_atoms != sim_data.num_atoms())
    {
        return;
    }

    let dt_ms = time.delta_seconds() * 1000.0;

    match session.phase {
        ProfilingPhase::WaitingForLoad => {}
        ProfilingPhase::Warmup => {
            session.frames_in_phase += 1;
            if session.frames_in_phase >= session.warmup_frames {
                session.phase = ProfilingPhase::Sampling;
                session.frames_in_phase = 0;
                session.sample_times_ms.clear();
                frame_stats.min_fps = f32::MAX;
                frame_stats.max_frame_time_ms = 0.0;
                info!(
                    "Profiling: warmup complete, sampling {} frames",
                    session.sample_frames
                );
            }
        }
        ProfilingPhase::Sampling => {
            session.sample_times_ms.push(dt_ms);
            session.frames_in_phase += 1;
            if session.frames_in_phase >= session.sample_frames {
                let report = ProfilingReport::evaluate(
                    sim_data.num_atoms(),
                    sim_data.num_frames(),
                    instanced.entities.len(),
                    session.playback,
                    session.warmup_frames,
                    session.sample_frames,
                    &session.sample_times_ms,
                    session.min_fps_target,
                );
                log_profiling_report(&report);
                if let Some(ref path) = session.output_path {
                    if let Err(err) = write_report_json(path, &report) {
                        warn!(
                            "Failed to write profiling report to {}: {err}",
                            path.display()
                        );
                    }
                }
                diagnostics.profiling_report = Some(report.clone());
                session.report = Some(report);
                session.phase = ProfilingPhase::Complete;
                session.active = false;

                if session.exit_on_complete {
                    let code = if session.report.as_ref().is_some_and(|r| r.passed) {
                        AppExit::Success
                    } else {
                        AppExit::error()
                    };
                    exit.send(code);
                }
            }
        }
        ProfilingPhase::Complete => {}
    }
}

fn log_profiling_report(report: &ProfilingReport) {
    let status = if report.passed { "PASS" } else { "FAIL" };
    info!(
        "Profiling {status}: {} atoms, {} frames, draw_calls={}, avg_fps={:.1}, min_fps={:.1}, p95={:.2} ms, target>={:.0} FPS",
        report.atom_count,
        report.frame_count,
        report.draw_calls,
        report.avg_fps,
        report.min_fps,
        report.p95_frame_time_ms,
        report.target_fps,
    );
    info!(
        "Profiling JSON: {}",
        serde_json::to_string(report).unwrap_or_default()
    );
}

fn write_report_json(path: &Path, report: &ProfilingReport) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
    }
    let json = serde_json::to_string_pretty(report).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profiling_report_passes_at_60fps() {
        let samples = vec![16.0; 300];
        let report =
            ProfilingReport::evaluate(100_000, 1, 3, false, 120, 300, &samples, TARGET_FPS);
        assert!(report.passed);
        assert!((report.avg_fps - 62.5).abs() < 1.0);
    }

    #[test]
    fn profiling_report_fails_below_target() {
        let samples = vec![40.0; 300];
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
        assert!(!report.passed);
    }

    #[test]
    fn percentile_p95_is_stable() {
        let values: Vec<f32> = (1..=100).map(|v| v as f32).collect();
        assert!((percentile(&values, 0.95) - 95.0).abs() < 1.0);
    }
}
