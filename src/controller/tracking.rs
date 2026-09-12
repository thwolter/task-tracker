use super::UiController;
use crate::{AppWindow, Page, domain};
use slint::SharedString;

impl UiController {
    pub(super) fn tick(&mut self, ui: &AppWindow) {
        if let Err(error) = self.tracker.tick(domain::now()) {
            self.set_error(ui, format!("Could not save data: {error}"));
        }
        // Keep the application-owned projection live even when TrackingView is
        // conditionally removed. HomeView consumes this same state as its link
        // back to the active session, while other editable projections retain
        // their current drafts and focus.
        ui.set_tracking(crate::presentation::tracking(&self.tracker, domain::now()));
    }

    pub(super) fn start_tracking(&mut self, ui: &AppWindow, project_id: SharedString) {
        match self
            .tracker
            .start_tracking(project_id.to_string(), domain::now())
        {
            Ok(()) => ui.set_current_page(Page::Tracking),
            Err(error) => self.set_error(ui, format!("Could not save data: {error}")),
        }
        self.refresh(ui);
    }

    pub(super) fn open_last_task(&mut self, ui: &AppWindow) {
        if self.tracker.data().has_tasks() {
            ui.set_current_page(Page::Note);
        }
    }

    pub(super) fn end_tracking(&mut self, ui: &AppWindow) {
        match self.tracker.end_tracking(domain::now()) {
            Ok(true) => ui.set_current_page(Page::Note),
            Ok(false) => {}
            Err(error) => self.set_error(ui, format!("Could not save data: {error}")),
        }
        self.refresh(ui);
    }

    pub(super) fn toggle_tracking_pause(&mut self, ui: &AppWindow) {
        if let Err(error) = self.tracker.toggle_tracking_pause(domain::now()) {
            self.set_error(ui, format!("Could not save data: {error}"));
        }
        self.refresh(ui);
    }

    pub(super) fn save_task_note(&mut self, ui: &AppWindow, note: SharedString) {
        if let Err(error) = self.tracker.save_task_note(note.to_string()) {
            self.set_error(ui, format!("Could not save data: {error}"));
        }
        ui.set_current_page(Page::Home);
        self.refresh(ui);
    }
}
