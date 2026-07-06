//! Keyboard shortcut help overlay

use bevy::prelude::*;

/// Toggle with `?` key
#[derive(Resource, Default, Debug)]
pub struct HelpOverlay {
    pub visible: bool,
}

pub fn toggle_help_overlay(keyboard: Res<ButtonInput<KeyCode>>, mut help: ResMut<HelpOverlay>) {
    let shift = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);
    if keyboard.just_pressed(KeyCode::Slash) && shift {
        help.visible = !help.visible;
    }
    if keyboard.just_pressed(KeyCode::F1) {
        help.visible = !help.visible;
    }
}

pub fn help_overlay_ui(mut contexts: bevy_egui::EguiContexts, mut help: ResMut<HelpOverlay>) {
    if !help.visible {
        return;
    }

    let ctx = contexts.ctx_mut();

    let mut close = false;
    bevy_egui::egui::Window::new("Keyboard Shortcuts")
        .collapsible(false)
        .resizable(false)
        .default_width(320.0)
        .show(ctx, |ui| {
            ui.heading("General");
            shortcut_row(ui, "Click", "Select atom");
            shortcut_row(ui, "Shift+Click", "Toggle atom in selection");
            shortcut_row(ui, "Escape", "Clear selection");
            shortcut_row(ui, "F", "Focus camera on molecule");
            shortcut_row(ui, "Shift+F", "Focus camera on selection");
            shortcut_row(ui, "F11", "Toggle fullscreen");
            shortcut_row(ui, "? (Shift+/)", "Toggle this help");

            ui.separator();
            ui.heading("Timeline");
            shortcut_row(ui, "Space", "Play / Pause");
            shortcut_row(ui, "← / →", "Previous / Next frame");
            shortcut_row(ui, "Home / End", "First / Last frame");
            shortcut_row(ui, "↑ / ↓", "Increase / Decrease speed");
            shortcut_row(ui, "L", "Toggle loop");
            shortcut_row(ui, "I", "Toggle smooth interpolation");

            ui.separator();
            ui.heading("Render modes");
            shortcut_row(ui, "1", "CPK");
            shortcut_row(ui, "2", "Ball-and-stick");
            shortcut_row(ui, "3", "Licorice");
            shortcut_row(ui, "4", "Wireframe");
            shortcut_row(ui, "5", "Points");

            if ui.button("Close").clicked() {
                close = true;
            }
        });

    if close {
        help.visible = false;
    }
}

fn shortcut_row(ui: &mut bevy_egui::egui::Ui, key: &str, action: &str) {
    ui.horizontal(|ui| {
        ui.label(bevy_egui::egui::RichText::new(key).monospace().strong());
        ui.label(action);
    });
}

pub fn register(app: &mut App) {
    app.init_resource::<HelpOverlay>()
        .add_systems(Update, toggle_help_overlay)
        .add_systems(Update, help_overlay_ui.after(toggle_help_overlay));
}
