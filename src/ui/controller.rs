//! UI controller connecting Slint user interface events to application logic.
//!
//! This module defines [`bind`], which attaches a [`UiController`] to the Slint [`AppWindow`].
//! The controller handles user interactions through a unified command dispatch mechanism,
//! updates the underlying [`Tracker`] state, triggers view refreshes via [`presentation::refresh`],
//! and manages transient UI state such as dialogs and expiring status messages.

use crate::application::Tracker;
use crate::language::Language;
use crate::{
    AppActions, AppWindow, Page, Status, StatusKind, UiCommand, UiCommandKind, domain, persistence,
    presentation,
};
use rfd::FileDialog;
use slint::{ComponentHandle, SharedString, Timer, TimerMode, Weak};
use std::time::Duration;

/// The duration for which temporary status messages (success/error alerts) remain visible
/// in the user interface before being automatically cleared.
const STATUS_DURATION: Duration = Duration::from_secs(3);

/// Connects the Slint UI to the application tracker and begins dispatching commands.
///
/// This initializes a [`UiController`], performs an initial projection refresh and persistence
/// check, and registers a callback on the global [`AppActions`] to handle UI commands.
///
/// # Arguments
///
/// * `ui` - Reference to the instantiated Slint [`AppWindow`].
/// * `tracker` - The [`Tracker`] holding application state and persistence logic.
/// * `language` - The active [`Language`] used for localization and date formatting.
///
/// # Examples
///
/// ```rust,no_run
/// use crate::application::Tracker;
/// use crate::language::Language;
/// use crate::ui::bind;
/// use crate::AppWindow;
///
/// let ui = AppWindow::new().unwrap();
/// let tracker = Tracker::load_default();
/// bind(&ui, tracker, Language::English);
/// ```
pub(crate) fn bind(ui: &AppWindow, tracker: Tracker, language: Language) {
    let mut controller = UiController::new(ui, tracker, language);
    controller.refresh(ui);
    controller.persist_initial(ui);

    ui.global::<AppActions>()
        .on_dispatch(move |command| controller.handle(command));
}

/// State holder and coordinator for Slint UI events and presentation projections.
///
/// `UiController` maintains a weak reference to the Slint [`AppWindow`] to safely update
/// views without reference cycles, holds the domain [`Tracker`], tracks the active UI [`Language`],
/// and manages a [`Timer`] for expiring transient status banners.
struct UiController {
    ui: Weak<AppWindow>,
    tracker: Tracker,
    language: Language,
    status_timer: Timer,
}

impl UiController {
    /// Creates a new `UiController` instance for the given window, tracker, and language.
    fn new(ui: &AppWindow, tracker: Tracker, language: Language) -> Self {
        Self {
            ui: ui.as_weak(),
            tracker,
            language,
            status_timer: Timer::default(),
        }
    }

    /// Dispatches an incoming [`UiCommand`] from the Slint UI to the appropriate handler method.
    ///
    /// If the window handle cannot be upgraded (e.g. the window has closed), the command is ignored.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use crate::{AppActions, AppWindow};
    /// use slint::ComponentHandle;
    ///
    /// let ui = AppWindow::new().unwrap();
    /// let actions = ui.global::<AppActions>();
    /// // Dispatches a tick command to the controller's handle method
    /// actions.invoke_tick();
    /// ```
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

            UiCommandKind::AddProject => self.add_project(&ui, command.text),
            UiCommandKind::SaveProject => self.save_project(&ui, command.id, command.text),
            UiCommandKind::ArchiveProject => self.archive_project(&ui, command.id),
            UiCommandKind::UnarchiveProject => self.unarchive_project(&ui, command.id),
            UiCommandKind::DeleteProject => self.delete_project(&ui, command.id),

            UiCommandKind::ExportMarkdown => self.export_markdown(&ui),
            UiCommandKind::OpenDrilldown => self.open_drilldown(&ui, command.id),
        }
    }

    /// Handles a periodic timer tick.
    ///
    /// Checkpoints active time tracking state and saves data if changed. Only refreshes UI
    /// projections when on [`Page::Tracking`] to prevent stealing focus from text inputs
    /// on other pages.
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

    /// Starts tracking time for the project identified by `project_id`.
    ///
    /// Transitions the active view to [`Page::Tracking`] and refreshes presentation state.
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

    /// Navigates to the note entry page ([`Page::Note`]) for the most recently completed task,
    /// if at least one task exists.
    fn open_last_task(&mut self, ui: &AppWindow) {
        if self.tracker.data().has_tasks() {
            ui.set_page(Page::Note);
        }
    }

    /// Ends active time tracking and navigates to [`Page::Note`] to allow entering a task note.
    fn end_tracking(&mut self, ui: &AppWindow) {
        match self.tracker.end_tracking(domain::now()) {
            Ok(true) => ui.set_page(Page::Note),
            Ok(false) => {}
            Err(error) => self.set_error(ui, format!("Could not save data: {error}")),
        }
        self.refresh(ui);
    }

    /// Toggles pause state on the currently active tracking session.
    fn toggle_tracking_pause(&mut self, ui: &AppWindow) {
        if let Err(error) = self.tracker.toggle_tracking_pause(domain::now()) {
            self.set_error(ui, format!("Could not save data: {error}"));
        }
        self.refresh(ui);
    }

    /// Saves the given `note` to the most recently completed task and returns to [`Page::Home`].
    fn save_task_note(&mut self, ui: &AppWindow, note: SharedString) {
        if let Err(error) = self.tracker.save_task_note(note.to_string()) {
            self.set_error(ui, format!("Could not save data: {error}"));
        }
        ui.set_page(Page::Home);
        self.refresh(ui);
    }

    /// Updates the note of an existing historical task in evaluation drilldown.
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

    /// Deletes a historical task by its identifier.
    fn delete_evaluation_task(&mut self, ui: &AppWindow, id: SharedString) {
        match self.tracker.delete_task(id.to_string()) {
            Ok(true) => self.refresh(ui),
            Ok(false) => self.set_error(ui, "The session no longer exists"),
            Err(error) => self.set_error(ui, format!("Could not save data: {error}")),
        }
    }

    /// Changes the active calendar range filter for evaluation and report projections.
    fn choose_range(&mut self, ui: &AppWindow, range: crate::Range) {
        self.tracker.choose_range(presentation::domain_range(range));
        self.refresh(ui);
    }

    fn add_project(&mut self, ui: &AppWindow, name: SharedString) {
        match self.tracker.add_project(name.as_str(), domain::now()) {
            Ok(()) => {
                ui.set_page(Page::Settings);
                self.refresh(ui);
            }
            Err(error) => self.set_error(ui, error.to_string()),
        }
    }

    /// Saves a renamed project name.
    ///
    /// On success, closes the dialog and refreshes projections; on validation error, displays
    /// an error message.
    fn save_project(&mut self, ui: &AppWindow, id: SharedString, name: SharedString) {
        match self.tracker.save_project(id.to_string(), name.as_str()) {
            Ok(()) => {
                self.refresh(ui);
            }
            Err(error) => self.set_error(ui, error.to_string()),
        }
    }

    /// Archives a project by its identifier, hiding it from tracking selection while retaining history.
    fn archive_project(&mut self, ui: &AppWindow, id: SharedString) {
        match self.tracker.archive_project(id.to_string()) {
            Ok(()) => {
                ui.set_page(Page::Settings);
                self.refresh(ui);
            }
            Err(error) => self.set_error(ui, format!("Could not save data: {error}")),
        }
    }

    /// Unarchives a previously archived project, restoring it to the active tracking list.
    fn unarchive_project(&mut self, ui: &AppWindow, id: SharedString) {
        match self.tracker.unarchive_project(id.to_string()) {
            Ok(()) => {
                ui.set_page(Page::Settings);
                self.refresh(ui);
            }
            Err(error) => self.set_error(ui, format!("Could not save data: {error}")),
        }
    }

    /// Permanently deletes a project and its associated task history.
    fn delete_project(&mut self, ui: &AppWindow, id: SharedString) {
        match self.tracker.delete_project(id.to_string()) {
            Ok(()) => {
                ui.set_page(Page::Settings);
                self.refresh(ui);
            }
            Err(error) => self.set_error(ui, format!("Could not save data: {error}")),
        }
    }

    /// Prompts the user with a file dialog to choose an export location and writes a Markdown report.
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

    /// Selects a project for evaluation breakdown and navigates to [`Page::Drilldown`].
    fn open_drilldown(&mut self, ui: &AppWindow, id: SharedString) {
        self.tracker.select_evaluation_project(id.to_string());
        self.refresh(ui);
        ui.set_page(Page::Drilldown);
    }

    /// Re-evaluates presentation projections from the current tracker state and pushes them to the UI.
    fn refresh(&self, ui: &AppWindow) {
        presentation::refresh(ui, &self.tracker, domain::now(), self.language);
    }

    /// Saves the initial database state on startup, displaying an error status if persistence fails.
    fn persist_initial(&self, ui: &AppWindow) {
        if let Err(error) = self.tracker.save() {
            self.set_error(ui, format!("Could not save data: {error}"));
        }
    }

    /// Displays a temporary green success banner in the UI status bar.
    fn set_success(&self, ui: &AppWindow, message: impl Into<SharedString>) {
        self.set_status(ui, message, StatusKind::Success);
    }

    /// Displays a temporary red error banner in the UI status bar.
    fn set_error(&self, ui: &AppWindow, message: impl Into<SharedString>) {
        self.set_status(ui, message, StatusKind::Error);
    }

    /// Sets the UI status message and schedules a timer to clear it after [`STATUS_DURATION`].
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
}
