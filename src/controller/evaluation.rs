use super::UiController;
use crate::{AppWindow, Page, Range, domain, persistence, presentation};
use rfd::FileDialog;
use slint::SharedString;

impl UiController {
    pub(super) fn update_evaluation_task(
        &mut self,
        ui: &AppWindow,
        id: SharedString,
        note: SharedString,
    ) {
        match self
            .tracker
            .update_task_note(id.to_string(), note.to_string())
        {
            Ok(true) => self.refresh(ui),
            Ok(false) => self.set_error(ui, "The session no longer exists"),
            Err(error) => self.set_error(ui, format!("Could not save data: {error}")),
        }
    }

    pub(super) fn delete_evaluation_task(&mut self, ui: &AppWindow, id: SharedString) {
        match self.tracker.delete_task(id.to_string()) {
            Ok(true) => self.refresh(ui),
            Ok(false) => self.set_error(ui, "The session no longer exists"),
            Err(error) => self.set_error(ui, format!("Could not save data: {error}")),
        }
    }

    pub(super) fn choose_range(&mut self, ui: &AppWindow, range: Range) {
        self.tracker.choose_range(presentation::domain_range(range));
        self.refresh(ui);
    }

    pub(super) fn export_markdown(&mut self, ui: &AppWindow) {
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

    pub(super) fn open_drilldown(&mut self, ui: &AppWindow, id: SharedString) {
        self.tracker.select_evaluation_project(id.to_string());
        self.refresh(ui);
        ui.set_page(Page::Drilldown);
    }
}
