//! Transient UI notifications (e.g. file loaded toast)

use bevy::prelude::*;

#[derive(Resource, Default, Debug)]
pub struct UiNotifications {
    pub message: Option<String>,
    ticks_remaining: u32,
}

impl UiNotifications {
    pub fn show(&mut self, message: impl Into<String>, duration_ticks: u32) {
        self.message = Some(message.into());
        self.ticks_remaining = duration_ticks;
    }

    pub fn tick(&mut self) {
        if self.ticks_remaining > 0 {
            self.ticks_remaining -= 1;
            if self.ticks_remaining == 0 {
                self.message = None;
            }
        }
    }
}

pub fn on_file_loaded_notify(
    mut notifications: ResMut<UiNotifications>,
    mut events: EventReader<crate::systems::loading::FileLoadedEvent>,
) {
    for event in events.read() {
        notifications.show(
            format!(
                "Loaded {} — {} atoms, {} frames",
                event
                    .path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("file"),
                event.num_atoms,
                event.num_frames
            ),
            180, // ~3 seconds at 60 FPS
        );
    }
}

pub fn notification_banner(
    mut contexts: bevy_egui::EguiContexts,
    mut notifications: ResMut<UiNotifications>,
) {
    notifications.tick();

    let Some(ref message) = notifications.message else {
        return;
    };

    let ctx = contexts.ctx_mut();
    bevy_egui::egui::Area::new(bevy_egui::egui::Id::new("load_notification"))
        .fixed_pos(bevy_egui::egui::pos2(
            ctx.screen_rect().center().x - 160.0,
            12.0,
        ))
        .show(ctx, |ui| {
            bevy_egui::egui::Frame::popup(ui.style())
                .fill(bevy_egui::egui::Color32::from_rgb(30, 80, 40))
                .show(ui, |ui| {
                    ui.label(
                        bevy_egui::egui::RichText::new(message.as_str())
                            .color(bevy_egui::egui::Color32::WHITE),
                    );
                });
        });
}

pub fn register(app: &mut App) {
    app.init_resource::<UiNotifications>()
        .add_systems(Update, (on_file_loaded_notify, notification_banner));
}
