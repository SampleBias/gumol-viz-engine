//! Trajectory video export via FFmpeg subprocess.
//!
//! Captures one screenshot per timeline frame, then encodes with `ffmpeg` on PATH.

use crate::core::trajectory::TimelineState;
use crate::systems::frame_cache::TimelineFrames;
use crate::ui::notifications::UiNotifications;
use bevy::prelude::*;
use bevy::render::view::window::screenshot::ScreenshotManager;
use bevy::window::PrimaryWindow;
use crossbeam_channel::{Receiver, Sender};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// Default frames per second for exported video.
pub const DEFAULT_VIDEO_FPS: u32 = 30;

/// Frames to wait after seeking before capturing (GPU + layout settle).
const SETTLE_FRAMES: u8 = 2;

/// User-facing export parameters.
#[derive(Debug, Clone)]
pub struct VideoExportSettings {
    pub fps: u32,
    pub start_frame: usize,
    pub end_frame: usize,
}

impl Default for VideoExportSettings {
    fn default() -> Self {
        Self {
            fps: DEFAULT_VIDEO_FPS,
            start_frame: 0,
            end_frame: 0,
        }
    }
}

/// Request encoding a trajectory range to MP4/WebM/GIF (extension selects codec).
#[derive(Event, Debug)]
pub struct RequestVideoExportEvent {
    pub output_path: PathBuf,
    pub settings: VideoExportSettings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoExportStatus {
    Idle,
    Capturing,
    Encoding,
}

/// Global video export progress (read by UI).
#[derive(Resource)]
pub struct VideoExportState {
    pub status: VideoExportStatus,
    /// Capture progress in `[0.0, 1.0]` while capturing; `-1.0` while encoding.
    pub progress: f32,
    pub message: Option<String>,
    internal: Option<VideoExportSession>,
}

impl Default for VideoExportState {
    fn default() -> Self {
        Self {
            status: VideoExportStatus::Idle,
            progress: 0.0,
            message: None,
            internal: None,
        }
    }
}

enum CapturePhase {
    Settling { ticks: u8 },
    WaitingScreenshot,
}

struct VideoExportSession {
    output_path: PathBuf,
    temp_dir: PathBuf,
    fps: u32,
    start_frame: usize,
    capture_frame: usize,
    end_frame: usize,
    phase: CapturePhase,
    frame_done_tx: Sender<usize>,
    frame_done_rx: Receiver<usize>,
    encode_done_rx: Option<Receiver<Result<PathBuf, String>>>,
    restore_playing: bool,
    restore_interpolate: bool,
}

/// Returns true when `ffmpeg` is available on PATH.
pub fn ffmpeg_available() -> bool {
    Command::new("ffmpeg")
        .arg("-version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Build FFmpeg CLI arguments for a numbered PNG sequence.
pub fn build_ffmpeg_args(
    output: &Path,
    frames_dir: &Path,
    fps: u32,
    start_number: usize,
) -> Result<Vec<String>, String> {
    if fps == 0 {
        return Err("FPS must be at least 1".into());
    }

    let input_pattern = frames_dir.join("frame_%05d.png");
    let ext = output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("mp4")
        .to_ascii_lowercase();

    let mut args = vec![
        "ffmpeg".to_string(),
        "-y".to_string(),
        "-framerate".to_string(),
        fps.to_string(),
        "-start_number".to_string(),
        start_number.to_string(),
        "-i".to_string(),
        input_pattern.to_string_lossy().into_owned(),
    ];

    match ext.as_str() {
        "webm" => {
            args.extend([
                "-c:v".to_string(),
                "libvpx-vp9".to_string(),
                "-pix_fmt".to_string(),
                "yuv420p".to_string(),
            ]);
        }
        "gif" => {
            args.extend([
                "-vf".to_string(),
                format!("fps={fps}"),
                "-loop".to_string(),
                "0".to_string(),
            ]);
        }
        "mp4" | "m4v" | "mov" => {
            args.extend([
                "-c:v".to_string(),
                "libx264".to_string(),
                "-pix_fmt".to_string(),
                "yuv420p".to_string(),
            ]);
        }
        other => {
            return Err(format!(
                "Unsupported video extension '{other}' (use mp4, webm, or gif)"
            ));
        }
    }

    args.push(output.to_string_lossy().into_owned());
    Ok(args)
}

/// Run FFmpeg with pre-built args (first element is the binary name).
pub fn run_ffmpeg(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err("Empty FFmpeg argument list".into());
    }

    let status = Command::new(&args[0])
        .args(&args[1..])
        .status()
        .map_err(|e| format!("Failed to run ffmpeg: {e}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("ffmpeg exited with status {status}"))
    }
}

fn temp_frames_dir() -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("gumol_video_{stamp}"))
}

fn cleanup_temp_dir(path: &Path) {
    if let Err(err) = std::fs::remove_dir_all(path) {
        warn!(
            "Failed to remove temp video frames at {}: {err}",
            path.display()
        );
    }
}

fn finish_session(
    state: &mut VideoExportState,
    timeline: &mut TimelineState,
    notifications: &mut UiNotifications,
    success: bool,
    message: impl Into<String>,
) {
    if let Some(session) = state.internal.take() {
        cleanup_temp_dir(&session.temp_dir);
        timeline.interpolate = session.restore_interpolate;
        if session.restore_playing {
            timeline.play();
        } else {
            timeline.pause();
        }
    }

    state.status = VideoExportStatus::Idle;
    state.progress = 0.0;
    state.message = None;

    let text = message.into();
    if success {
        info!("{text}");
        notifications.show(text, 240);
    } else {
        error!("{text}");
        notifications.show(format!("Video export failed: {text}"), 300);
    }
}

fn start_encode_thread(
    output_path: PathBuf,
    temp_dir: PathBuf,
    fps: u32,
    start_number: usize,
) -> Receiver<Result<PathBuf, String>> {
    let (tx, rx) = crossbeam_channel::bounded(1);
    std::thread::spawn(move || {
        let result = (|| {
            let args = build_ffmpeg_args(&output_path, &temp_dir, fps, start_number)?;
            run_ffmpeg(&args)?;
            Ok(output_path)
        })();
        let _ = tx.send(result);
    });
    rx
}

/// Handle new export requests.
pub fn handle_video_export_requests(
    mut state: ResMut<VideoExportState>,
    mut timeline: ResMut<TimelineState>,
    sim_data: Res<crate::systems::loading::SimulationData>,
    mut notifications: ResMut<UiNotifications>,
    mut requests: EventReader<RequestVideoExportEvent>,
) {
    if state.status != VideoExportStatus::Idle {
        return;
    }

    for event in requests.read() {
        if !sim_data.loaded || sim_data.num_frames() == 0 {
            notifications.show("Load a trajectory before recording video", 180);
            continue;
        }

        let total_frames = sim_data.num_frames();
        let settings = event.settings.clone();
        let output_path = event.output_path.clone();

        let end_frame = if settings.end_frame == 0 {
            total_frames.saturating_sub(1)
        } else {
            settings.end_frame.min(total_frames.saturating_sub(1))
        };
        let start_frame = settings.start_frame.min(end_frame);

        if !ffmpeg_available() {
            notifications.show(
                "FFmpeg not found — install ffmpeg and ensure it is on PATH",
                300,
            );
            continue;
        }

        let temp_dir = temp_frames_dir();
        if let Err(err) = std::fs::create_dir_all(&temp_dir) {
            notifications.show(format!("Cannot create temp directory: {err}"), 240);
            continue;
        }

        let (frame_done_tx, frame_done_rx) = crossbeam_channel::unbounded();
        let restore_playing = timeline.is_playing;
        let restore_interpolate = timeline.interpolate;

        timeline.pause();
        timeline.interpolate = false;
        timeline.interpolation_factor = 0.0;
        timeline.goto_frame(start_frame);

        let frame_count = end_frame.saturating_sub(start_frame) + 1;
        if frame_count > 1000 {
            warn!("Video export spans {frame_count} frames; capture may take a while");
        }

        state.status = VideoExportStatus::Capturing;
        state.progress = 0.0;
        state.message = Some(format!(
            "Recording frame {} / {}…",
            start_frame + 1,
            end_frame + 1
        ));
        state.internal = Some(VideoExportSession {
            output_path,
            temp_dir,
            fps: settings.fps.max(1),
            start_frame,
            capture_frame: start_frame,
            end_frame,
            phase: CapturePhase::Settling {
                ticks: SETTLE_FRAMES,
            },
            frame_done_tx,
            frame_done_rx,
            encode_done_rx: None,
            restore_playing,
            restore_interpolate,
        });

        info!(
            "Video export started: frames {}..={} @ {} fps",
            start_frame,
            end_frame,
            settings.fps.max(1)
        );
    }
}

/// Advance capture / encoding state machine.
pub fn video_export_step(
    mut state: ResMut<VideoExportState>,
    mut timeline: ResMut<TimelineState>,
    timeline_frames: Res<TimelineFrames>,
    mut screenshot_manager: ResMut<ScreenshotManager>,
    mut notifications: ResMut<UiNotifications>,
    primary_window: Query<Entity, With<PrimaryWindow>>,
) {
    if state.internal.is_none() {
        return;
    }

    if let Some(encode_rx) = state
        .internal
        .as_ref()
        .and_then(|session| session.encode_done_rx.as_ref())
    {
        match encode_rx.try_recv() {
            Ok(Ok(path)) => {
                let msg = format!("Video saved to {}", path.display());
                finish_session(&mut state, &mut timeline, &mut notifications, true, msg);
            }
            Ok(Err(err)) => {
                finish_session(&mut state, &mut timeline, &mut notifications, false, err);
            }
            Err(crossbeam_channel::TryRecvError::Empty) => {}
            Err(crossbeam_channel::TryRecvError::Disconnected) => {
                finish_session(
                    &mut state,
                    &mut timeline,
                    &mut notifications,
                    false,
                    "Encoding thread disconnected",
                );
            }
        }
        return;
    }

    let waiting_screenshot = state
        .internal
        .as_ref()
        .is_some_and(|session| matches!(session.phase, CapturePhase::WaitingScreenshot));

    if waiting_screenshot {
        let recv_result = state.internal.as_ref().unwrap().frame_done_rx.try_recv();
        match recv_result {
            Ok(done_frame) => {
                let (end_frame, start_frame, fps, output_path, temp_dir) = {
                    let session = state.internal.as_ref().unwrap();
                    (
                        session.end_frame,
                        session.start_frame,
                        session.fps,
                        session.output_path.clone(),
                        session.temp_dir.clone(),
                    )
                };
                let frame_span = end_frame.saturating_sub(start_frame) + 1;

                if done_frame >= end_frame {
                    let encode_rx = start_encode_thread(output_path, temp_dir, fps, start_frame);
                    state.status = VideoExportStatus::Encoding;
                    state.progress = -1.0;
                    state.message = Some("Encoding video with FFmpeg…".into());
                    state.internal.as_mut().unwrap().encode_done_rx = Some(encode_rx);
                } else {
                    let next = done_frame + 1;
                    {
                        let session = state.internal.as_mut().unwrap();
                        session.capture_frame = next;
                        session.phase = CapturePhase::Settling {
                            ticks: SETTLE_FRAMES,
                        };
                    }
                    timeline.goto_frame(next);
                    state.progress = (done_frame + 1 - start_frame) as f32 / frame_span as f32;
                    state.message =
                        Some(format!("Recording frame {} / {}…", next + 1, end_frame + 1));
                }
            }
            Err(crossbeam_channel::TryRecvError::Empty) => {}
            Err(crossbeam_channel::TryRecvError::Disconnected) => {
                finish_session(
                    &mut state,
                    &mut timeline,
                    &mut notifications,
                    false,
                    "Screenshot callback channel disconnected",
                );
            }
        }
        return;
    }

    let settling_ticks = state.internal.as_ref().and_then(|session| {
        if let CapturePhase::Settling { ticks } = session.phase {
            Some(ticks)
        } else {
            None
        }
    });

    let Some(ticks) = settling_ticks else {
        return;
    };

    if timeline_frames.loading {
        return;
    }

    if ticks > 0 {
        state.internal.as_mut().unwrap().phase = CapturePhase::Settling { ticks: ticks - 1 };
        return;
    }

    let Ok(window_entity) = primary_window.get_single() else {
        finish_session(
            &mut state,
            &mut timeline,
            &mut notifications,
            false,
            "No primary window for screenshot capture",
        );
        return;
    };

    let (frame_index, frame_path, tx, start_frame, end_frame) = {
        let session = state.internal.as_ref().unwrap();
        (
            session.capture_frame,
            session
                .temp_dir
                .join(format!("frame_{:05}.png", session.capture_frame)),
            session.frame_done_tx.clone(),
            session.start_frame,
            session.end_frame,
        )
    };

    if screenshot_manager
        .take_screenshot(window_entity, move |img| {
            match img.try_into_dynamic() {
                Ok(dyn_img) => {
                    let rgb = dyn_img.to_rgb8();
                    if let Err(err) = rgb.save(&frame_path) {
                        error!("Failed to save video frame {}: {err}", frame_path.display());
                    }
                }
                Err(err) => {
                    error!("Screenshot format error for video frame: {err}");
                }
            }
            let _ = tx.send(frame_index);
        })
        .is_err()
    {
        finish_session(
            &mut state,
            &mut timeline,
            &mut notifications,
            false,
            "Screenshot already in progress",
        );
        return;
    }

    state.internal.as_mut().unwrap().phase = CapturePhase::WaitingScreenshot;
    let range = end_frame.saturating_sub(start_frame) + 1;
    state.progress = (frame_index + 1 - start_frame) as f32 / range as f32;
    state.message = Some(format!(
        "Recording frame {} / {}…",
        frame_index + 1,
        end_frame + 1
    ));
}

/// Cancel export if a new trajectory is loaded.
pub fn cancel_video_export_on_load(
    mut state: ResMut<VideoExportState>,
    mut timeline: ResMut<TimelineState>,
    mut events: EventReader<crate::systems::loading::FileLoadedEvent>,
) {
    if events.read().next().is_none() || state.status == VideoExportStatus::Idle {
        return;
    }

    if let Some(session) = state.internal.take() {
        cleanup_temp_dir(&session.temp_dir);
        timeline.interpolate = session.restore_interpolate;
        if session.restore_playing {
            timeline.play();
        }
    }
    state.status = VideoExportStatus::Idle;
    state.progress = 0.0;
    state.message = None;
    warn!("Video export cancelled due to new file load");
}

/// Register video export systems.
pub fn register(app: &mut App) {
    app.init_resource::<VideoExportState>()
        .add_event::<RequestVideoExportEvent>()
        .add_systems(
            Update,
            (
                handle_video_export_requests,
                cancel_video_export_on_load,
                video_export_step.after(crate::rendering::instanced::update_instanced_atom_colors),
            )
                .chain(),
        );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_ffmpeg_args_mp4() {
        let args = build_ffmpeg_args(Path::new("/out/movie.mp4"), Path::new("/tmp/frames"), 30, 0)
            .unwrap();
        assert!(args.iter().any(|a| a == "libx264"));
        assert!(args.iter().any(|a| a.contains("frame_%05d.png")));
        assert!(args.iter().any(|a| a == "0"));
        assert_eq!(args.last().map(String::as_str), Some("/out/movie.mp4"));
    }

    #[test]
    fn test_build_ffmpeg_args_webm() {
        let args = build_ffmpeg_args(
            Path::new("/out/movie.webm"),
            Path::new("/tmp/frames"),
            24,
            5,
        )
        .unwrap();
        assert!(args.iter().any(|a| a == "libvpx-vp9"));
        assert!(args.iter().any(|a| a == "5"));
    }

    #[test]
    fn test_build_ffmpeg_args_rejects_zero_fps() {
        assert!(build_ffmpeg_args(Path::new("a.mp4"), Path::new("/t"), 0, 0).is_err());
    }

    #[test]
    fn test_build_ffmpeg_args_rejects_unknown_ext() {
        assert!(build_ffmpeg_args(Path::new("a.avi"), Path::new("/t"), 30, 0).is_err());
    }
}
