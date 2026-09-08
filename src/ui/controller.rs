use crate::application::{ProjectDialog, SharedTracker};
use crate::{
    AppWindow, Page, ProjectDialogState, Status, StatusKind, domain, language::Language,
    persistence, presentation,
};
use rfd::FileDialog;
use slint::{ComponentHandle, SharedString, Timer, TimerMode, Weak};
use std::{rc::Rc, time::Duration};

const STATUS_DURATION: Duration = Duration::from_secs(3);

#[derive(Clone)]
pub(super) struct UiController {
    ui: Weak<AppWindow>,
    tracker: SharedTracker,
    language: Language,
    status_timer: Rc<Timer>,
}

impl UiController {
    pub(super) fn new(ui: &AppWindow, tracker: SharedTracker, language: Language) -> Self {
        Self {
            ui: ui.as_weak(),
            tracker,
            language,
            status_timer: Rc::new(Timer::default()),
        }
    }

    pub(super) fn start_timer(&self) -> Timer {
        let timer = Timer::default();
        let controller = self.clone();
        timer.start(TimerMode::Repeated, Duration::from_secs(1), move || {
            controller.tick();
        });
        timer
    }

    pub(super) fn tick(&self) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        let result = self.tracker.borrow_mut().tick(domain::now());
        if let Err(error) = result {
            self.set_error(&ui, format!("Could not save data: {error}"));
        }
        self.refresh(&ui);
    }

    pub(super) fn start_tracking(&self, project_id: SharedString) {
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

    pub(super) fn open_last_task(&self) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        if self.tracker.borrow().data().has_tasks() {
            ui.set_page(Page::Note);
        }
    }

    pub(super) fn end_tracking(&self) {
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

    pub(super) fn toggle_tracking_pause(&self) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        if let Err(error) = self
            .tracker
            .borrow_mut()
            .toggle_tracking_pause(domain::now())
        {
            self.set_error(&ui, format!("Could not save data: {error}"));
        }
        self.refresh(&ui);
    }

    pub(super) fn save_task_note(&self, note: SharedString) {
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

    pub(super) fn update_evaluation_task(&self, id: SharedString, note: SharedString) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        let result = {
            self.tracker
                .borrow_mut()
                .update_task_note(id.to_string(), note.to_string())
        };
        match result {
            Ok(true) => self.refresh(&ui),
            Ok(false) => self.set_error(&ui, "The session no longer exists"),
            Err(error) => self.set_error(&ui, format!("Could not save data: {error}")),
        }
    }

    pub(super) fn delete_evaluation_task(&self, id: SharedString) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        let result = { self.tracker.borrow_mut().delete_task(id.to_string()) };
        match result {
            Ok(true) => self.refresh(&ui),
            Ok(false) => self.set_error(&ui, "The session no longer exists"),
            Err(error) => self.set_error(&ui, format!("Could not save data: {error}")),
        }
    }

    pub(super) fn choose_range(&self, range: crate::Range) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        self.tracker
            .borrow_mut()
            .choose_range(presentation::domain_range(range));
        self.refresh(&ui);
    }

    pub(super) fn open_add_project(&self) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        let dialog = self.tracker.borrow_mut().begin_add_project();
        self.show_project_dialog(&ui, dialog);
    }

    pub(super) fn open_rename_project(&self, id: SharedString) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        let dialog = self
            .tracker
            .borrow_mut()
            .begin_rename_project(id.to_string());
        self.show_project_dialog(&ui, dialog);
    }

    pub(super) fn save_project(&self, name: SharedString) {
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

    pub(super) fn close_project_dialog(&self) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        self.tracker.borrow_mut().cancel_project_dialog();
        self.dismiss_project_dialog(&ui);
    }

    pub(super) fn archive_project(&self, id: SharedString) {
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

    pub(super) fn unarchive_project(&self, id: SharedString) {
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

    pub(super) fn delete_project(&self, id: SharedString) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        let result = self.tracker.borrow_mut().delete_project(id.to_string());
        match result {
            Ok(()) => {
                self.dismiss_project_dialog(&ui);
                self.refresh(&ui);
            }
            Err(error) => self.set_error(&ui, format!("Could not save data: {error}")),
        }
    }

    pub(super) fn export_markdown(&self) {
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
        let report = self.tracker.borrow().report(domain::now(), self.language);
        match persistence::export_markdown(&path, &report) {
            Ok(()) => self.set_success(&ui, "Markdown exported"),
            Err(error) => self.set_error(&ui, format!("Export failed: {error}")),
        }
    }

    pub(super) fn refresh(&self, ui: &AppWindow) {
        presentation::refresh(ui, &self.tracker.borrow(), domain::now(), self.language);
    }

    pub(super) fn persist_initial(&self, ui: &AppWindow) {
        if let Err(error) = self.tracker.borrow().save() {
            self.set_error(ui, format!("Could not save data: {error}"));
        }
    }

    pub(super) fn set_success(&self, ui: &AppWindow, message: impl Into<SharedString>) {
        self.set_status(ui, message, StatusKind::Success);
    }

    pub(super) fn set_error(&self, ui: &AppWindow, message: impl Into<SharedString>) {
        self.set_status(ui, message, StatusKind::Error);
    }

    pub(super) fn set_status(
        &self,
        ui: &AppWindow,
        message: impl Into<SharedString>,
        kind: StatusKind,
    ) {
        ui.set_status(Status {
            message: message.into(),
            kind,
        });

        let ui = self.ui.clone();
        self.status_timer
            .start(TimerMode::SingleShot, STATUS_DURATION, move || {
                let Some(ui) = ui.upgrade() else {
                    return;
                };
                ui.set_status(Status {
                    message: SharedString::new(),
                    kind: StatusKind::Success,
                });
            });
    }

    pub(super) fn show_project_dialog(&self, ui: &AppWindow, dialog: ProjectDialog) {
        ui.set_project_dialog(ProjectDialogState {
            rename_mode: dialog.rename_mode,
            initial_draft: dialog.initial_draft.into(),
            project_id: dialog.project_id.into(),
            archived: dialog.archived,
        });
        ui.set_page(Page::ProjectEditor);
    }

    pub(super) fn dismiss_project_dialog(&self, ui: &AppWindow) {
        ui.set_project_dialog(ProjectDialogState {
            rename_mode: false,
            initial_draft: SharedString::new(),
            project_id: SharedString::new(),
            archived: false,
        });
        ui.set_page(Page::Settings);
    }

    pub(super) fn open_evaluate_tasks(&self, id: SharedString) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        self.tracker
            .borrow_mut()
            .select_evaluation_project(id.to_string());
        self.refresh(&ui);
        ui.set_page(Page::EvaluationTasks);
    }
}
