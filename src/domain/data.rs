//! The aggregate that owns project configuration and tracked work.
//!
//! It is the domain boundary for starting, pausing, finishing, and recovering
//! tracking, as well as for project lifecycle changes.

use super::{ActiveTask, Project, ProjectId, Task, TaskId};

/// Owns all projects, completed tasks, and at most one active task.
///
/// Project deletion also removes its completed work and any matching active
/// task. Archiving, by contrast, retains the project's history.
#[derive(Clone)]
pub(crate) struct Data {
    projects: Vec<Project>,
    tasks: Vec<Task>,
    active_task: Option<ActiveTask>,
}

impl Data {
    /// Reconstructs the aggregate from the persistence adapter's records.
    pub(crate) fn from_storage(
        projects: Vec<Project>,
        tasks: Vec<Task>,
        active_task: Option<ActiveTask>,
    ) -> Self {
        Self {
            projects,
            tasks,
            active_task,
        }
    }

    /// Creates the initial set of projects with no recorded or active work.
    pub(crate) fn defaults() -> Self {
        Self {
            projects: ["Project Atlas", "Admin", "Writing", "Personal"]
                .into_iter()
                .enumerate()
                .map(|(index, name)| {
                    Project::new(
                        ProjectId::new(format!("project-{}", index + 1)),
                        name.into(),
                    )
                })
                .collect(),
            tasks: Vec::new(),
            active_task: None,
        }
    }

    pub(crate) fn projects(&self) -> &[Project] {
        &self.projects
    }
    pub(crate) fn tasks(&self) -> &[Task] {
        &self.tasks
    }
    pub(crate) fn active_task(&self) -> Option<&ActiveTask> {
        self.active_task.as_ref()
    }
    pub(crate) fn has_tasks(&self) -> bool {
        !self.tasks.is_empty()
    }

    /// Returns the project's name, or a display label when its project was deleted.
    pub(crate) fn project_name(&self, id: &ProjectId) -> String {
        self.projects
            .iter()
            .find(|project| project.id() == id)
            .map(|project| project.name().to_owned())
            .unwrap_or_else(|| "Deleted project".into())
    }

    /// Stops a restored active task at its most recently persisted checkpoint.
    ///
    /// This prevents time spent while the application was not running from
    /// being included in the completed task.
    pub(crate) fn recover_active(&mut self) {
        if let Some(active) = self.active_task.take() {
            self.tasks.push(active.recover());
        }
    }

    /// Begins a task for `project_id`, replacing any existing active task.
    pub(crate) fn start_tracking(&mut self, project_id: ProjectId, timestamp: i64) {
        let id = TaskId::new(format!("task-{timestamp}-{}", self.tasks.len() + 1));
        self.active_task = Some(ActiveTask::new(id, project_id, timestamp));
    }
    /// Records elapsed work through `timestamp` when an unpaused task is active.
    ///
    /// Returns `false` when there is no active task or it is paused.
    pub(crate) fn checkpoint_active(&mut self, timestamp: i64) -> bool {
        self.active_task
            .as_mut()
            .is_some_and(|active| active.checkpoint_at(timestamp))
    }
    /// Pauses or resumes the active task at `timestamp`.
    ///
    /// Returns `false` when there is no active task.
    pub(crate) fn toggle_pause(&mut self, timestamp: i64) -> bool {
        let Some(active) = &mut self.active_task else {
            return false;
        };
        active.toggle_pause(timestamp);
        true
    }

    /// Converts the active task into a completed task at `timestamp`.
    ///
    /// Returns `false` when no task is active.
    pub(crate) fn end_tracking(&mut self, timestamp: i64) -> bool {
        let Some(active) = self.active_task.take() else {
            return false;
        };
        self.tasks.push(active.into_task(timestamp));
        true
    }
    /// Replaces the note on the most recently completed task, if there is one.
    pub(crate) fn save_task_note(&mut self, note: String) {
        if let Some(task) = self.tasks.last_mut() {
            task.set_note(note);
        }
    }
    /// Replaces one completed task's note, returning whether its identifier was found.
    pub(crate) fn update_task_note(&mut self, id: &TaskId, note: String) -> bool {
        let Some(task) = self.tasks.iter_mut().find(|task| task.id() == id) else {
            return false;
        };
        task.set_note(note);
        true
    }
    /// Removes one completed task, returning whether its identifier was found.
    pub(crate) fn delete_task(&mut self, id: &TaskId) -> bool {
        let previous_len = self.tasks.len();
        self.tasks.retain(|task| task.id() != id);
        self.tasks.len() != previous_len
    }
    /// Adds an active project with the supplied identifier and name.
    pub(crate) fn add_project(&mut self, id: ProjectId, name: String) {
        self.projects.push(Project::new(id, name));
    }
    /// Renames the matching project; an unknown identifier leaves the aggregate unchanged.
    pub(crate) fn rename_project(&mut self, id: &ProjectId, name: String) {
        if let Some(project) = self.projects.iter_mut().find(|project| project.id() == id) {
            project.rename(name);
        }
    }
    /// Archives the matching project without removing its work history.
    pub(crate) fn archive_project(&mut self, id: &ProjectId) {
        if let Some(project) = self.projects.iter_mut().find(|project| project.id() == id) {
            project.archive();
        }
    }
    /// Restores the matching project to the active lifecycle state.
    pub(crate) fn unarchive_project(&mut self, id: &ProjectId) {
        if let Some(project) = self.projects.iter_mut().find(|project| project.id() == id) {
            project.unarchive();
        }
    }

    /// Removes a project and every task associated with it.
    pub(crate) fn delete_project(&mut self, id: &ProjectId) {
        self.projects.retain(|project| project.id() != id);
        self.tasks.retain(|task| task.project_id() != id);
        if self
            .active_task
            .as_ref()
            .is_some_and(|task| task.project_id() == id)
        {
            self.active_task = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{domain::validate_project_name, error::TrackerError};

    #[test]
    fn tracking_creates_tasks_under_projects() {
        let mut data = Data::defaults();
        data.start_tracking(ProjectId::new("project-2"), 10);
        assert!(data.checkpoint_active(40));
        data.recover_active();
        assert!(data.active_task().is_none());
        assert_eq!(data.tasks()[0].ended() - data.tasks()[0].started(), 30);
        assert_eq!(data.project_name(data.tasks()[0].project_id()), "Admin");
        data.start_tracking(ProjectId::new("project-1"), 50);
        assert!(data.end_tracking(80));
        data.save_task_note("Brief".into());
        assert_eq!(data.tasks()[1].note(), "Brief");

        let first_id = data.tasks()[0].id().clone();
        assert!(data.update_task_note(&first_id, "Updated planning".into()));
        assert_eq!(data.tasks()[0].note(), "Updated planning");
        assert!(data.delete_task(&first_id));
        assert_eq!(data.tasks().len(), 1);
        assert!(!data.delete_task(&first_id));
    }

    #[test]
    fn paused_tracking_excludes_the_paused_interval() {
        let mut data = Data::defaults();
        data.start_tracking(ProjectId::new("project-1"), 10);
        assert!(data.toggle_pause(70));
        assert_eq!(data.active_task().unwrap().elapsed_until(120), 60);
        assert!(data.toggle_pause(130));
        assert_eq!(data.active_task().unwrap().elapsed_until(160), 90);
        assert!(data.toggle_pause(170));
        assert!(data.end_tracking(220));
        assert_eq!(data.tasks()[0].ended() - data.tasks()[0].started(), 100);
    }

    #[test]
    fn project_names_are_validated_and_projects_can_be_renamed() {
        let mut data = Data::defaults();
        assert!(matches!(
            validate_project_name("  "),
            Err(TrackerError::EmptyProjectName)
        ));
        data.rename_project(
            &ProjectId::new("project-1"),
            validate_project_name("  Planning ").unwrap(),
        );
        data.add_project(
            ProjectId::new("project-5"),
            validate_project_name("Review").unwrap(),
        );
        assert_eq!(data.projects()[4].name(), "Review");
    }

    #[test]
    fn projects_can_be_archived_without_removing_their_history() {
        let mut data = Data::defaults();
        let project_id = ProjectId::new("project-1");
        data.start_tracking(project_id.clone(), 10);
        assert!(data.end_tracking(70));
        data.archive_project(&project_id);
        assert!(data.projects()[0].archived());
        assert_eq!(
            data.project_name(data.tasks()[0].project_id()),
            "Project Atlas"
        );
        data.unarchive_project(&project_id);
        assert!(!data.projects()[0].archived());
    }

    #[test]
    fn deleting_a_project_removes_its_sessions() {
        let mut data = Data::defaults();
        let project_id = ProjectId::new("project-1");
        data.start_tracking(project_id.clone(), 10);
        assert!(data.end_tracking(70));
        data.delete_project(&project_id);
        assert!(
            data.projects()
                .iter()
                .all(|project| project.id() != &project_id)
        );
        assert!(data.tasks().is_empty());
    }
}
