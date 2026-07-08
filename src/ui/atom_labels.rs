//! Floating atom name labels projected over the 3D viewport.

use crate::interaction::pick_proxy::PickProxy;
use crate::interaction::selection::SelectionState;
use crate::rendering::atom_index::InstancedAtomIndex;
use crate::rendering::instanced::{InstancedAtomEntity, InstancedAtomMesh};
use bevy::prelude::*;

/// Toggle atom label overlay from the UI.
#[derive(Resource, Debug)]
pub struct AtomLabelSettings {
    pub show_selected: bool,
}

impl Default for AtomLabelSettings {
    fn default() -> Self {
        Self {
            show_selected: true,
        }
    }
}

const MAX_LABELS: usize = 48;

/// Draw egui text labels at screen positions for selected atoms.
pub fn atom_label_overlay(
    mut contexts: bevy_egui::EguiContexts,
    settings: Res<AtomLabelSettings>,
    selection: Res<SelectionState>,
    index: Res<InstancedAtomIndex>,
    instanced: Query<(&InstancedAtomEntity, &InstancedAtomMesh)>,
    atom_query: Query<(&PickProxy, &crate::core::atom::Atom)>,
    camera_q: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
) {
    if !settings.show_selected || selection.is_empty() {
        return;
    }

    let Ok((camera, camera_transform)) = camera_q.get_single() else {
        return;
    };

    let ctx = contexts.ctx_mut();
    let painter = ctx.layer_painter(bevy_egui::egui::LayerId::new(
        bevy_egui::egui::Order::Foreground,
        bevy_egui::egui::Id::new("atom_labels"),
    ));

    for &atom_id in selection.atom_ids().iter().take(MAX_LABELS) {
        let Some(world) = index.get_position(atom_id, &instanced) else {
            continue;
        };

        let Some(screen) = crate::interaction::box_selection::world_to_window(
            camera,
            camera_transform,
            world,
        ) else {
            continue;
        };

        let label = atom_query
            .iter()
            .find(|(p, _)| p.atom_id == atom_id)
            .map(|(_, atom)| {
                format!(
                    "{} {}",
                    atom.element.symbol(),
                    atom.name.trim()
                )
            })
            .unwrap_or_else(|| format!("#{atom_id}"));

        let pos = bevy_egui::egui::pos2(screen.x + 8.0, screen.y - 8.0);
        painter.text(
            pos,
            bevy_egui::egui::Align2::LEFT_BOTTOM,
            label,
            bevy_egui::egui::FontId::proportional(13.0),
            bevy_egui::egui::Color32::from_rgb(255, 255, 180),
        );
    }

    if selection.len() > MAX_LABELS {
        painter.text(
            bevy_egui::egui::pos2(12.0, 12.0),
            bevy_egui::egui::Align2::LEFT_TOP,
            format!("… {} more selected", selection.len() - MAX_LABELS),
            bevy_egui::egui::FontId::proportional(12.0),
            bevy_egui::egui::Color32::GRAY,
        );
    }
}

/// Rubber-band rectangle while middle-mouse box drag is active.
pub fn box_selection_overlay(
    mut contexts: bevy_egui::EguiContexts,
    box_state: Res<crate::interaction::box_selection::BoxSelectionState>,
) {
    if !box_state.dragging {
        return;
    }

    let rect = crate::interaction::box_selection::ScreenRect::from_corners(
        box_state.start,
        box_state.current,
    );

    let ctx = contexts.ctx_mut();
    let painter = ctx.layer_painter(bevy_egui::egui::LayerId::new(
        bevy_egui::egui::Order::Foreground,
        bevy_egui::egui::Id::new("box_selection"),
    ));

    let egui_rect = bevy_egui::egui::Rect::from_min_max(
        bevy_egui::egui::pos2(rect.min.x, rect.min.y),
        bevy_egui::egui::pos2(rect.max.x, rect.max.y),
    );

    painter.rect_stroke(
        egui_rect,
        0.0,
        bevy_egui::egui::Stroke::new(1.5, bevy_egui::egui::Color32::from_rgb(100, 200, 255)),
    );
    painter.rect_filled(
        egui_rect,
        0.0,
        bevy_egui::egui::Color32::from_rgba_unmultiplied(100, 180, 255, 30),
    );
}

pub fn register(app: &mut App) {
    app.init_resource::<AtomLabelSettings>()
        .add_systems(Update, (atom_label_overlay, box_selection_overlay));
}
