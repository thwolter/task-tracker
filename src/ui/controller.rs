use crate::application::{ProjectDialog, Tracker};
use crate::language::Language;
use crate::{
    AppActions, AppWindow, Page, ProjectDialogState, Status, StatusKind, UiCommand, UiCommandKind,
    domain, persistence, presentation,
};
use rfd::FileDialog;
use slint::{ComponentHandle, SharedString, Timer, TimerMode, Weak};
use std::time::Duration;

const STATUS_DURATION: Duration = Duration::from_secs(3);

pub(crate) fn bind(ui: &AppWindow, tracker: Tracker, language: Language) {
    let mut controller = UiController::new(ui, tracker, language);
    controller.refresh(ui);
    controller.persist_initial(ui);

    ui.global::<AppActions>()
        .on_dispatch(move |command| controller.handle(command));
}

struct UiController {
    ui: Weak<AppWindow>,
    tracker: Tracker,
    language: Language,
    status_timer: Timer,
}

impl UiController {
    fn new(ui: &AppWindow, tracker: Tracker, language: Language) -> Self {
        Self {
            ui: ui.as_weak(),
            tracker,
            language,
            status_timer: Timer::default(),
        }
    }

    fn handle(&mut self, command: UiCommand) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };

        match command.kind {
            UiCommandKind::Tick => self.tick(&ui),
            UiCommandKind::StartTracking => self.start_tracking(&ui, command.id),
            UiCommandKind::OpenLastTask => self.open_last_task(&ui),
            UiCommandKind::EndTracking => self.end_tracking(&ui),
            UiCommandKind::ToggleTrackingPause => self.toggle_tracking_pause(&ui),
            UiCommandKind::SaveTaskNote => self.save_task_note(&ui, command.text),
            UiCommandKind::UpdateEvaluationTask => {
                self.update_evaluation_task(&ui, command.id, command.text)
            }
            UiCommandKind::DeleteEvaluationTask => self.delete_evaluation_task(&ui, command.id),
            UiCommandKind::ChooseRange => self.choose_range(&ui, command.range),
            UiCommandKind::OpenAddProject => self.open_add_project(&ui),
            UiCommandKind::OpenRenameProject => self.open_rename_project(&ui, command.id),
            UiCommandKind::SaveProject => self.save_project(&ui, command.text),
            UiCommandKind::CloseProjectDialog => self.close_project_dialog(&ui),
            UiCommandKind::ArchiveProject => self.archive_project(&ui, command.id),
            UiCommandKind::UnarchiveProject => self.unarchive_project(&ui, command.id),
            UiCommandKind::DeleteProject => self.delete_project(&ui, command.id),
            UiCommandKind::ExportMarkdown => self.export_markdown(&ui),
            UiCommandKind::OpenDrilldown => self.open_drilldown(&ui, command.id),
        }
    }

    fn tick(&mut self, ui: &AppWindow) {
        if let Err(error) = self.tracker.tick(domain::now()) {
            self.set_error(ui, format!("Could not save data: {error}"));
        }
        // Only the tracking view displays data that changes on every timer tick.
        // Refreshing every projection recreates inactive views and steals focus
        // from their text inputs.
        if ui.get_page() == Page::Tracking {
            self.refresh(ui);
        }
    }

    fn start_tracking(&mut self, ui: &AppWindow, project_id: SharedString) {
        if let Err(error) = self
            .tracker
            .start_tracking(project_id.to_string(), domain::now())
        {
            self.set_error(ui, format!("Could not save data: {error}"));
        }
        ui.set_page(Page::Tracking);
        self.refresh(ui);
    }

    fn open_last_task(&mut self, ui: &AppWindow) {
        if self.tracker.data().has_tasks() {
            ui.set_page(Page::Note);
        }
    }

    fn end_tracking(&mut self, ui: &AppWindow) {
        match self.tracker.end_tracking(domain::now()) {
            Ok(true) => ui.set_page(Page::Note),
            Ok(false) => {}
            Err(error) => self.set_error(ui, format!("Could not save data: {error}")),
        }
        self.refresh(ui);
    }

    fn toggle_tracking_pause(&mut self, ui: &AppWindow) {
        if let Err(error) = self.tracker.toggle_tracking_pause(domain::now()) {
            self.set_error(ui, format!("Could not save data: {error}"));
        }
        self.refresh(ui);
    }

    fn save_task_note(&mut self, ui: &AppWindow, note: SharedString) {
        if let Err(error) = self.tracker.save_task_note(note.to_string()) {
            self.set_error(ui, format!("Could not save data: {error}"));
        }
        ui.set_page(Page::Home);
        self.refresh(ui);
    }

    fn update_evaluation_task(&mut self, ui: &AppWindow, id: SharedString, note: SharedString) {
        match self
            .tracker
            .update_task_note(id.to_string(), note.to_string())
        {
            Ok(true) => self.refresh(ui),
            Ok(false) => self.set_error(ui, "The session no longer exists"),
            Err(error) => self.set_error(ui, format!("Could not save data: {error}")),
        }
    }

    fn delete_evaluation_task(&mut self, ui: &AppWindow, id: SharedString) {
        match self.tracker.delete_task(id.to_string()) {
            Ok(true) => self.refresh(ui),
            Ok(false) => self.set_error(ui, "The session no longer exists"),
            Err(error) => self.set_error(ui, format!("Could not save data: {error}")),
        }
    }

    fn choose_range(&mut self, ui: &AppWindow, range: crate::Range) {
        self.tracker.choose_range(presentation::domain_range(range));
        self.refresh(ui);
    }

    fn open_add_project(&mut self, ui: &AppWindow) {
        let dialog = self.tracker.begin_add_project();
        self.show_project_dialog(ui, dialog);
    }

    fn open_rename_project(&mut self, ui: &AppWindow, id: SharedString) {
        let dialog = self.tracker.begin_rename_project(id.to_string());
        self.show_project_dialog(ui, dialog);
    }

    fn save_project(&mut self, ui: &AppWindow, name: SharedString) {
        match self.tracker.save_project(name.as_str(), domain::now()) {
            Ok(()) => {
                self.dismiss_project_dialog(ui);
                self.refresh(ui);
            }
            Err(error) => self.set_error(ui, error.to_string()),
        }
    }

    fn close_project_dialog(&mut self, ui: &AppWindow) {
        self.tracker.cancel_project_dialog();
        self.dismiss_project_dialog(ui);
    }

    fn archive_project(&mut self, ui: &AppWindow, id: SharedString) {
        match self.tracker.archive_project(id.to_string()) {
            Ok(()) => {
                self.dismiss_project_dialog(ui);
                self.refresh(ui);
            }
            Err(error) => self.set_error(ui, format!("Could not save data: {error}")),
        }
    }

    fn unarchive_project(&mut self, ui: &AppWindow, id: SharedString) {
        match self.tracker.unarchive_project(id.to_string()) {
            Ok(()) => {
                self.dismiss_project_dialog(ui);
                self.refresh(ui);
            }
            Err(error) => self.set_error(ui, format!("Could not save data: {error}")),
        }
    }

    fn delete_project(&mut self, ui: &AppWindow, id: SharedString) {
        match self.tracker.delete_project(id.to_string()) {
            Ok(()) => {
                self.dismiss_project_dialog(ui);
                self.refresh(ui);
            }
            Err(error) => self.set_error(ui, format!("Could not save data: {error}")),
        }
    }

    fn export_markdown(&mut self, ui: &AppWindow) {
        let Some(path) = FileDialog::new()
            .add_filter("Markdown", &["md"])
            .set_file_name("tempo-report.md")
            .save_file()
        else {
            return;
        };
        let report = self.tracker.report(domain::now(), self.language);
        match persistence::export_markdown(&path, &report) {
            Ok(()) => self.set_success(ui, "Markdown exported"),
            Err(error) => self.set_error(ui, format!("Export failed: {error}")),
        }
    }

    fn open_drilldown(&mut self, ui: &AppWindow, id: SharedString) {
        self.tracker.select_evaluation_project(id.to_string());
        self.refresh(ui);
        ui.set_page(Page::Drilldown);
    }

    fn refresh(&self, ui: &AppWindow) {
        presentation::refresh(ui, &self.tracker, domain::now(), self.language);
    }

    fn persist_initial(&self, ui: &AppWindow) {
        if let Err(error) = self.tracker.save() {
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
