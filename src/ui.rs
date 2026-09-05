use crate::{
    AppWindow, Page, ProjectDialogState, Status, StatusKind,
    application::{ProjectDialog, SharedTracker},
    domain, persistence, presentation,
};
use rfd::FileDialog;
use slint::{ComponentHandle, SharedString, Timer, TimerMode, Weak};
use std::time::Duration;

pub(crate) fn bind(ui: &AppWindow, tracker: SharedTracker) -> Timer {
    let controller = UiController::new(ui, tracker);
    controller.refresh(ui);
    controller.persist_initial(ui);
    let timer = controller.start_timer();
    bind_callbacks(ui, &controller);
    timer
}

#[derive(Clone)]
struct UiController {
    ui: Weak<AppWindow>,
    tracker: SharedTracker,
}

impl UiController {
    fn new(ui: &AppWindow, tracker: SharedTracker) -> Self {
        Self {
            ui: ui.as_weak(),
            tracker,
        }
    }

    fn start_timer(&self) -> Timer {
        let timer = Timer::default();
        let controller = self.clone();
        timer.start(TimerMode::Repeated, Duration::from_secs(1), move || {
            controller.tick();
        });
        timer
    }

    fn tick(&self) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        let result = self.tracker.borrow_mut().tick(domain::now());
        if let Err(error) = result {
            self.set_error(&ui, format!("Could not save data: {error}"));
        }
        self.refresh(&ui);
    }

    fn start_tracking(&self, project_id: SharedString) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        let result = self
            .tracker
            .borrow_mut()
            .start_tracking(project_id.to_string(), domain::now());
        if let Err(error) = result {
            self.set_error(&ui, format!("Could not save data: {error}"));
        }
        ui.set_page(Page::Tracking);
        self.refresh(&ui);
    }

    fn open_last_task(&self) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        if self.tracker.borrow().data().has_tasks() {
            ui.set_page(Page::Note);
        }
    }

    fn end_tracking(&self) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        match self.tracker.borrow_mut().end_tracking(domain::now()) {
            Ok(true) => ui.set_page(Page::Note),
            Ok(false) => {}
            Err(error) => self.set_error(&ui, format!("Could not save data: {error}")),
        }
        self.refresh(&ui);
    }

    fn save_task_note(&self, note: SharedString) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        let result = self.tracker.borrow_mut().save_task_note(note.to_string());
        if let Err(error) = result {
            self.set_error(&ui, format!("Could not save data: {error}"));
        }
        ui.set_page(Page::Home);
        self.refresh(&ui);
    }

    fn choose_range(&self, range: crate::Range) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        self.tracker
            .borrow_mut()
            .choose_range(presentation::domain_range(range));
        self.refresh(&ui);
    }

    fn open_add_project(&self) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        let dialog = self.tracker.borrow_mut().begin_add_project();
        self.show_project_dialog(&ui, dialog);
    }

    fn open_rename_project(&self, id: SharedString) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        let dialog = self
            .tracker
            .borrow_mut()
            .begin_rename_project(id.to_string());
        self.show_project_dialog(&ui, dialog);
    }

    fn save_project(&self, name: SharedString) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        let result = self
            .tracker
            .borrow_mut()
            .save_project(name.as_str(), domain::now());
        match result {
            Ok(()) => {
                self.dismiss_project_dialog(&ui);
                self.refresh(&ui);
            }
            Err(error) => self.set_error(&ui, error.to_string()),
        }
    }

    fn close_project_dialog(&self) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        self.tracker.borrow_mut().cancel_project_dialog();
        self.dismiss_project_dialog(&ui);
    }

    fn archive_project(&self, id: SharedString) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        let result = self.tracker.borrow_mut().archive_project(id.to_string());
        match result {
            Ok(()) => {
                self.dismiss_project_dialog(&ui);
                self.refresh(&ui);
            }
            Err(error) => self.set_error(&ui, format!("Could not save data: {error}")),
        }
    }

    fn unarchive_project(&self, id: SharedString) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        let result = self.tracker.borrow_mut().unarchive_project(id.to_string());
        match result {
            Ok(()) => {
                self.dismiss_project_dialog(&ui);
                self.refresh(&ui);
            }
            Err(error) => self.set_error(&ui, format!("Could not save data: {error}")),
        }
    }

    fn export_markdown(&self) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        let Some(path) = FileDialog::new()
            .add_filter("Markdown", &["md"])
            .set_file_name("tempo-report.md")
            .save_file()
        else {
            return;
        };
        let report = self.tracker.borrow().report(domain::now());
        match persistence::export_markdown(&path, &report) {
            Ok(()) => self.set_success(&ui, "Markdown exported"),
            Err(error) => self.set_error(&ui, format!("Export failed: {error}")),
        }
    }

    fn refresh(&self, ui: &AppWindow) {
        presentation::refresh(ui, &self.tracker.borrow(), domain::now());
    }

    fn persist_initial(&self, ui: &AppWindow) {
        if let Err(error) = self.tracker.borrow().save() {
            self.set_error(ui, format!("Could not save data: {error}"));
        }
    }

    fn set_success(&self, ui: &AppWindow, message: impl Into<SharedString>) {
        self.set_status(ui, message, StatusKind::Success);
    }

    fn set_error(&self, ui: &AppWindow, message: impl Into<SharedString>) {
        self.set_status(ui, message, StatusKind::Error);
    }

    fn set_status(&self, ui: &AppWindow, message: impl Into<SharedString>, kind: StatusKind) {
        ui.set_status(Status {
            message: message.into(),
            kind,
        });
    }

    fn show_project_dialog(&self, ui: &AppWindow, dialog: ProjectDialog) {
        ui.set_project_dialog(ProjectDialogState {
            rename_mode: dialog.rename_mode,
            initial_draft: dialog.initial_draft.into(),
            project_id: dialog.project_id.into(),
            archived: dialog.archived,
        });
        ui.set_page(Page::ProjectEditor);
    }

    fn dismiss_project_dialog(&self, ui: &AppWindow) {
        ui.set_project_dialog(ProjectDialogState {
            rename_mode: false,
            initial_draft: SharedString::new(),
            project_id: SharedString::new(),
            archived: false,
        });
        ui.set_page(Page::Settings);
    }
}

fn bind_callbacks(ui: &AppWindow, controller: &UiController) {
    let start_tracking = controller.clone();
    ui.on_start_tracking(move |project_id| start_tracking.start_tracking(project_id));

    let open_last_task = controller.clone();
    ui.on_open_last_task(move || open_last_task.open_last_task());

    let end_tracking = controller.clone();
    ui.on_end_tracking(move || end_tracking.end_tracking());

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
}

#[cfg(test)]
mod tests {
    use super::bind;
    use crate::{AppWindow, Page, application::Tracker};
    use std::{cell::RefCell, rc::Rc};

    #[test]
    fn settings_home_callback_returns_to_home() {
        i_slint_backend_testing::init_no_event_loop();
        let ui = AppWindow::new().unwrap();
        ui.set_page(Page::Settings);
        ui.invoke_open_home_view();
        assert_eq!(ui.get_page(), Page::Home);
    }

    #[test]
    fn project_editor_is_an_exclusive_page_and_returns_to_settings() {
        i_slint_backend_testing::init_no_event_loop();
        let path = std::env::temp_dir().join(format!(
            "tempo-ui-test-{}-{}.json",
            std::process::id(),
            crate::domain::now()
        ));
        let ui = AppWindow::new().unwrap();
        let tracker = Rc::new(RefCell::new(Tracker::at(path.clone())));
        let _timer = bind(&ui, tracker);

        ui.invoke_open_add_project();
        assert_eq!(ui.get_page(), Page::ProjectEditor);

        ui.invoke_close_project_dialog();
        assert_eq!(ui.get_page(), Page::Settings);

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn unarchive_project_callback_restores_project_to_home() {
        i_slint_backend_testing::init_no_event_loop();
        let path = std::env::temp_dir().join(format!(
            "tempo-ui-unarchive-test-{}-{}.json",
            std::process::id(),
            crate::domain::now()
        ));
        let ui = AppWindow::new().unwrap();
        let tracker = Rc::new(RefCell::new(Tracker::at(path.clone())));
        let _timer = bind(&ui, tracker.clone());

        ui.invoke_open_rename_project("project-1".into());
        ui.invoke_archive_project("project-1".into());
        assert!(tracker.borrow().data().projects()[0].archived());

        ui.invoke_open_rename_project("project-1".into());
        assert!(ui.get_project_dialog().archived);

        ui.invoke_unarchive_project("project-1".into());

        assert_eq!(ui.get_page(), Page::Settings);
        assert!(!tracker.borrow().data().projects()[0].archived());

        std::fs::remove_file(path).unwrap();
    }
}
