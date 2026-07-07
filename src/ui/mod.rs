//! User interface systems (EGUI)

pub mod help;
pub mod inspector;
pub mod notifications;

use crate::core::secondary_structure::ProteinBackbone;
use crate::core::secondary_structure::MIN_CARTOON_RESIDUES;
use crate::core::trajectory::TimelineState;
use crate::core::visualization::{ColorScheme, RenderMode, VisualizationConfig};
use crate::export::gltf_export::RequestExportGltfEvent;
use crate::export::obj::RequestExportObjEvent;
use crate::export::screenshot::RequestScreenshotEvent;
use crate::export::video::{RequestVideoExportEvent, VideoExportSettings, VideoExportState};
use crate::interaction::measurement::MeasurementState;
use crate::interaction::selection::SelectionState;
use crate::io::FileFormat;
use crate::performance::{memory, PerformanceDiagnostics};
use crate::rendering::instanced::InstancedAtomEntities;
use crate::systems::bonds::{BondDetectionConfig, BondEntities};
use crate::systems::loading::{
    AsyncLoadState, CliFileArg, FileLoadErrorEvent, LoadFileEvent, LoadTopologyEvent,
    SimulationData, TopologyState,
};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::window::FileDragAndDrop;
use crossbeam_channel;
use std::path::Path;

/// Resource holding receiver for async topology file picker results
#[derive(Resource, Default)]
pub struct TopologyPickerState {
    receiver: Option<crossbeam_channel::Receiver<Option<std::path::PathBuf>>>,
}

/// Supported molecular file extensions for filtering
const SUPPORTED_EXTENSIONS: &[&str] = &["xyz", "pdb", "gro", "dcd", "cif", "mmcif", "mcif"];

/// Extensions that have implemented parsers (loadable)
const LOADABLE_EXTENSIONS: &[&str] = &["xyz", "pdb", "gro", "cif", "mmcif", "mcif", "dcd"];

/// Resource holding receiver for async file picker results
#[derive(Resource, Default)]
pub struct FilePickerState {
    /// Receiver for file path from dialog thread (None when no dialog pending)
    receiver: Option<crossbeam_channel::Receiver<Option<std::path::PathBuf>>>,
}

/// Resource holding receiver for async screenshot save path
#[derive(Resource, Default)]
pub struct ScreenshotSaveState {
    receiver: Option<crossbeam_channel::Receiver<Option<std::path::PathBuf>>>,
}

/// Resource holding receiver for async OBJ export save path
#[derive(Resource, Default)]
pub struct ObjSaveState {
    receiver: Option<crossbeam_channel::Receiver<Option<std::path::PathBuf>>>,
}

/// Resource holding receiver for async glTF export save path
#[derive(Resource, Default)]
pub struct GltfSaveState {
    receiver: Option<crossbeam_channel::Receiver<Option<std::path::PathBuf>>>,
}

/// Resource holding receiver for async video export save path
#[derive(Resource, Default)]
pub struct VideoSaveState {
    receiver: Option<crossbeam_channel::Receiver<Option<std::path::PathBuf>>>,
}

#[derive(SystemParam)]
pub struct ExportSaveStates<'w> {
    pub screenshot: ResMut<'w, ScreenshotSaveState>,
    pub obj: ResMut<'w, ObjSaveState>,
    pub gltf: ResMut<'w, GltfSaveState>,
    pub video: ResMut<'w, VideoSaveState>,
}

#[derive(SystemParam)]
pub struct ExportPanelState<'w> {
    pub saves: ExportSaveStates<'w>,
    pub video: Res<'w, VideoExportState>,
}

#[derive(SystemParam)]
pub struct VisualizationUiState<'w> {
    pub viz_config: ResMut<'w, VisualizationConfig>,
    pub bond_config: ResMut<'w, BondDetectionConfig>,
    pub bond_entities: Res<'w, BondEntities>,
    pub backbone: Res<'w, ProteinBackbone>,
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

/// Topology file extensions (structure files for DCD pairing)
const TOPOLOGY_EXTENSIONS: &[&str] = &["pdb", "gro", "cif", "mmcif", "mcif"];

/// Poll for topology file picker result
pub fn topology_picker_poll(
    mut picker_state: ResMut<TopologyPickerState>,
    mut load_topology_events: EventWriter<LoadTopologyEvent>,
) {
    if let Some(receiver) = picker_state.receiver.take() {
        match receiver.try_recv() {
            Ok(Some(path)) => {
                if path.exists() {
                    info!("Loading topology from dialog: {:?}", path);
                    load_topology_events.send(LoadTopologyEvent { path });
                }
            }
            Ok(None) => {}
            Err(crossbeam_channel::TryRecvError::Empty) => {
                picker_state.receiver = Some(receiver);
            }
            Err(crossbeam_channel::TryRecvError::Disconnected) => {}
        }
    }
}

/// Handle files dropped onto window
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
                    warn!("Dropped file format not yet supported: {:?}", path_buf);
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

/// Poll for screenshot save path and send RequestScreenshotEvent
pub fn screenshot_save_poll(
    mut save_state: ResMut<ScreenshotSaveState>,
    mut screenshot_events: EventWriter<RequestScreenshotEvent>,
) {
    if let Some(receiver) = save_state.receiver.take() {
        match receiver.try_recv() {
            Ok(Some(path)) => {
                screenshot_events.send(RequestScreenshotEvent { path });
            }
            Ok(None) => {}
            Err(crossbeam_channel::TryRecvError::Empty) => {
                save_state.receiver = Some(receiver);
            }
            Err(crossbeam_channel::TryRecvError::Disconnected) => {}
        }
    }
}

/// Poll for OBJ export save path and send RequestExportObjEvent
pub fn export_obj_save_poll(
    mut save_state: ResMut<ObjSaveState>,
    mut export_events: EventWriter<RequestExportObjEvent>,
) {
    if let Some(receiver) = save_state.receiver.take() {
        match receiver.try_recv() {
            Ok(Some(path)) => {
                export_events.send(RequestExportObjEvent { path });
            }
            Ok(None) => {}
            Err(crossbeam_channel::TryRecvError::Empty) => {
                save_state.receiver = Some(receiver);
            }
            Err(crossbeam_channel::TryRecvError::Disconnected) => {}
        }
    }
}

/// Poll for glTF export save path and send RequestExportGltfEvent
pub fn export_gltf_save_poll(
    mut save_state: ResMut<GltfSaveState>,
    mut export_events: EventWriter<RequestExportGltfEvent>,
) {
    if let Some(receiver) = save_state.receiver.take() {
        match receiver.try_recv() {
            Ok(Some(path)) => {
                export_events.send(RequestExportGltfEvent { path });
            }
            Ok(None) => {}
            Err(crossbeam_channel::TryRecvError::Empty) => {
                save_state.receiver = Some(receiver);
            }
            Err(crossbeam_channel::TryRecvError::Disconnected) => {}
        }
    }
}

/// Poll for video export save path and send RequestVideoExportEvent
pub fn video_save_poll(
    mut save_state: ResMut<VideoSaveState>,
    mut export_events: EventWriter<RequestVideoExportEvent>,
    sim_data: Res<SimulationData>,
) {
    if let Some(receiver) = save_state.receiver.take() {
        match receiver.try_recv() {
            Ok(Some(path)) => {
                let total = sim_data.num_frames().max(1);
                export_events.send(RequestVideoExportEvent {
                    output_path: path,
                    settings: VideoExportSettings {
                        fps: crate::export::video::DEFAULT_VIDEO_FPS,
                        start_frame: 0,
                        end_frame: total.saturating_sub(1),
                    },
                });
            }
            Ok(None) => {}
            Err(crossbeam_channel::TryRecvError::Empty) => {
                save_state.receiver = Some(receiver);
            }
            Err(crossbeam_channel::TryRecvError::Disconnected) => {}
        }
    }
}

#[derive(SystemParam)]
pub struct TopologyUiState<'w> {
    pub topology_state: Res<'w, TopologyState>,
    pub topology_picker: ResMut<'w, TopologyPickerState>,
}

/// Main UI panel: status, Open button, controls, error display
#[allow(clippy::too_many_arguments)]
pub fn main_ui_panel(
    mut contexts: bevy_egui::EguiContexts,
    sim_data: Res<SimulationData>,
    mut topology_ui: TopologyUiState,
    instanced_entities: Res<InstancedAtomEntities>,
    async_load: Res<AsyncLoadState>,
    perf_diag: Res<PerformanceDiagnostics>,
    cli_arg: Res<CliFileArg>,
    mut picker_state: ResMut<FilePickerState>,
    mut export_panel: ExportPanelState,
    mut load_errors: EventReader<FileLoadErrorEvent>,
    mut timeline: ResMut<TimelineState>,
    timeline_frames: Res<crate::systems::frame_cache::TimelineFrames>,
    mut selection: ResMut<SelectionState>,
    measurements: Res<MeasurementState>,
    mut commands: Commands,
    mut viz_ui: VisualizationUiState,
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
                .add_enabled(
                    !dialog_pending,
                    bevy_egui::egui::Button::new("📂 Open file..."),
                )
                .clicked()
            {
                let (tx, rx) = crossbeam_channel::unbounded();
                picker_state.receiver = Some(rx);

                std::thread::spawn(move || {
                    let result = rfd::FileDialog::new()
                        .add_filter(
                            "Molecular files (XYZ, PDB, GRO, mmCIF, DCD)",
                            LOADABLE_EXTENSIONS,
                        )
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
                ui.label(format!("  {}", sim_data.trajectory.file_path.display()));
                ui.label(format!("  Atoms: {}", sim_data.num_atoms()));
                ui.label(format!("  Frames: {}", sim_data.num_frames()));
                ui.label(format!("  Time: {:.2} fs", sim_data.total_time()));
                ui.label(format!(
                    "  Draw calls: ~{}",
                    instanced_entities.entities.len()
                ));
                if async_load.in_progress {
                    ui.label(
                        bevy_egui::egui::RichText::new("  Loading file (background)...")
                            .color(bevy_egui::egui::Color32::from_rgb(200, 180, 50)),
                    );
                }
                if perf_diag.estimated_bytes > 0 {
                    ui.label(format!(
                        "  Est. memory: {}",
                        memory::format_bytes(perf_diag.estimated_bytes)
                    ));
                }
                if let Some(ref warn) = perf_diag.memory_warning {
                    ui.label(
                        bevy_egui::egui::RichText::new(format!("  ⚠ {warn}"))
                            .color(bevy_egui::egui::Color32::from_rgb(220, 140, 50)),
                    );
                }
                if perf_diag.selection_disabled {
                    if let Some(ref reason) = perf_diag.selection_disabled_reason {
                        ui.label(
                            bevy_egui::egui::RichText::new(format!("  ⚠ {reason}"))
                                .color(bevy_egui::egui::Color32::from_rgb(220, 140, 50)),
                        );
                    }
                }
                if sim_data.needs_topology {
                    ui.separator();
                    ui.label(
                        bevy_egui::egui::RichText::new(
                            "⚠ DCD loaded without topology — element colors are placeholders",
                        )
                        .color(bevy_egui::egui::Color32::from_rgb(220, 140, 50)),
                    );
                    let topo_pending = topology_ui.topology_picker.receiver.is_some();
                    if ui
                        .add_enabled(
                            !topo_pending,
                            bevy_egui::egui::Button::new("Load topology (PDB/GRO/mmCIF)..."),
                        )
                        .clicked()
                    {
                        let (tx, rx) = crossbeam_channel::unbounded();
                        topology_ui.topology_picker.receiver = Some(rx);
                        std::thread::spawn(move || {
                            let result = rfd::FileDialog::new()
                                .add_filter("Topology files", TOPOLOGY_EXTENSIONS)
                                .pick_file();
                            let _ = tx.send(result);
                        });
                    }
                    if topo_pending {
                        ui.label(
                            bevy_egui::egui::RichText::new("Opening topology dialog...").italics(),
                        );
                    }
                } else if topology_ui.topology_state.path.is_some() {
                    ui.label(format!(
                        "  Topology: {}",
                        topology_ui.topology_state.path.as_ref().unwrap().display()
                    ));
                }
                ui.label(format!(
                    "  Visible/Culled: {}/{}  LOD: {}",
                    perf_diag.visible_instance_count,
                    perf_diag.culled_instance_count,
                    perf_diag.current_lod.name()
                ));
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
                let time_ps = timeline.simulation_time(sim_data.trajectory.time_step) / 1000.0;
                ui.label(format!(
                    "Frame: {} / {}",
                    timeline.current_frame + 1,
                    total_frames
                ));
                ui.label(format!("Time: {:.3} ps", time_ps));
                if timeline.interpolate && timeline.is_playing {
                    ui.label(
                        bevy_egui::egui::RichText::new(format!(
                            "  (α = {:.0}%)",
                            timeline.frame_alpha() * 100.0
                        ))
                        .small()
                        .italics(),
                    );
                }

                if timeline_frames.loading {
                    ui.label(
                        bevy_egui::egui::RichText::new("Loading frame…")
                            .color(bevy_egui::egui::Color32::YELLOW),
                    );
                }

                if timeline_frames.streaming {
                    ui.label(
                        bevy_egui::egui::RichText::new(format!(
                            "Streaming: {} cached frames",
                            timeline_frames.cached_count
                        ))
                        .small(),
                    );
                }

                // Frame scrubber (includes sub-frame progress when interpolating)
                let mut progress = timeline.progress();
                if ui
                    .add(
                        bevy_egui::egui::Slider::new(&mut progress, 0.0..=1.0)
                            .show_value(false)
                            .text("Scrub"),
                    )
                    .changed()
                {
                    let max = (total_frames - 1) as f32;
                    let frame_f = progress * max;
                    timeline.goto_frame(frame_f.floor() as usize);
                    timeline.interpolation_factor = frame_f.fract();
                    timeline.pause();
                }

                // Jump to frame number
                ui.horizontal(|ui| {
                    ui.label("Go to frame:");
                    let mut frame_input = (timeline.current_frame + 1) as u32;
                    if ui
                        .add(
                            bevy_egui::egui::DragValue::new(&mut frame_input)
                                .range(1..=total_frames as u32)
                                .speed(1),
                        )
                        .changed()
                    {
                        timeline.goto_frame((frame_input as usize).saturating_sub(1));
                        timeline.pause();
                    }
                });

                // Playback controls
                ui.horizontal(|ui| {
                    // Play/Pause button
                    if ui
                        .button(if timeline.is_playing {
                            "⏸ Pause"
                        } else {
                            "▶ Play"
                        })
                        .clicked()
                    {
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
                    ui.add(
                        bevy_egui::egui::Slider::new(&mut timeline.playback_speed, 0.1..=5.0)
                            .logarithmic(true)
                            .step_by(0.1),
                    );
                    ui.label(format!("{:.2}x", timeline.playback_speed));
                });

                ui.horizontal(|ui| {
                    ui.label("Presets:");
                    if ui.button("0.25x").clicked() {
                        timeline.playback_speed = 0.25;
                    }
                    if ui.button("0.5x").clicked() {
                        timeline.playback_speed = 0.5;
                    }
                    if ui.button("1x").clicked() {
                        timeline.playback_speed = 1.0;
                    }
                    if ui.button("2x").clicked() {
                        timeline.playback_speed = 2.0;
                    }
                    if ui.button("4x").clicked() {
                        timeline.playback_speed = 4.0;
                    }
                });

                // Options
                ui.horizontal(|ui| {
                    ui.checkbox(&mut timeline.loop_playback, "Loop");
                    ui.checkbox(&mut timeline.interpolate, "Smooth playback");
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
            if selection.len() < 2 {
                ui.label(
                    bevy_egui::egui::RichText::new("Shift+Click to add atoms for measurements")
                        .small(),
                );
            }

            // Measurement display
            if selection.len() >= 2 {
                if let Some(d) = measurements.distance {
                    ui.label(
                        bevy_egui::egui::RichText::new(format!("Distance: {:.3} Å", d))
                            .color(bevy_egui::egui::Color32::from_rgb(100, 200, 100)),
                    );
                }
            }
            if selection.len() >= 3 {
                if let Some(a) = measurements.angle {
                    ui.label(
                        bevy_egui::egui::RichText::new(format!("Angle: {:.2}°", a))
                            .color(bevy_egui::egui::Color32::from_rgb(100, 200, 100)),
                    );
                }
            }
            if selection.len() >= 4 {
                if let Some(d) = measurements.dihedral {
                    ui.label(
                        bevy_egui::egui::RichText::new(format!("Dihedral: {:.2}°", d))
                            .color(bevy_egui::egui::Color32::from_rgb(100, 200, 100)),
                    );
                }
            }

            // Clear selection button
            if !selection.is_empty() {
                if ui.button("Clear selection").clicked() {
                    for selected_entity in selection.entities().to_vec() {
                        commands
                            .entity(selected_entity)
                            .remove::<crate::interaction::selection::Selected>();
                    }
                    selection.clear();
                    commands.trigger(crate::interaction::selection::SelectionClearedEvent);
                }
            } else {
                ui.label("No atoms selected");
            }

            ui.separator();
            ui.heading("Visualization");
            ui.separator();

            // Visualization mode selector
            ui.label("Mode:");
            bevy_egui::egui::ComboBox::from_label("")
                .selected_text(viz_ui.viz_config.render_mode.name())
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut viz_ui.viz_config.render_mode,
                        RenderMode::CPK,
                        RenderMode::CPK.name(),
                    );
                    ui.selectable_value(
                        &mut viz_ui.viz_config.render_mode,
                        RenderMode::BallAndStick,
                        RenderMode::BallAndStick.name(),
                    );
                    ui.selectable_value(
                        &mut viz_ui.viz_config.render_mode,
                        RenderMode::Licorice,
                        RenderMode::Licorice.name(),
                    );
                    ui.selectable_value(
                        &mut viz_ui.viz_config.render_mode,
                        RenderMode::Wireframe,
                        RenderMode::Wireframe.name(),
                    );
                    ui.selectable_value(
                        &mut viz_ui.viz_config.render_mode,
                        RenderMode::Points,
                        RenderMode::Points.name(),
                    );
                    ui.selectable_value(
                        &mut viz_ui.viz_config.render_mode,
                        RenderMode::Surface,
                        RenderMode::Surface.name(),
                    );

                    ui.separator();
                    ui.label("Protein backbone:");
                    ui.add_enabled_ui(viz_ui.backbone.cartoon_available, |ui| {
                        ui.selectable_value(
                            &mut viz_ui.viz_config.render_mode,
                            RenderMode::Cartoon,
                            RenderMode::Cartoon.name(),
                        );
                        ui.selectable_value(
                            &mut viz_ui.viz_config.render_mode,
                            RenderMode::Tube,
                            RenderMode::Tube.name(),
                        );
                        ui.selectable_value(
                            &mut viz_ui.viz_config.render_mode,
                            RenderMode::Trace,
                            RenderMode::Trace.name(),
                        );
                    });
                    if !viz_ui.backbone.cartoon_available {
                        ui.label(format!(
                            "Requires ≥{MIN_CARTOON_RESIDUES} CA atoms (found {})",
                            viz_ui.backbone.ca_count
                        ));
                    }
                });

            ui.separator();

            ui.label("Color scheme:");
            bevy_egui::egui::ComboBox::from_label("")
                .selected_text(viz_ui.viz_config.color_scheme.name())
                .show_ui(ui, |ui| {
                    for scheme in ColorScheme::UI_SCHEMES {
                        ui.selectable_value(
                            &mut viz_ui.viz_config.color_scheme,
                            *scheme,
                            scheme.name(),
                        );
                    }
                });

            ui.separator();

            // Atom size control
            ui.label("Atom Scale:");
            if ui
                .add(
                    bevy_egui::egui::Slider::new(&mut viz_ui.viz_config.atom_scale, 0.1..=2.0)
                        .logarithmic(true)
                        .step_by(0.1),
                )
                .changed()
            {
                viz_ui.viz_config.atom_scale = viz_ui.viz_config.atom_scale.clamp(0.1, 2.0);
            }
            ui.label(format!("x ({:.2}x)", viz_ui.viz_config.atom_scale));

            ui.separator();

            // Bond size control
            ui.label("Bond Scale:");
            if ui
                .add(
                    bevy_egui::egui::Slider::new(&mut viz_ui.viz_config.bond_scale, 0.1..=3.0)
                        .logarithmic(true)
                        .step_by(0.1),
                )
                .changed()
            {
                viz_ui.viz_config.bond_scale = viz_ui.viz_config.bond_scale.clamp(0.1, 3.0);
            }
            ui.label(format!("x ({:.2}x)", viz_ui.viz_config.bond_scale));

            ui.separator();

            // Visibility toggles
            ui.checkbox(&mut viz_ui.viz_config.show_atoms, "Show atoms");
            ui.checkbox(&mut viz_ui.viz_config.show_bonds, "Show bonds");

            ui.separator();
            ui.heading("Bonds");
            ui.separator();

            // Enable/disable bond detection
            ui.checkbox(&mut viz_ui.bond_config.enabled, "Show bonds");

            if viz_ui.bond_config.enabled {
                ui.label(format!(
                    "Bond count: {}",
                    viz_ui.bond_entities.entities.len()
                ));

                // Distance settings
                ui.label("Detection settings:");
                ui.horizontal(|ui| {
                    ui.label("Multiplier:");
                    ui.add(
                        bevy_egui::egui::Slider::new(
                            &mut viz_ui.bond_config.distance_multiplier,
                            1.0..=2.0,
                        )
                        .step_by(0.1),
                    );
                    ui.label("x".to_string());
                });

                ui.horizontal(|ui| {
                    ui.label("Max distance:");
                    ui.add(
                        bevy_egui::egui::Slider::new(
                            &mut viz_ui.bond_config.max_bond_distance,
                            2.0..=5.0,
                        )
                        .step_by(0.1),
                    );
                    ui.label("Å".to_string());
                });

                ui.checkbox(
                    &mut viz_ui.bond_config.same_residue_only,
                    "Same residue only",
                );
            } else {
                ui.label("Bond detection disabled");
            }

            ui.separator();
            ui.heading("Export");
            ui.separator();

            let screenshot_pending = export_panel.saves.screenshot.receiver.is_some();
            let obj_pending = export_panel.saves.obj.receiver.is_some();
            let gltf_pending = export_panel.saves.gltf.receiver.is_some();
            let video_dialog_pending = export_panel.saves.video.receiver.is_some();
            let video_recording = export_panel.video.status
                != crate::export::video::VideoExportStatus::Idle;
            let any_export_pending = screenshot_pending
                || obj_pending
                || gltf_pending
                || video_dialog_pending;

            if ui
                .add_enabled(
                    !screenshot_pending,
                    bevy_egui::egui::Button::new("📷 Screenshot..."),
                )
                .clicked()
            {
                let (tx, rx) = crossbeam_channel::unbounded();
                export_panel.saves.screenshot.receiver = Some(rx);

                std::thread::spawn(move || {
                    let result = rfd::FileDialog::new()
                        .add_filter("PNG image", &["png"])
                        .add_filter("JPEG image", &["jpg", "jpeg"])
                        .set_file_name("gumol_screenshot.png")
                        .save_file();
                    let _ = tx.send(result);
                });
            }

            if ui
                .add_enabled(
                    !obj_pending,
                    bevy_egui::egui::Button::new("📦 Export OBJ..."),
                )
                .clicked()
            {
                let (tx, rx) = crossbeam_channel::unbounded();
                export_panel.saves.obj.receiver = Some(rx);

                std::thread::spawn(move || {
                    let result = rfd::FileDialog::new()
                        .add_filter("Wavefront OBJ", &["obj"])
                        .set_file_name("molecule.obj")
                        .save_file();
                    let _ = tx.send(result);
                });
            }

            if ui
                .add_enabled(
                    !gltf_pending,
                    bevy_egui::egui::Button::new("📦 Export glTF..."),
                )
                .clicked()
            {
                let (tx, rx) = crossbeam_channel::unbounded();
                export_panel.saves.gltf.receiver = Some(rx);

                std::thread::spawn(move || {
                    let result = rfd::FileDialog::new()
                        .add_filter("glTF 2.0 (JSON + embedded buffer)", &["gltf"])
                        .set_file_name("molecule.gltf")
                        .save_file();
                    let _ = tx.send(result);
                });
            }

            if ui
                .add_enabled(
                    sim_data.loaded
                        && sim_data.num_frames() > 0
                        && !video_dialog_pending
                        && !video_recording,
                    bevy_egui::egui::Button::new("🎬 Record video..."),
                )
                .clicked()
            {
                let (tx, rx) = crossbeam_channel::unbounded();
                export_panel.saves.video.receiver = Some(rx);

                std::thread::spawn(move || {
                    let result = rfd::FileDialog::new()
                        .add_filter("MP4 video", &["mp4"])
                        .add_filter("WebM video", &["webm"])
                        .add_filter("GIF animation", &["gif"])
                        .set_file_name("trajectory.mp4")
                        .save_file();
                    let _ = tx.send(result);
                });
            }

            if video_recording {
                if let Some(ref msg) = export_panel.video.message {
                    ui.label(
                        bevy_egui::egui::RichText::new(msg.as_str())
                            .color(bevy_egui::egui::Color32::LIGHT_BLUE),
                    );
                }
                if export_panel.video.progress >= 0.0 {
                    ui.add(
                        bevy_egui::egui::ProgressBar::new(export_panel.video.progress)
                            .show_percentage(),
                    );
                } else {
                    ui.label(
                        bevy_egui::egui::RichText::new("Encoding…").italics(),
                    );
                }
            }

            if any_export_pending {
                ui.label(bevy_egui::egui::RichText::new("Choosing save location...").italics());
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

/// Keyboard shortcuts for render modes (1–5)
pub fn render_mode_shortcuts(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut viz_config: ResMut<VisualizationConfig>,
) {
    if keyboard.just_pressed(KeyCode::Digit1) {
        viz_config.render_mode = RenderMode::CPK;
    } else if keyboard.just_pressed(KeyCode::Digit2) {
        viz_config.render_mode = RenderMode::BallAndStick;
    } else if keyboard.just_pressed(KeyCode::Digit3) {
        viz_config.render_mode = RenderMode::Licorice;
    } else if keyboard.just_pressed(KeyCode::Digit4) {
        viz_config.render_mode = RenderMode::Wireframe;
    } else if keyboard.just_pressed(KeyCode::Digit5) {
        viz_config.render_mode = RenderMode::Points;
    }
}

/// Register all UI systems
pub fn register(app: &mut App) {
    help::register(app);
    notifications::register(app);

    app.init_resource::<FilePickerState>()
        .init_resource::<TopologyPickerState>()
        .init_resource::<ScreenshotSaveState>()
        .init_resource::<ObjSaveState>()
        .init_resource::<GltfSaveState>()
        .init_resource::<VideoSaveState>()
        .add_systems(
            Update,
            (
                file_drop_handler,
                file_picker_poll,
                topology_picker_poll,
                screenshot_save_poll,
                export_obj_save_poll,
                export_gltf_save_poll,
                video_save_poll,
                render_mode_shortcuts,
            ),
        )
        .add_systems(Update, (main_ui_panel, inspector::inspector_ui));

    info!("UI module registered");
}
