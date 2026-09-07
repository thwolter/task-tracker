use crate::AppWindow;
use crate::application::SharedTracker;
use crate::language::Language;
use crate::ui::controller::UiController;
use slint::Timer;

pub(crate) fn bind(ui: &AppWindow, tracker: SharedTracker, language: Language) -> Timer {
    let controller = UiController::new(ui, tracker, language);
    controller.refresh(ui);
    controller.persist_initial(ui);
    let timer = controller.start_timer();
    bind_callbacks(ui, &controller);
    timer
}

fn bind_callbacks(ui: &AppWindow, controller: &UiController) {
    let start_tracking = controller.clone();
    ui.on_start_tracking(move |project_id| start_tracking.start_tracking(project_id));

    let open_last_task = controller.clone();
    ui.on_open_last_task(move || open_last_task.open_last_task());

    let end_tracking = controller.clone();
    ui.on_end_tracking(move || end_tracking.end_tracking());

    let toggle_tracking_pause = controller.clone();
    ui.on_toggle_tracking_pause(move || toggle_tracking_pause.toggle_tracking_pause());

    let save_task_note = controller.clone();
    ui.on_save_task_note(move |note| save_task_note.save_task_note(note));

    let choose_range = controller.clone();
    ui.on_choose_range(move |range| choose_range.choose_range(range));

    let open_add_project = controller.clone();
    ui.on_open_add_project(move || open_add_project.open_add_project());

    let open_rename_project = controller.clone();
    ui.on_open_rename_project(move |id| open_rename_project.open_rename_project(id));

    let save_project = controller.clone();
    ui.on_save_project(move |name| save_project.save_project(name));

    let close_project_dialog = controller.clone();
    ui.on_close_project_dialog(move || close_project_dialog.close_project_dialog());

    let export_markdown = controller.clone();
    ui.on_export_markdown(move || export_markdown.export_markdown());

    let archive_project = controller.clone();
    ui.on_archive_project(move |id| archive_project.archive_project(id));

    let unarchive_project = controller.clone();
    ui.on_unarchive_project(move |id| unarchive_project.unarchive_project(id));

    let delete_project = controller.clone();
    ui.on_delete_project(move |id| delete_project.delete_project(id));
}
