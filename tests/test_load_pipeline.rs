//! End-to-end load pipeline: LoadFileEvent → SimulationData populated.

mod common;

use bevy::prelude::*;
use common::fixture;
use gumol_viz_engine::systems::loading::{
    handle_load_file_events_sync, FileLoadedEvent, FileLoadErrorEvent, LoadFileEvent,
    SimulationData,
};

fn load_test_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<SimulationData>();
    app.add_event::<LoadFileEvent>();
    app.add_event::<FileLoadedEvent>();
    app.add_event::<FileLoadErrorEvent>();
    app.add_systems(Update, handle_load_file_events_sync);
    app
}

#[test]
fn test_load_file_event_populates_simulation_data() {
    let path = fixture("water.xyz");
    let mut app = load_test_app();

    app.world_mut()
        .send_event(LoadFileEvent { path: path.clone() });
    app.update();

    let sim = app.world().resource::<SimulationData>();
    assert!(sim.loaded, "SimulationData should be marked loaded");
    assert_eq!(sim.num_atoms(), 3);
    assert_eq!(sim.num_frames(), 1);

    let mut success_events = app.world_mut().resource_mut::<Events<FileLoadedEvent>>();
    let loaded: Vec<_> = success_events.drain().collect();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].num_atoms, 3);
    assert_eq!(loaded[0].path, path);
}

#[test]
fn test_load_missing_file_emits_error_event() {
    let path = fixture("does_not_exist.xyz");
    let mut app = load_test_app();

    app.world_mut()
        .send_event(LoadFileEvent { path: path.clone() });
    app.update();

    let sim = app.world().resource::<SimulationData>();
    assert!(!sim.loaded);

    let mut error_events = app.world_mut().resource_mut::<Events<FileLoadErrorEvent>>();
    let errors: Vec<_> = error_events.drain().collect();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].path, path);
}
