//! Atom inspector panel for selected atoms

use crate::core::atom::Atom;
use crate::interaction::pick_proxy::PickProxy;
use crate::interaction::selection::SelectionState;
use crate::rendering::atom_index::InstancedAtomIndex;
use crate::rendering::instanced::{InstancedAtomEntity, InstancedAtomMesh};
use bevy::prelude::*;
use bevy_panorbit_camera::PanOrbitCamera;

/// Inspector UI window for selected atom details
pub fn inspector_ui(
    mut contexts: bevy_egui::EguiContexts,
    selection: Res<SelectionState>,
    index: Res<InstancedAtomIndex>,
    instanced: Query<(&InstancedAtomEntity, &InstancedAtomMesh)>,
    atom_query: Query<(&PickProxy, &Atom)>,
    mut camera_query: Query<&mut PanOrbitCamera, With<Camera3d>>,
) {
    if selection.is_empty() {
        return;
    }

    let ctx = contexts.ctx_mut();

    bevy_egui::egui::Window::new("Inspector")
        .default_width(280.0)
        .default_pos([340.0, 20.0])
        .show(ctx, |ui| {
            ui.label(format!("{} atom(s) selected", selection.len()));

            if ui.button("Focus camera on selection").clicked() {
                focus_camera_on_selection(&selection, &index, &instanced, &mut camera_query);
            }

            ui.separator();

            bevy_egui::egui::ScrollArea::vertical()
                .max_height(320.0)
                .show(ui, |ui| {
                    let ids: Vec<u32> = selection.atom_ids().iter().copied().take(20).collect();

                    for atom_id in ids {
                        if let Some((proxy, atom)) =
                            atom_query.iter().find(|(p, _)| p.atom_id == atom_id)
                        {
                            let pos = index
                                .get_position(atom_id, &instanced)
                                .unwrap_or(atom.position);

                            ui.group(|ui| {
                                ui.label(
                                    bevy_egui::egui::RichText::new(format!(
                                        "#{} {} ({})",
                                        atom.id,
                                        atom.name,
                                        atom.element.symbol()
                                    ))
                                    .strong(),
                                );
                                ui.label(format!(
                                    "Residue: {} {} (chain {})",
                                    atom.residue_name, atom.residue_id, atom.chain_id
                                ));
                                ui.label(format!(
                                    "Position: {:.3}, {:.3}, {:.3} Å",
                                    pos.x, pos.y, pos.z
                                ));
                                if atom.b_factor > 0.0 || atom.occupancy != 1.0 {
                                    ui.label(format!(
                                        "B-factor: {:.2}  Occupancy: {:.2}",
                                        atom.b_factor, atom.occupancy
                                    ));
                                }
                                let _ = proxy;
                            });
                            ui.add_space(4.0);
                        }
                    }

                    if selection.len() > 20 {
                        ui.label(format!("… and {} more", selection.len() - 20));
                    }
                });
        });
}

fn focus_camera_on_selection(
    selection: &SelectionState,
    index: &InstancedAtomIndex,
    instanced: &Query<(&InstancedAtomEntity, &InstancedAtomMesh)>,
    camera_query: &mut Query<&mut PanOrbitCamera, With<Camera3d>>,
) {
    let mut sum = Vec3::ZERO;
    let mut count = 0;

    for &atom_id in selection.atom_ids() {
        if let Some(pos) = index.get_position(atom_id, instanced) {
            sum += pos;
            count += 1;
        }
    }

    if count == 0 {
        return;
    }

    let center = sum / count as f32;
    for mut cam in camera_query.iter_mut() {
        cam.focus = center;
        cam.target_focus = center;
    }
}
