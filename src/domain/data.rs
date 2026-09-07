use serde::{Deserialize, Deserializer, Serialize};

use super::{ActiveTask, Project, ProjectId, Task, TaskId};

/// The aggregate that owns all projects and their tracked work.
#[derive(Clone, Serialize)]
pub(crate) struct Data {
    projects: Vec<Project>,
    tasks: Vec<Task>,
    active_task: Option<ActiveTask>,
}

#[derive(Deserialize)]
struct CurrentData {
    projects: Vec<Project>,
    tasks: Vec<Task>,
    active_task: Option<ActiveTask>,
}

#[derive(Deserialize)]
struct LegacyData {
    tasks: Vec<LegacyProject>,
    sessions: Vec<LegacyTask>,
    active: Option<LegacyActiveTask>,
}

#[derive(Deserialize)]
struct LegacyProject {
    id: String,
    title: String,
}

#[derive(Deserialize)]
struct LegacyTask {
    task_id: String,
    started: i64,
    ended: i64,
    note: String,
}

#[derive(Deserialize)]
struct LegacyActiveTask {
    task_id: String,
    started: i64,
    checkpoint: i64,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum StoredData {
    Current(CurrentData),
    Legacy(LegacyData),
}

impl<'de> Deserialize<'de> for Data {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match StoredData::deserialize(deserializer)? {
            StoredData::Current(current) => Ok(Self::from_storage(
                current.projects,
                current.tasks,
                current.active_task,
            )),
            StoredData::Legacy(legacy) => Ok(Self {
                projects: legacy
                    .tasks
                    .into_iter()
                    .map(|project| Project::new(ProjectId::new(project.id), project.title))
                    .collect(),
                tasks: legacy
                    .sessions
                    .into_iter()
                    .enumerate()
                    .map(|(index, task)| {
                        Task::from_storage(
                            TaskId::new(format!("legacy-task-{}", index + 1)),
                            ProjectId::new(task.task_id),
                            None,
                            task.started,
                            task.ended,
                            task.note,
                        )
                    })
                    .collect(),
                active_task: legacy.active.map(|task| {
                    ActiveTask::from_storage(
                        TaskId::new(format!("legacy-active-task-{}", task.started)),
                        ProjectId::new(task.task_id),
                        task.started,
                        task.checkpoint,
                        false,
                    )
                }),
            }),
        }
    }
}

impl Data {
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

    pub(crate) fn project_name(&self, id: &ProjectId) -> String {
        self.projects
            .iter()
            .find(|project| project.id() == id)
            .map(|project| project.name().to_owned())
            .unwrap_or_else(|| "Deleted project".into())
    }
    pub(crate) fn project_name_by_str(&self, id: &str) -> Option<&str> {
        self.projects
            .iter()
            .find(|project| project.id().as_str() == id)
            .map(Project::name)
    }

    pub(crate) fn recover_active(&mut self) {
        if let Some(active) = self.active_task.take() {
            self.tasks.push(active.recover());
        }
    }
    pub(crate) fn start_tracking(&mut self, project_id: ProjectId, timestamp: i64) {
        let id = TaskId::new(format!("task-{timestamp}-{}", self.tasks.len() + 1));
        self.active_task = Some(ActiveTask::new(id, project_id, timestamp));
    }
    pub(crate) fn checkpoint_active(&mut self, timestamp: i64) -> bool {
        self.active_task
            .as_mut()
            .is_some_and(|active| active.checkpoint_at(timestamp))
    }
    pub(crate) fn toggle_pause(&mut self, timestamp: i64) -> bool {
        let Some(active) = &mut self.active_task else {
            return false;
        };
        active.toggle_pause(timestamp);
        true
    }
    pub(crate) fn end_tracking(&mut self, timestamp: i64) -> bool {
        let Some(active) = self.active_task.take() else {
            return false;
        };
        self.tasks.push(active.into_task(timestamp));
        true
    }
    pub(crate) fn save_task_note(&mut self, note: String) {
        if let Some(task) = self.tasks.last_mut() {
            task.set_note(note);
        }
    }
    pub(crate) fn add_project(&mut self, id: ProjectId, name: String) {
        self.projects.push(Project::new(id, name));
    }
    pub(crate) fn rename_project(&mut self, id: &ProjectId, name: String) {
        if let Some(project) = self.projects.iter_mut().find(|project| project.id() == id) {
            project.rename(name);
        }
    }
    pub(crate) fn archive_project(&mut self, id: &ProjectId) {
        if let Some(project) = self.projects.iter_mut().find(|project| project.id() == id) {
            project.archive();
        }
    }
    pub(crate) fn unarchive_project(&mut self, id: &ProjectId) {
        if let Some(project) = self.projects.iter_mut().find(|project| project.id() == id) {
            project.unarchive();
        }
    }
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
        assert_eq!(data.project_name_by_str("project-1"), Some("Planning"));
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

    #[test]
    fn legacy_data_migrates_projects_and_tasks_without_losing_history() {
        let legacy = r#"{"tasks": [{"id": "task-1", "title": "Project Atlas"}], "sessions": [{"task_id": "task-1", "started": 10, "ended": 70, "note": "Brief"}], "active": null}"#;
        let data: Data = serde_json::from_str(legacy).unwrap();
        assert_eq!(data.projects()[0].name(), "Project Atlas");
        assert_eq!(
            data.project_name(data.tasks()[0].project_id()),
            "Project Atlas"
        );
        assert_eq!(data.tasks()[0].note(), "Brief");
        let saved = serde_json::to_value(&data).unwrap();
        assert!(saved.get("projects").is_some());
        assert!(saved.get("sessions").is_none());
    }

    #[test]
    fn legacy_active_session_becomes_an_active_task_for_its_project() {
        let legacy = r#"{"tasks": [{"id": "task-1", "title": "Project Atlas"}], "sessions": [], "active": {"task_id": "task-1", "started": 10, "checkpoint": 40}}"#;
        let data: Data = serde_json::from_str(legacy).unwrap();
        let active = data.active_task().unwrap();
        assert_eq!(data.project_name(active.project_id()), "Project Atlas");
    }
}
