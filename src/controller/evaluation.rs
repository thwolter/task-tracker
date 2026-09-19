use super::{ExportContext, UiController};
use crate::{
    domain, persistence, presentation, report, AppWindow, ExportFormat, ExportState, Page, Range,
};
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

    pub(super) fn open_export(&mut self, ui: &AppWindow) {
        let (project_id, project) = match ui.get_current_page() {
            Page::Evaluation => (None, String::new()),
            Page::Drilldown => {
                let Some(project_id) = self.tracker.evaluating_project().cloned() else {
                    self.set_error(ui, "No project is selected for export");
                    return;
                };
                let project = self.tracker.data().project_name(&project_id);
                (Some(project_id), project)
            }
            _ => {
                self.set_error(ui, "Export is available from Evaluation");
                return;
            }
        };

        self.export_context = Some(ExportContext { project_id });
        ui.set_export_state(ExportState {
            project: project.into(),
            all_projects: ui.get_current_page() == Page::Evaluation,
            return_page: ui.get_current_page(),
        });
        ui.set_current_page(Page::Exportpage);
    }

    pub(super) fn export_report(&mut self, ui: &AppWindow, format: ExportFormat) {
        let Some(context) = self.export_context.clone() else {
            self.set_error(ui, "Choose an export from Evaluation first");
            return;
        };
        let (label, extension, report_format) = match format {
            ExportFormat::Csv => ("CSV", "csv", report::Format::Csv),
            ExportFormat::Markdown => ("Markdown", "md", report::Format::Markdown),
            ExportFormat::Json => ("JSON", "json", report::Format::Json),
        };
        let Some(path) = FileDialog::new()
            .add_filter(label, &[extension])
            .set_file_name(format!("tempo-report.{extension}"))
            .save_file()
        else {
            return;
        };
        let report = self.tracker.export_report(
            context.project_id.as_ref(),
            domain::now(),
            self.language,
            report_format,
        );
        match report.and_then(|contents| {
            persistence::export_report(&path, &contents).map_err(|error| error.to_string())
        }) {
            Ok(()) => self.set_success(ui, "Export completed"),
            Err(error) => self.set_error(ui, format!("Export failed: {error}")),
        }
    }

    pub(super) fn open_drilldown(&mut self, ui: &AppWindow, id: SharedString) {
        self.tracker.select_evaluation_project(id.to_string());
        self.refresh(ui);
        ui.set_current_page(Page::Drilldown);
    }
}
