use crate::ui::bind;
use crate::{AppActions, AppWindow, Page, Range, application::Tracker, domain, language::Language};
use slint::{ComponentHandle, Model};
use std::{path::PathBuf, time::Duration};

fn test_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "tempo-ui-{name}-{}-{}.sqlite",
        std::process::id(),
        domain::now()
    ))
}

#[test]
fn settings_home_callback_returns_to_home() {
    i_slint_backend_testing::init_no_event_loop();
    let ui = AppWindow::new().unwrap();
    assert!(slint::select_bundled_translation("de").is_ok());
    ui.set_page(Page::Settings);
    ui.invoke_open_home_view();
    assert_eq!(ui.get_page(), Page::Home);
}

#[test]
fn project_editor_is_an_exclusive_page_and_returns_to_settings() {
    i_slint_backend_testing::init_no_event_loop();
    let path = test_path("project-editor");
    let ui = AppWindow::new().unwrap();
    bind(&ui, Tracker::at(path.clone()), Language::English);
    let actions = ui.global::<AppActions>();

    actions.invoke_open_add_project();
    assert_eq!(ui.get_page(), Page::ProjectEditor);

    actions.invoke_close_project_dialog();
    assert_eq!(ui.get_page(), Page::Settings);

    std::fs::remove_file(path).unwrap();
}

#[test]
fn project_commands_archive_and_restore_the_home_projection() {
    i_slint_backend_testing::init_no_event_loop();
    let path = test_path("unarchive");
    let ui = AppWindow::new().unwrap();
    bind(&ui, Tracker::at(path.clone()), Language::English);
    let actions = ui.global::<AppActions>();

    assert_eq!(ui.get_home().projects.row_count(), 4);
    actions.invoke_open_rename_project("project-1".into());
    actions.invoke_archive_project("project-1".into());
    assert_eq!(ui.get_home().projects.row_count(), 3);

    actions.invoke_open_rename_project("project-1".into());
    assert!(ui.get_project_dialog().archived);

    actions.invoke_unarchive_project("project-1".into());
    assert_eq!(ui.get_page(), Page::Settings);
    assert_eq!(ui.get_home().projects.row_count(), 4);

    std::fs::remove_file(path).unwrap();
}

#[test]
fn evaluation_commands_change_range_update_and_delete_tasks() {
    i_slint_backend_testing::init_no_event_loop();
    let path = test_path("evaluation-task");
    let ui = AppWindow::new().unwrap();
    let mut tracker = Tracker::at(path.clone());
    let now = domain::now();
    tracker.start_tracking("project-1".into(), now - 2).unwrap();
    tracker.end_tracking(now - 1).unwrap();
    let task_id = tracker.data().tasks()[0].id().as_str().to_owned();
    bind(&ui, tracker, Language::English);
    let actions = ui.global::<AppActions>();

    actions.invoke_choose_range(Range::Year);
    assert_eq!(ui.get_evaluation().range, Range::Year);

    actions.invoke_open_drilldown("project-1".into());
    actions.invoke_update_evaluation_task(task_id.clone().into(), "Updated note".into());
    assert_eq!(
        ui.get_drilldown().tasks.row_data(0).unwrap().note,
        "Updated note"
    );

    actions.invoke_delete_evaluation_task(task_id.into());
    assert_eq!(ui.get_drilldown().tasks.row_count(), 0);

    std::fs::remove_file(path).unwrap();
}

#[test]
fn tracking_commands_refresh_the_ui_through_the_single_dispatcher() {
    i_slint_backend_testing::init_no_event_loop();
    let path = test_path("tracking");
    let ui = AppWindow::new().unwrap();
    bind(&ui, Tracker::at(path.clone()), Language::English);
    let actions = ui.global::<AppActions>();

    actions.invoke_start_tracking("project-1".into());
    assert_eq!(ui.get_page(), Page::Tracking);
    assert_eq!(ui.get_tracking().active_task, "PROJECT ATLAS");

    actions.invoke_toggle_tracking_pause();
    assert!(ui.get_tracking().paused);
    actions.invoke_tick();

    actions.invoke_end_tracking();
    assert_eq!(ui.get_page(), Page::Note);
    assert!(ui.get_home().has_last_task);

    actions.invoke_save_task_note("Finished the review".into());
    assert_eq!(ui.get_page(), Page::Home);
    assert_eq!(ui.get_home().last_task.note, "Finished the review");

    std::fs::remove_file(path).unwrap();
}

#[test]
fn persistence_errors_are_shown_and_status_expires() {
    i_slint_backend_testing::init_no_event_loop();
    let blocked_parent = test_path("blocked-parent");
    std::fs::write(&blocked_parent, "not a directory").unwrap();
    let path = blocked_parent.join("tempo.sqlite");
    let ui = AppWindow::new().unwrap();
    bind(&ui, Tracker::at(path), Language::English);

    assert!(!ui.get_status().message.is_empty());
    i_slint_backend_testing::mock_elapsed_time(Duration::from_secs(3));
    assert!(ui.get_status().message.is_empty());

    std::fs::remove_file(blocked_parent).unwrap();
}
