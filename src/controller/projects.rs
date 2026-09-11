use super::UiController;
use crate::{AppWindow, Page, ProjectEditorMode, ProjectEditorState, ProjectItem, domain};
use slint::SharedString;

impl UiController {
    pub(super) fn open_project_create(&mut self, ui: &AppWindow) {
        ui.set_project_editor(ProjectEditorState {
            mode: ProjectEditorMode::Create,
            ..Default::default()
        });
        ui.set_page(Page::ProjectEditor);
    }

    pub(super) fn open_project_edit(&mut self, ui: &AppWindow, id: SharedString) {
        let Some(project) = self
            .tracker
            .data()
            .projects()
            .iter()
            .find(|project| project.id().as_str() == id.as_str())
        else {
            self.set_error(ui, "The project no longer exists");
            return;
        };
        let editor_project = ProjectItem {
            id: project.id().as_str().into(),
            name: project.name().into(),
            completed: false,
            archived: project.archived(),
        };
        ui.set_project_editor(ProjectEditorState {
            mode: ProjectEditorMode::Edit,
            draft_name: editor_project.name.clone(),
            project: editor_project,
            ..Default::default()
        });
        ui.set_page(Page::ProjectEditor);
    }

    pub(super) fn add_project(&mut self, ui: &AppWindow, name: SharedString) {
        let result = self.tracker.add_project(name.as_str(), domain::now());
        self.finish_project_action(ui, result);
    }

    pub(super) fn save_project(&mut self, ui: &AppWindow, id: SharedString, name: SharedString) {
        let result = self.tracker.save_project(id.to_string(), name.as_str());
        self.finish_project_action(ui, result);
    }

    pub(super) fn archive_project(&mut self, ui: &AppWindow, id: SharedString) {
        let result = self.tracker.archive_project(id.to_string());
        self.finish_project_action(ui, result);
    }

    pub(super) fn unarchive_project(&mut self, ui: &AppWindow, id: SharedString) {
        let result = self.tracker.unarchive_project(id.to_string());
        self.finish_project_action(ui, result);
    }

    pub(super) fn delete_project(&mut self, ui: &AppWindow, id: SharedString) {
        let result = self.tracker.delete_project(id.to_string());
        self.finish_project_action(ui, result);
    }

    /// Close only after persistence succeeds; failures preserve the draft and show an inline error.
    pub(super) fn finish_project_action(&self, ui: &AppWindow, result: crate::error::Result<()>) {
        match result {
            Ok(()) => {
                self.refresh(ui);
                ui.set_project_editor(Default::default());
                ui.set_page(Page::Settings);
            }
            Err(error) => {
                let mut editor = ui.get_project_editor();
                editor.error = error.to_string().into();
                ui.set_project_editor(editor);
            }
        }
    }
}
