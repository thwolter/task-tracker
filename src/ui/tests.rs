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
fn add_project_command_refreshes_the_settings_projection() {
    i_slint_backend_testing::init_no_event_loop();
    let path = test_path("project-editor");
    let ui = AppWindow::new().unwrap();
    bind(&ui, Tracker::at(path.clone()), Language::English);
    let actions = ui.global::<AppActions>();

    ui.set_page(Page::Settings);
    actions.invoke_add_project("Focus work".into());
    assert_eq!(ui.get_page(), Page::Settings);
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

// Invoke actual view controls, rather than bypassing the Slint action routing.
#[cfg(debug_assertions)]
fn activate(ui: &AppWindow, label: &str) {
    let control = i_slint_backend_testing::ElementHandle::find_by_accessible_label(ui, label)
        .next()
        .unwrap_or_else(|| panic!("missing control: {label}"));
    control.invoke_accessible_default_action();
}

#[cfg(debug_assertions)]
#[test]
fn project_editor_creates_after_archived_edit_and_confirms_deletion() {
    use crate::ProjectEditorMode;
    i_slint_backend_testing::init_no_event_loop();
    let path = test_path("project-controls");
    let ui = AppWindow::new().unwrap();
    slint::select_bundled_translation("en").unwrap();
    bind(&ui, Tracker::at(path.clone()), Language::English);
    ui.set_page(Page::Settings);
    ui.global::<AppActions>()
        .invoke_archive_project("project-1".into());
    activate(&ui, "Archived (1)");
    let archived = ui.get_project_settings().archived.row_data(0).unwrap();
    activate(&ui, &format!("{} · Archived", archived.name));
    activate(&ui, "Back to projects");
    activate(&ui, "Add project");
    assert_eq!(ui.get_project_editor().mode, ProjectEditorMode::Create);
    assert!(ui.get_project_editor().project.id.is_empty());
    assert!(ui.get_project_editor().draft_name.is_empty());
    type_text(&ui, "Focus work");
    assert_eq!(ui.get_project_editor().draft_name, "Focus work");
    type_text(
        &ui,
        &slint::SharedString::from(slint::platform::Key::Return),
    );
    assert_eq!(ui.get_project_editor().mode, ProjectEditorMode::List);
    assert_eq!(ui.get_project_settings().active.row_count(), 4);
    assert_eq!(
        ui.get_project_settings().archived.row_data(0).unwrap().name,
        archived.name
    );

    activate(&ui, "Focus work");
    let mut editor = ui.get_project_editor();
    editor.draft_name = "Renamed focus work".into();
    ui.set_project_editor(editor);
    activate(&ui, "Save");
    assert_eq!(ui.get_project_editor().mode, ProjectEditorMode::List);
    activate(&ui, "Renamed focus work");
    activate(&ui, "Archive project");
    assert_eq!(ui.get_project_settings().archived.row_count(), 2);
    activate(&ui, "Archived (2)");
    activate(&ui, "Renamed focus work · Archived");
    activate(&ui, "Restore project");
    assert_eq!(ui.get_project_settings().archived.row_count(), 1);
    assert_eq!(ui.get_project_settings().active.row_count(), 4);

    activate(&ui, "Archived (1)");
    activate(&ui, &format!("{} · Archived", archived.name));
    activate(&ui, "Delete permanently");
    assert_eq!(
        ui.get_project_editor().mode,
        ProjectEditorMode::ConfirmDelete
    );
    assert_eq!(ui.get_project_settings().archived.row_count(), 1);
    type_text(
        &ui,
        &slint::SharedString::from(slint::platform::Key::Escape),
    );
    assert_eq!(ui.get_project_editor().mode, ProjectEditorMode::Edit);
    activate(&ui, "Delete permanently");
    activate(&ui, "Delete permanently");
    assert_eq!(ui.get_project_editor().mode, ProjectEditorMode::List);
    assert_eq!(ui.get_project_settings().archived.row_count(), 0);
    std::fs::remove_file(path).unwrap();
}

#[cfg(debug_assertions)]
#[test]
fn invalid_project_name_keeps_editor_and_escape_cancels() {
    use crate::ProjectEditorMode;
    i_slint_backend_testing::init_no_event_loop();
    let path = test_path("invalid-project-editor");
    let ui = AppWindow::new().unwrap();
    slint::select_bundled_translation("en").unwrap();
    bind(&ui, Tracker::at(path.clone()), Language::English);
    ui.set_page(Page::Settings);
    activate(&ui, "Add project");
    type_text(&ui, "   ");
    type_text(
        &ui,
        &slint::SharedString::from(slint::platform::Key::Return),
    );
    assert_eq!(ui.get_project_editor().mode, ProjectEditorMode::Create);
    assert_eq!(ui.get_project_editor().draft_name, "   ");
    assert!(!ui.get_project_editor().error.is_empty());
    assert_eq!(ui.get_project_settings().active.row_count(), 4);
    type_text(
        &ui,
        &slint::SharedString::from(slint::platform::Key::Escape),
    );
    assert_eq!(ui.get_project_editor().mode, ProjectEditorMode::List);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn failed_project_save_preserves_draft_and_can_be_retried() {
    use crate::{ProjectEditorMode, ProjectEditorState};
    i_slint_backend_testing::init_no_event_loop();
    let blocked_parent = test_path("project-retry-parent");
    std::fs::write(&blocked_parent, "not a directory").unwrap();
    let path = blocked_parent.join("tempo.sqlite");
    let ui = AppWindow::new().unwrap();
    bind(&ui, Tracker::at(path.clone()), Language::English);
    ui.set_page(Page::Settings);
    ui.set_project_editor(ProjectEditorState {
        mode: ProjectEditorMode::Create,
        draft_name: "Retry me".into(),
        ..Default::default()
    });
    ui.global::<AppActions>()
        .invoke_add_project("Retry me".into());
    assert_eq!(ui.get_project_editor().mode, ProjectEditorMode::Create);
    assert_eq!(ui.get_project_editor().draft_name, "Retry me");
    assert!(!ui.get_project_editor().error.is_empty());
    assert_eq!(ui.get_project_settings().active.row_count(), 4);

    std::fs::remove_file(&blocked_parent).unwrap();
    std::fs::create_dir(&blocked_parent).unwrap();
    ui.global::<AppActions>()
        .invoke_add_project("Retry me".into());
    assert_eq!(ui.get_project_editor().mode, ProjectEditorMode::List);
    assert_eq!(ui.get_project_settings().active.row_count(), 5);
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(blocked_parent).unwrap();
}

#[cfg(debug_assertions)]
fn type_text(ui: &AppWindow, text: &str) {
    for ch in text.chars() {
        let text: slint::SharedString = ch.to_string().into();
        ui.window()
            .dispatch_event(slint::platform::WindowEvent::KeyPressed { text: text.clone() });
        ui.window()
            .dispatch_event(slint::platform::WindowEvent::KeyReleased { text });
    }
}
