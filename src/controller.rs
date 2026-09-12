//! UI controller connecting Slint user interface events to application logic.
//!
//! This module owns the application-facing UI state: navigation, projected data refreshes,
//! persistence feedback, and command dispatch. Feature-specific command handlers live in
//! private child modules while sharing this single controller and its state.

mod evaluation;
mod projects;
#[cfg(test)]
mod tests;
mod tracking;

use crate::application::Tracker;
use crate::language::Language;
use crate::{
    AppActions, AppWindow, Page, Status, StatusKind, UiCommand, UiCommandKind, domain, presentation,
};
use slint::{ComponentHandle, SharedString, Timer, TimerMode, Weak};
use std::time::Duration;

const STATUS_DURATION: Duration = Duration::from_secs(3);

/// Connects the Slint UI to the application tracker and begins dispatching commands.
pub(crate) fn bind(ui: &AppWindow, tracker: Tracker, language: Language) {
    let mut controller = UiController::new(ui, tracker, language);
    controller.refresh(ui);
    controller.persist_initial(ui);

    ui.global::<AppActions>()
        .on_dispatch(move |command| controller.handle(command));
}

/// State holder and coordinator for Slint UI events and presentation projections.
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
            UiCommandKind::SaveProject => self.save_project(&ui, command.id, command.text),
            UiCommandKind::ArchiveProject => self.archive_project(&ui, command.id),
            UiCommandKind::UnarchiveProject => self.unarchive_project(&ui, command.id),
            UiCommandKind::DeleteProject => self.delete_project(&ui, command.id),
            UiCommandKind::ExportMarkdown => self.export_markdown(&ui),
            UiCommandKind::OpenDrilldown => self.open_drilldown(&ui, command.id),
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
