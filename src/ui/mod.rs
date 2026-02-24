//! User interface systems (EGUI)
//!
//! Provides file intake via:
//! - CLI argument (handled in loading module)
//! - Drag-and-drop onto window
//! - Open button with native file dialog

use crate::io::FileFormat;
use crate::systems::loading::{
    CliFileArg, FileLoadErrorEvent, LoadFileEvent, SimulationData,
};
use crate::systems::spawning::AtomEntities;
use crate::core::trajectory::TimelineState;
use crate::interaction::selection::SelectionState;
use bevy::prelude::*;
use bevy::window::FileDragAndDrop;
use crossbeam_channel;
use std::path::Path;

/// Supported molecular file extensions for filtering
const SUPPORTED_EXTENSIONS: &[&str] = &["xyz", "pdb", "gro", "dcd", "cif", "mmcif", "mcif"];

/// Extensions that have implemented parsers (loadable)
const LOADABLE_EXTENSIONS: &[&str] = &["xyz", "pdb"];

/// Resource holding receiver for async file picker results
#[derive(Resource, Default)]
pub struct FilePickerState {
    /// Receiver for file path from dialog thread (None when no dialog pending)
    receiver: Option<crossbeam_channel::Receiver<Option<std::path::PathBuf>>>,
}

/// Check if a path has a supported/loadable molecular format
fn is_loadable_molecular_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            let ext_lower = ext.to_lowercase();
            LOADABLE_EXTENSIONS.iter().any(|e| *e == ext_lower)
        })
        .unwrap_or(false)
}

/// Handle files dropped onto the window
pub fn file_drop_handler(
    mut drop_events: EventReader<FileDragAndDrop>,
    mut load_events: EventWriter<LoadFileEvent>,
) {
    for event in drop_events.read() {
        if let FileDragAndDrop::DroppedFile { path_buf, .. } = event {
            if path_buf.exists() && is_loadable_molecular_file(path_buf) {
                info!("Loading dropped file: {:?}", path_buf);
                load_events.send(LoadFileEvent {
                    path: path_buf.clone(),
                });
            } else if path_buf.exists() {
                let format = FileFormat::from_path(path_buf);
                if !format.is_loadable() {
                    warn!(
                        "Dropped file format not yet supported: {:?}",
                        path_buf
                    );
                }
            } else {
                warn!("Dropped path does not exist: {:?}", path_buf);
            }
        }
    }
}

/// Poll for file picker result and send LoadFileEvent
pub fn file_picker_poll(
    mut picker_state: ResMut<FilePickerState>,
    mut load_events: EventWriter<LoadFileEvent>,
) {
    if let Some(receiver) = picker_state.receiver.take() {
        match receiver.try_recv() {
            Ok(Some(path)) => {
                if path.exists() && is_loadable_molecular_file(&path) {
                    info!("Loading file from dialog: {:?}", path);
                    load_events.send(LoadFileEvent { path });
                } else if path.exists() {
                    warn!("Selected file format not yet supported: {:?}", path);
                }
            }
            Ok(None) => {
                // User cancelled dialog
            }
            Err(crossbeam_channel::TryRecvError::Empty) => {
                // Still waiting for user - put receiver back
                picker_state.receiver = Some(receiver);
            }
            Err(crossbeam_channel::TryRecvError::Disconnected) => {
                // Thread finished without sending (shouldn't happen)
            }
        }
    }
}

/// Main UI panel: status, Open button, controls, error display
pub fn main_ui_panel(
    mut contexts: bevy_egui::EguiContexts,
    sim_data: Res<SimulationData>,
    atom_entities: Res<AtomEntities>,
    cli_arg: Res<CliFileArg>,
    mut picker_state: ResMut<FilePickerState>,
    mut load_errors: EventReader<FileLoadErrorEvent>,
    mut timeline: ResMut<TimelineState>,
    selection: Res<SelectionState>,
    mut commands: Commands,
) {
    let ctx = contexts.ctx_mut();

    // Collect latest load error for display
    let latest_error = load_errors.read().last().map(|e| e.error.clone());

    bevy_egui::egui::Window::new("Gumol Viz Engine")
        .default_width(320.0)
        .show(ctx, |ui| {
            ui.heading("File");
            ui.separator();

            // Open file button
            let dialog_pending = picker_state.receiver.is_some();
            if ui
                .add_enabled(!dialog_pending, bevy_egui::egui::Button::new("📂 Open file..."))
                .clicked()
            {
                let (tx, rx) = crossbeam_channel::unbounded();
                picker_state.receiver = Some(rx);

                std::thread::spawn(move || {
                    let result = rfd::FileDialog::new()
                        .add_filter("Molecular files (XYZ, PDB)", LOADABLE_EXTENSIONS)
                        .add_filter("All molecular formats", SUPPORTED_EXTENSIONS)
                        .add_filter("All files", &["*"])
                        .pick_file();
                    let _ = tx.send(result);
                });
            }

            if dialog_pending {
                ui.label(bevy_egui::egui::RichText::new("Opening dialog...").italics());
            }

            ui.separator();
            ui.heading("Status");
            ui.separator();

            if sim_data.loaded {
                ui.label(
                    bevy_egui::egui::RichText::new("✓ File loaded")
                        .color(bevy_egui::egui::Color32::from_rgb(0, 180, 0)),
                );
                ui.label(format!(
                    "  {}",
                    sim_data.trajectory.file_path.display()
                ));
                ui.label(format!("  Atoms: {}", sim_data.num_atoms()));
                ui.label(format!("  Frames: {}", sim_data.num_frames()));
                ui.label(format!("  Time: {:.2} fs", sim_data.total_time()));
                ui.label(format!("  Entities: {}", atom_entities.entities.len()));
            } else {
                ui.label(
                    bevy_egui::egui::RichText::new("✗ No file loaded")
                        .color(bevy_egui::egui::Color32::from_rgb(180, 80, 80)),
                );
                if cli_arg.0.is_some() {
                    ui.label("  (CLI file not found or invalid)");
                } else {
                    ui.label("  Displaying demo water molecule");
                }
            }

            if let Some(ref err) = latest_error {
                ui.separator();
                ui.label(
                    bevy_egui::egui::RichText::new("Error:")
                        .color(bevy_egui::egui::Color32::from_rgb(220, 50, 50)),
                );
                ui.label(
                    bevy_egui::egui::RichText::new(err.as_str())
                        .color(bevy_egui::egui::Color32::from_rgb(200, 100, 100)),
                );
            }

            ui.separator();
            ui.heading("Timeline");
            ui.separator();

            let total_frames = sim_data.num_frames();
            if total_frames > 1 {
                ui.label(format!("Frame: {} / {}", timeline.current_frame + 1, total_frames));
                ui.label(format!("Time: {:.2} ps", timeline.simulation_time(sim_data.trajectory.time_step) / 1000.0));

                // Progress bar
                let progress = timeline.progress();
                let mut frame_value = timeline.current_frame as f32;
                if ui.add(
                    bevy_egui::egui::Slider::new(&mut frame_value, 0.0..=(total_frames - 1) as f32)
                        .integer()
                        .step_by(1.0)
                        .show_value(false)
                ).changed() {
                    // Slider dragged - update frame
                    timeline.goto_frame(frame_value as usize);
                    timeline.pause();
                }

                // Playback controls
                ui.horizontal(|ui| {
                    // Play/Pause button
                    if ui.button(if timeline.is_playing { "⏸ Pause" } else { "▶ Play" }).clicked() {
                        timeline.toggle_playback();
                    }

                    // Stop button
                    if ui.button("⏹ Stop").clicked() {
                        timeline.stop();
                    }

                    // Previous frame
                    if ui.button("⏮").clicked() {
                        timeline.pause();
                        timeline.previous_frame();
                    }

                    // Next frame
                    if ui.button("⏭").clicked() {
                        timeline.pause();
                        timeline.next_frame();
                    }
                });

                // Playback speed
                ui.horizontal(|ui| {
                    ui.label("Speed:");
                    ui.add(bevy_egui::egui::Slider::new(&mut timeline.playback_speed, 0.1..=5.0).logarithmic(true).step_by(0.1));
                    ui.label(format!("x"));
                });

                // Options
                ui.horizontal(|ui| {
                    if ui.checkbox(&mut timeline.loop_playback, "Loop").changed() {}
                    if ui.checkbox(&mut timeline.interpolate, "Interpolate").changed() {}
                });
            } else if total_frames == 1 {
                ui.label("Single frame trajectory");
            } else {
                ui.label("No trajectory loaded");
            }

            ui.separator();
            ui.heading("Selection");
            ui.separator();

            ui.label(format!("Selected atoms: {}", selection.len()));

            // Clear selection button
            if !selection.is_empty() {
                if ui.button("Clear selection").clicked() {
                    for selected_entity in selection.entities().iter().copied().collect::<Vec<_>>() {
                        commands.entity(selected_entity).remove::<crate::interaction::selection::Selected>();
                    }
                    // SelectionState will be cleared by the clear_selection_on_load system
                }
            } else {
                ui.label("No atoms selected");
            }

            ui.separator();
            ui.heading("Controls");
            ui.separator();
            ui.label("  Mouse drag — Rotate camera");
            ui.label("  Scroll — Zoom");
            ui.label("  F11 — Toggle fullscreen");
            ui.label("  Drag file — Load molecular file");
            ui.label("  Click atom — Select atom");
            ui.label("  Shift+Click — Toggle selection");
            ui.label("  Escape — Clear selection");
            if total_frames > 1 {
                ui.separator();
                ui.label("Timeline controls:");
                ui.label("  Space — Play/Pause");
                ui.label("  ← → — Previous/Next frame");
                ui.label("  Home/End — First/Last frame");
                ui.label("  ↑ ↓ — Increase/Decrease speed");
                ui.label("  L — Toggle loop");
                ui.label("  I — Toggle interpolation");
            }
        });
}

/// Register all UI systems
pub fn register(app: &mut App) {
    app.init_resource::<FilePickerState>()
        .add_systems(
            Update,
            (
                file_drop_handler,
                file_picker_poll,
                main_ui_panel,
            ),
        );

    info!("UI module registered (file intake: CLI, drag-drop, Open button)");
}
