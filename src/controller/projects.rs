use super::UiController;
use crate::{
    domain, AppWindow, FirstRunPhase, Page, ProjectEditorMode, ProjectEditorState, ProjectItem,
};
use slint::{Model, ModelRc, SharedString, VecModel};

impl UiController {
    pub(super) fn open_project_create(&mut self, ui: &AppWindow) {
        ui.set_project_editor(ProjectEditorState {
            mode: ProjectEditorMode::Create,
            ..Default::default()
        });
        ui.set_current_page(Page::ProjectEditor);
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
        ui.set_current_page(Page::ProjectEditor);
    }

    pub(super) fn add_project(&mut self, ui: &AppWindow, name: SharedString) {
        let result = self.create_project(name).map(|_| ());
        self.finish_project_action(ui, result);
    }

    pub(super) fn project_name_exists(&self, name: SharedString) -> bool {
        self.tracker
            .data()
            .projects()
            .iter()
            .any(|project| project.name() == name.as_str())
    }

    /// Persists one onboarding project and adds it to the list available for tracking.
    pub(super) fn add_first_run_project(&mut self, ui: &AppWindow, name: SharedString) {
        match self.create_project(name) {
            Ok(project) => {
                let mut first_run = ui.get_first_run();
                let mut created_projects = (0..first_run.created_projects.row_count())
                    .filter_map(|index| first_run.created_projects.row_data(index))
                    .collect::<Vec<_>>();
                created_projects.push(project);
                first_run.created_projects = ModelRc::new(VecModel::from(created_projects));
                first_run.draft_name = SharedString::new();
                first_run.error = SharedString::new();
                first_run.phase = FirstRunPhase::ChooseNext;
                ui.set_first_run(first_run);
                self.refresh(ui);
            }
            Err(error) => {
                let mut first_run = ui.get_first_run();
                first_run.error = error.to_string().into();
                ui.set_first_run(first_run);
            }
        }
    }

    /// Creates and persists a project, returning the UI item identified by its generated ID.
    fn create_project(&mut self, name: SharedString) -> crate::error::Result<ProjectItem> {
        let project_id = self.tracker.add_project(name.as_str(), domain::now())?;
        let project = self
            .tracker
            .data()
            .projects()
            .iter()
            .find(|project| project.id() == &project_id)
            .expect("a newly persisted project remains in the tracker");
        Ok(ProjectItem {
            id: project.id().as_str().into(),
            name: project.name().into(),
            completed: false,
            archived: project.archived(),
        })
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
                ui.set_current_page(Page::Settings);
            }
            Err(error) => {
                let mut editor = ui.get_project_editor();
                editor.error = error.to_string().into();
                ui.set_project_editor(editor);
            }
        }
    }
}
