//! The aggregate that owns project configuration and tracked work.
//!
//! It is the domain boundary for starting, pausing, finishing, and recovering
//! tracking, as well as for project lifecycle changes.

use super::{ActiveTask, Project, ProjectId, Task, TaskId};
use crate::error::{Result, TrackerError};

/// Owns all projects, completed tasks, and one active task with an optional
/// paused task it interrupted.
///
/// Project deletion also removes its completed work and any matching active
/// task. Archiving, by contrast, retains the project's history.
#[derive(Clone)]
pub(crate) struct Data {
    projects: Vec<Project>,
    tasks: Vec<Task>,
    active_task: Option<ActiveTask>,
    interrupted_task: Option<ActiveTask>,
}

impl Data {
    /// Reconstructs the aggregate from the persistence adapter's records.
    pub(crate) fn from_storage(
        projects: Vec<Project>,
        tasks: Vec<Task>,
        active_task: Option<ActiveTask>,
        interrupted_task: Option<ActiveTask>,
    ) -> Self {
        Self {
            projects,
            tasks,
            active_task,
            interrupted_task,
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
            interrupted_task: None,
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
    pub(crate) fn interrupted_task(&self) -> Option<&ActiveTask> {
        self.interrupted_task.as_ref()
    }
    /// Returns whether the active task temporarily interrupts a paused task.
    pub(crate) fn has_interrupted_task(&self) -> bool {
        self.interrupted_task.is_some()
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

    /// Stops restored unfinished work at each task's most recently persisted checkpoint.
    ///
    /// This prevents time spent while the application was not running from
    /// being included in the completed task.
    pub(crate) fn recover_active(&mut self) {
        if let Some(interrupted) = self.interrupted_task.take() {
            self.tasks.push(interrupted.recover());
        }
        if let Some(active) = self.active_task.take() {
            self.tasks.push(active.recover());
        }
    }

    /// Begins a task or temporarily interrupts the current task.
    ///
    /// Only one interruption level is supported: the active secondary task
    /// must be finished before another task can begin.
    pub(crate) fn start_tracking(&mut self, project_id: ProjectId, timestamp: i64) -> Result<()> {
        if self.interrupted_task.is_some() {
            return Err(TrackerError::SecondaryTaskAlreadyActive);
        }
        if let Some(mut primary) = self.active_task.take() {
            if !primary.paused() {
                primary.toggle_pause(timestamp);
            }
            self.interrupted_task = Some(primary);
        }
        let id = self.next_task_id(timestamp);
        self.active_task = Some(ActiveTask::new(id, project_id, timestamp));
        Ok(())
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

    /// Converts the active task into a completed task at `timestamp` and makes
    /// a paused primary task active again after its secondary task finishes.
    ///
    /// Returns `false` when no task is active.
    pub(crate) fn end_tracking(&mut self, timestamp: i64) -> bool {
        let Some(active) = self.active_task.take() else {
            return false;
        };
        self.tasks.push(active.into_task(timestamp));
        if let Some(primary) = self.interrupted_task.take() {
            self.active_task = Some(primary);
        }
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
        if self
            .interrupted_task
            .as_ref()
            .is_some_and(|task| task.project_id() == id)
        {
            self.interrupted_task = None;
        }
    }

    /// Creates an ID that remains unique when multiple tasks start in the same second.
    ///
    /// Completed work, the visible active task, and a paused interrupted task all
    /// participate in the check because a primary and secondary task may coexist.
    fn next_task_id(&self, timestamp: i64) -> TaskId {
        let mut sequence = 1;
        loop {
            let candidate = format!("task-{timestamp}-{sequence}");
            let in_use = self
                .tasks
                .iter()
                .any(|task| task.id().as_str() == candidate)
                || self
                    .active_task
                    .as_ref()
                    .is_some_and(|task| task.id().as_str() == candidate)
                || self
                    .interrupted_task
                    .as_ref()
                    .is_some_and(|task| task.id().as_str() == candidate);
            if !in_use {
                return TaskId::new(candidate);
            }
            sequence += 1;
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
        data.start_tracking(ProjectId::new("project-2"), 10)
            .unwrap();
        assert!(data.checkpoint_active(40));
        data.recover_active();
        assert!(data.active_task().is_none());
        assert_eq!(data.tasks()[0].ended() - data.tasks()[0].started(), 30);
        assert_eq!(data.project_name(data.tasks()[0].project_id()), "Admin");
        data.start_tracking(ProjectId::new("project-1"), 50)
            .unwrap();
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
        data.start_tracking(ProjectId::new("project-1"), 10)
            .unwrap();
        assert!(data.toggle_pause(70));
        assert_eq!(data.active_task().unwrap().elapsed_until(120), 60);
        assert!(data.toggle_pause(130));
        assert_eq!(data.active_task().unwrap().elapsed_until(160), 90);
        assert!(data.toggle_pause(170));
        assert!(data.end_tracking(220));
        assert_eq!(data.tasks()[0].ended() - data.tasks()[0].started(), 100);
    }

    #[test]
    fn secondary_task_pauses_and_then_restores_the_primary_task() {
        let mut data = Data::defaults();
        data.start_tracking(ProjectId::new("project-1"), 10)
            .unwrap();
        assert!(data.checkpoint_active(40));

        data.start_tracking(ProjectId::new("project-2"), 50)
            .unwrap();
        assert!(data.has_interrupted_task());
        assert_eq!(
            data.active_task().unwrap().project_id().as_str(),
            "project-2"
        );
        assert_eq!(data.interrupted_task().unwrap().elapsed_until(80), 40);
        assert!(data.interrupted_task().unwrap().paused());
        assert!(matches!(
            data.start_tracking(ProjectId::new("project-3"), 60),
            Err(TrackerError::SecondaryTaskAlreadyActive)
        ));

        assert!(data.end_tracking(80));
        assert_eq!(data.tasks().len(), 1);
        assert_eq!(data.tasks()[0].project_id().as_str(), "project-2");
        assert_eq!(
            data.active_task().unwrap().project_id().as_str(),
            "project-1"
        );
        assert!(data.active_task().unwrap().paused());
        assert!(!data.has_interrupted_task());
    }

    #[test]
    fn recovery_finishes_both_tasks_without_resuming_them() {
        let mut data = Data::defaults();
        data.start_tracking(ProjectId::new("project-1"), 10)
            .unwrap();
        assert!(data.checkpoint_active(40));
        data.start_tracking(ProjectId::new("project-2"), 50)
            .unwrap();
        assert!(data.checkpoint_active(70));

        data.recover_active();

        assert!(data.active_task().is_none());
        assert!(!data.has_interrupted_task());
        assert_eq!(data.tasks().len(), 2);
        assert_eq!(data.tasks()[0].project_id().as_str(), "project-1");
        assert_eq!(data.tasks()[0].ended() - data.tasks()[0].started(), 40);
        assert_eq!(data.tasks()[1].project_id().as_str(), "project-2");
        assert_eq!(data.tasks()[1].ended() - data.tasks()[1].started(), 20);
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
        data.start_tracking(project_id.clone(), 10).unwrap();
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
        data.start_tracking(project_id.clone(), 10).unwrap();
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
