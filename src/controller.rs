//! UI controller connecting Slint user interface events to application logic.
//!
//! This module owns the application-facing UI state: navigation, projected data refreshes,
//! persistence feedback, and command dispatch. Feature-specific command handlers live in
//! private child modules while sharing this single controller and its state.

mod data;
mod evaluation;
mod projects;
#[cfg(test)]
mod tests;
mod tracking;

use crate::application::Tracker;
use crate::domain::ProjectId;
use crate::language::Language;
use crate::{
    AppActions, AppWindow, Page, Status, StatusKind, UiCommand, UiCommandKind, domain, presentation,
};
use slint::{ComponentHandle, SharedString, Timer, TimerMode, Weak};
use std::{cell::RefCell, rc::Rc, time::Duration};

const STATUS_DURATION: Duration = Duration::from_secs(3);

/// Connects the Slint UI to the application tracker and begins dispatching commands.
pub(crate) fn bind(ui: &AppWindow, tracker: Tracker, language: Language) {
    let controller = Rc::new(RefCell::new(UiController::new(ui, tracker, language)));
    controller.borrow().refresh(ui);
    controller.borrow().persist_initial(ui);
    controller.borrow().open_first_run_if_needed(ui);

    #[cfg(all(target_os = "macos", not(test)))]
    {
        crate::macos_menu::install(ui);
    }

    let actions = ui.global::<AppActions>();
    let dispatch_controller = controller.clone();
    actions.on_dispatch(move |command| dispatch_controller.borrow_mut().handle(command));
    actions.on_project_name_exists(move |name| controller.borrow().project_name_exists(name));
}

/// State holder and coordinator for Slint UI events and presentation projections.
struct UiController {
    ui: Weak<AppWindow>,
    tracker: Tracker,
    language: Language,
    status_timer: Timer,
    export_context: Option<ExportContext>,
}

#[derive(Clone)]
struct ExportContext {
    project_id: Option<ProjectId>,
}

impl UiController {
    fn new(ui: &AppWindow, tracker: Tracker, language: Language) -> Self {
        Self {
            ui: ui.as_weak(),
            tracker,
            language,
            status_timer: Timer::default(),
            export_context: None,
        }
    }

    fn handle(&mut self, command: UiCommand) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };

        match command.kind {
            UiCommandKind::Tick => self.tick(&ui),
            UiCommandKind::Navigate => self.navigate(&ui, command.page),
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
            UiCommandKind::OpenProjectCreate => self.open_project_create(&ui),
            UiCommandKind::OpenProjectEdit => self.open_project_edit(&ui, command.id),
            UiCommandKind::AddProject => self.add_project(&ui, command.text),
            UiCommandKind::AddFirstRunProject => self.add_first_run_project(&ui, command.text),
            UiCommandKind::SaveProject => self.save_project(&ui, command.id, command.text),
            UiCommandKind::ArchiveProject => self.archive_project(&ui, command.id),
            UiCommandKind::UnarchiveProject => self.unarchive_project(&ui, command.id),
            UiCommandKind::DeleteProject => self.delete_project(&ui, command.id),
            UiCommandKind::OpenExport => self.open_export(&ui),
            UiCommandKind::ExportReport => self.export_report(&ui, command.export_format),
            UiCommandKind::OpenDrilldown => self.open_drilldown(&ui, command.id),
            UiCommandKind::BackupData => self.backup_data(&ui),
            UiCommandKind::RestoreData => self.restore_data(&ui),
            UiCommandKind::ShowKeyboardShortcuts => self.show_keyboard_shortcuts(&ui),
        }
    }

    /// Displays a simple user-requested destination. Workflow handlers select pages only
    /// after their application operation succeeds.
    fn navigate(&self, ui: &AppWindow, page: Page) {
        if page == Page::Home && self.tracker.data().has_interrupted_task() {
            self.set_error(ui, "Finish the interrupted task before returning home");
            return;
        }
        ui.set_current_page(page);
    }

    fn refresh(&self, ui: &AppWindow) {
        presentation::refresh(ui, &self.tracker, domain::now(), self.language);
        #[cfg(all(target_os = "macos", not(test)))]
        crate::macos_menu::refresh(ui);
    }

    fn persist_initial(&self, ui: &AppWindow) {
        if let Err(error) = self.tracker.save() {
            self.set_error(ui, format!("Could not save data: {error}"));
        }
    }

    fn open_first_run_if_needed(&self, ui: &AppWindow) {
        if self.tracker.data().projects().is_empty() {
            ui.set_current_page(Page::FirstRun);
        }
    }

    fn set_success(&self, ui: &AppWindow, message: impl Into<SharedString>) {
        self.set_status(ui, message, StatusKind::Success);
    }

    fn set_error(&self, ui: &AppWindow, message: impl Into<SharedString>) {
        self.set_status(ui, message, StatusKind::Error);
    }

    fn set_status(&self, ui: &AppWindow, message: impl Into<SharedString>, kind: StatusKind) {
        ui.set_transient_status(Status {
            message: message.into(),
            kind,
        });

        let ui = self.ui.clone();
        self.status_timer
            .start(TimerMode::SingleShot, STATUS_DURATION, move || {
                let Some(ui) = ui.upgrade() else {
                    return;
                };
                ui.set_transient_status(Status {
                    message: SharedString::new(),
                    kind: StatusKind::Success,
                });
            });
    }
}
