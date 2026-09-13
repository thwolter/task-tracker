use super::bind;
use crate::{AppActions, AppWindow, Page, Range, application::Tracker, domain, language::Language};
use slint::{ComponentHandle, Model};
use std::{path::PathBuf, thread, time::Duration};

fn test_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "tempo-ui-{name}-{}-{}.sqlite",
        std::process::id(),
        domain::now()
    ))
}

#[test]
fn navigation_command_changes_the_controller_owned_page() {
    i_slint_backend_testing::init_no_event_loop();
    let path = test_path("navigate");
    let ui = AppWindow::new().unwrap();
    assert!(slint::select_bundled_translation("de").is_ok());
    bind(&ui, Tracker::at(path.clone()), Language::English);
    let actions = ui.global::<AppActions>();
    actions.invoke_navigate(Page::Settings);
    assert_eq!(ui.get_current_page(), Page::Settings);
    actions.invoke_navigate(Page::Home);
    assert_eq!(ui.get_current_page(), Page::Home);

    std::fs::remove_file(path).unwrap();
}

#[test]
fn add_project_command_refreshes_the_settings_projection() {
    i_slint_backend_testing::init_no_event_loop();
    let path = test_path("project-editor");
    let ui = AppWindow::new().unwrap();
    bind(&ui, Tracker::at(path.clone()), Language::English);
    let actions = ui.global::<AppActions>();

    actions.invoke_navigate(Page::Settings);
    actions.invoke_add_project("Focus work".into());
    assert_eq!(ui.get_current_page(), Page::Settings);
    assert_eq!(ui.get_project_settings().active.row_count(), 5);

    std::fs::remove_file(path).unwrap();
}

#[test]
fn save_project_command_renames_the_project_and_refreshes_settings() {
    i_slint_backend_testing::init_no_event_loop();
    let path = test_path("save-project");
    let ui = AppWindow::new().unwrap();
    bind(&ui, Tracker::at(path.clone()), Language::English);
    let actions = ui.global::<AppActions>();

    actions.invoke_save_project("project-1".into(), "Focus work".into());
    assert_eq!(
        ui.get_project_settings().active.row_data(0).unwrap().name,
        "Focus work"
    );

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
    actions.invoke_archive_project("project-1".into());
    assert_eq!(ui.get_home().projects.row_count(), 3);
    assert_eq!(ui.get_project_settings().archived.row_count(), 1);

    actions.invoke_unarchive_project("project-1".into());
    assert_eq!(ui.get_current_page(), Page::Settings);
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
        ui.get_evaluation_drilldown()
            .tasks
            .row_data(0)
            .unwrap()
            .note,
        "Updated note"
    );

    actions.invoke_delete_evaluation_task(task_id.into());
    assert_eq!(ui.get_evaluation_drilldown().tasks.row_count(), 0);

    std::fs::remove_file(path).unwrap();
}

#[test]
fn export_commands_capture_the_evaluation_scope_and_return_page() {
    i_slint_backend_testing::init_no_event_loop();
    let path = test_path("export-scope");
    let ui = AppWindow::new().unwrap();
    let mut tracker = Tracker::at(path.clone());
    let now = domain::now();
    tracker.start_tracking("project-1".into(), now - 2).unwrap();
    tracker.end_tracking(now - 1).unwrap();
    bind(&ui, tracker, Language::English);
    let actions = ui.global::<AppActions>();

    actions.invoke_navigate(Page::Evaluation);
    actions.invoke_open_export();
    assert_eq!(ui.get_current_page(), Page::Exportpage);
    assert!(ui.get_export_state().all_projects);
    assert_eq!(ui.get_export_state().return_page, Page::Evaluation);
    actions.invoke_navigate(ui.get_export_state().return_page);
    assert_eq!(ui.get_current_page(), Page::Evaluation);

    actions.invoke_open_drilldown("project-1".into());
    actions.invoke_open_export();
    assert_eq!(ui.get_current_page(), Page::Exportpage);
    assert!(!ui.get_export_state().all_projects);
    assert_eq!(ui.get_export_state().project, "Project Atlas");
    assert_eq!(ui.get_export_state().return_page, Page::Drilldown);
    actions.invoke_navigate(ui.get_export_state().return_page);
    assert_eq!(ui.get_current_page(), Page::Drilldown);

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
    assert_eq!(ui.get_current_page(), Page::Tracking);
    assert_eq!(ui.get_tracking().active_task, "Project Atlas");

    actions.invoke_toggle_tracking_pause();
    assert!(ui.get_tracking().paused);
    actions.invoke_tick();

    actions.invoke_end_tracking();
    assert_eq!(ui.get_current_page(), Page::Note);
    assert!(ui.get_home().has_last_task);

    actions.invoke_save_task_note("Finished the review".into());
    assert_eq!(ui.get_current_page(), Page::Home);
    assert_eq!(ui.get_home().last_task.note, "Finished the review");

    std::fs::remove_file(path).unwrap();
}

#[test]
fn active_tracking_remains_available_after_returning_home() {
    i_slint_backend_testing::init_no_event_loop();
    let path = test_path("tracking-home");
    let ui = AppWindow::new().unwrap();
    bind(&ui, Tracker::at(path.clone()), Language::English);
    let actions = ui.global::<AppActions>();

    actions.invoke_start_tracking("project-1".into());
    actions.invoke_navigate(Page::Home);
    let elapsed_before_tick = ui.get_tracking().elapsed;
    thread::sleep(Duration::from_millis(1_100));
    actions.invoke_tick();

    assert_eq!(ui.get_current_page(), Page::Home);
    assert_eq!(ui.get_tracking().active_task, "Project Atlas");
    assert_ne!(ui.get_tracking().elapsed, elapsed_before_tick);

    std::fs::remove_file(path).unwrap();
}

#[test]
fn secondary_tracking_locks_home_until_it_finishes_and_restores_the_primary() {
    i_slint_backend_testing::init_no_event_loop();
    let path = test_path("secondary-tracking");
    let ui = AppWindow::new().unwrap();
    bind(&ui, Tracker::at(path.clone()), Language::English);
    let actions = ui.global::<AppActions>();

    actions.invoke_start_tracking("project-1".into());
    actions.invoke_navigate(Page::Home);
    actions.invoke_start_tracking("project-2".into());
    assert_eq!(ui.get_tracking().active_task, "Admin");
    assert!(ui.get_tracking().secondary_active);

    actions.invoke_navigate(Page::Home);
    assert_eq!(ui.get_current_page(), Page::Tracking);

    actions.invoke_end_tracking();
    assert_eq!(ui.get_current_page(), Page::Note);
    assert_eq!(ui.get_tracking().active_task, "Project Atlas");
    assert!(ui.get_tracking().paused);
    assert!(!ui.get_tracking().secondary_active);

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

    assert!(!ui.get_transient_status().message.is_empty());
    i_slint_backend_testing::mock_elapsed_time(Duration::from_secs(3));
    assert!(ui.get_transient_status().message.is_empty());

    std::fs::remove_file(blocked_parent).unwrap();
}

#[test]
fn project_actions_create_edit_restore_and_delete_archived_project() {
    use crate::ProjectEditorMode;
    i_slint_backend_testing::init_no_event_loop();
    let path = test_path("project-controls");
    let ui = AppWindow::new().unwrap();
    bind(&ui, Tracker::at(path.clone()), Language::English);
    let actions = ui.global::<AppActions>();

    actions.invoke_archive_project("project-1".into());
    assert_eq!(ui.get_project_settings().archived.row_count(), 1);
    actions.invoke_open_project_edit("project-1".into());
    assert_eq!(ui.get_current_page(), Page::ProjectEditor);
    assert_eq!(ui.get_project_editor().mode, ProjectEditorMode::Edit);
    assert!(ui.get_project_editor().project.archived);

    actions.invoke_open_project_create();
    assert_eq!(ui.get_project_editor().mode, ProjectEditorMode::Create);
    actions.invoke_add_project("Focus work".into());
    assert_eq!(ui.get_project_settings().active.row_count(), 4);

    let created = (0..ui.get_project_settings().active.row_count())
        .map(|index| ui.get_project_settings().active.row_data(index).unwrap())
        .find(|project| project.name == "Focus work")
        .expect("new project should appear in the active-project projection");
    actions.invoke_save_project(created.id.clone(), "Renamed focus work".into());
    assert!(
        (0..ui.get_project_settings().active.row_count())
            .map(|index| ui.get_project_settings().active.row_data(index).unwrap())
            .any(|project| project.name == "Renamed focus work")
    );

    actions.invoke_archive_project(created.id.clone());
    assert_eq!(ui.get_project_settings().archived.row_count(), 2);
    actions.invoke_unarchive_project(created.id);
    assert_eq!(ui.get_project_settings().archived.row_count(), 1);
    assert_eq!(ui.get_project_settings().active.row_count(), 4);

    actions.invoke_delete_project("project-1".into());
    assert_eq!(ui.get_current_page(), Page::Settings);
    assert_eq!(ui.get_project_settings().archived.row_count(), 0);
    std::fs::remove_file(path).unwrap();
}

#[cfg(debug_assertions)]
#[test]
fn invalid_project_name_keeps_editor_and_navigation_returns_to_settings() {
    use crate::ProjectEditorMode;
    i_slint_backend_testing::init_no_event_loop();
    let path = test_path("invalid-project-editor");
    let ui = AppWindow::new().unwrap();
    slint::select_bundled_translation("en").unwrap();
    bind(&ui, Tracker::at(path.clone()), Language::English);
    let actions = ui.global::<AppActions>();
    actions.invoke_open_project_create();
    let mut editor = ui.get_project_editor();
    editor.draft_name = "   ".into();
    ui.set_project_editor(editor);
    actions.invoke_add_project("   ".into());

    assert_eq!(ui.get_project_editor().mode, ProjectEditorMode::Create);
    assert_eq!(ui.get_project_editor().draft_name, "   ");
    assert!(!ui.get_project_editor().error.is_empty());
    assert_eq!(ui.get_project_settings().active.row_count(), 4);

    actions.invoke_navigate(Page::Settings);
    assert_eq!(ui.get_current_page(), Page::Settings);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn failed_project_save_preserves_draft_and_can_be_retried() {
    use crate::ProjectEditorMode;
    i_slint_backend_testing::init_no_event_loop();
    let blocked_parent = test_path("project-retry-parent");
    std::fs::write(&blocked_parent, "not a directory").unwrap();
    let path = blocked_parent.join("tempo.sqlite");
    let ui = AppWindow::new().unwrap();
    bind(&ui, Tracker::at(path.clone()), Language::English);
    let actions = ui.global::<AppActions>();
    actions.invoke_open_project_create();
    let mut editor = ui.get_project_editor();
    editor.draft_name = "Retry me".into();
    ui.set_project_editor(editor);
    actions.invoke_add_project("Retry me".into());
    assert_eq!(ui.get_project_editor().mode, ProjectEditorMode::Create);
    assert_eq!(ui.get_project_editor().draft_name, "Retry me");
    assert!(!ui.get_project_editor().error.is_empty());
    assert_eq!(ui.get_project_settings().active.row_count(), 4);

    std::fs::remove_file(&blocked_parent).unwrap();
    std::fs::create_dir(&blocked_parent).unwrap();
    actions.invoke_add_project("Retry me".into());
    assert_eq!(ui.get_current_page(), Page::Settings);
    assert_eq!(ui.get_project_settings().active.row_count(), 5);
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(blocked_parent).unwrap();
}
