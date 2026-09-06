use crate::error::{Result, TrackerError};
use chrono::{Datelike, Local, TimeZone};
use serde::{Deserialize, Deserializer, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub(crate) struct ProjectId(String);

impl ProjectId {
    fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for ProjectId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub(crate) struct TaskId(String);

impl TaskId {
    fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Project {
    id: ProjectId,
    name: String,
    #[serde(default)]
    archived: bool,
}

impl Project {
    fn new(id: ProjectId, name: String) -> Self {
        Self {
            id,
            name,
            archived: false,
        }
    }

    pub(crate) fn id(&self) -> &ProjectId {
        &self.id
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn archived(&self) -> bool {
        self.archived
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Task {
    id: TaskId,
    project_id: ProjectId,
    name: Option<String>,
    started: i64,
    ended: i64,
    note: String,
}

impl Task {
    fn from_active(active: ActiveTask, ended: i64) -> Self {
        Self {
            id: active.id,
            project_id: active.project_id,
            name: None,
            started: active.started,
            ended,
            note: String::new(),
        }
    }

    pub(crate) fn project_id(&self) -> &ProjectId {
        &self.project_id
    }

    pub(crate) fn started(&self) -> i64 {
        self.started
    }

    pub(crate) fn ended(&self) -> i64 {
        self.ended
    }

    pub(crate) fn note(&self) -> &str {
        &self.note
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct ActiveTask {
    id: TaskId,
    project_id: ProjectId,
    started: i64,
    checkpoint: i64,
    #[serde(default)]
    paused: bool,
}

impl ActiveTask {
    fn new(id: TaskId, project_id: ProjectId, timestamp: i64) -> Self {
        Self {
            id,
            project_id,
            started: timestamp,
            checkpoint: timestamp,
            paused: false,
        }
    }

    pub(crate) fn project_id(&self) -> &ProjectId {
        &self.project_id
    }

    pub(crate) fn paused(&self) -> bool {
        self.paused
    }

    pub(crate) fn elapsed_until(&self, timestamp: i64) -> i64 {
        self.checkpoint - self.started
            + if self.paused {
                0
            } else {
                timestamp - self.checkpoint
            }
    }
}

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
            StoredData::Current(current) => Ok(Self {
                projects: current.projects,
                tasks: current.tasks,
                active_task: current.active_task,
            }),
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
                    .map(|(index, task)| Task {
                        id: TaskId::new(format!("legacy-task-{}", index + 1)),
                        project_id: ProjectId::new(task.task_id),
                        name: None,
                        started: task.started,
                        ended: task.ended,
                        note: task.note,
                    })
                    .collect(),
                active_task: legacy.active.map(|task| ActiveTask {
                    id: TaskId::new(format!("legacy-active-task-{}", task.started)),
                    project_id: ProjectId::new(task.task_id),
                    started: task.started,
                    checkpoint: task.checkpoint,
                    paused: false,
                }),
            }),
        }
    }
}

impl Data {
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
            .find(|project| project.id == *id)
            .map(|project| project.name.clone())
            .unwrap_or_else(|| "Deleted project".into())
    }

    pub(crate) fn project_name_by_str(&self, id: &str) -> Option<&str> {
        self.projects
            .iter()
            .find(|project| project.id.as_str() == id)
            .map(Project::name)
    }

    pub(crate) fn recover_active(&mut self) {
        if let Some(active) = self.active_task.take() {
            let ended = active.checkpoint;
            self.tasks.push(Task::from_active(active, ended));
        }
    }

    pub(crate) fn start_tracking(&mut self, project_id: ProjectId, timestamp: i64) {
        let id = TaskId::new(format!("task-{timestamp}-{}", self.tasks.len() + 1));
        self.active_task = Some(ActiveTask::new(id, project_id, timestamp));
    }

    pub(crate) fn checkpoint_active(&mut self, timestamp: i64) -> bool {
        if let Some(active) = &mut self.active_task {
            if active.paused {
                return false;
            }
            active.checkpoint = timestamp;
            true
        } else {
            false
        }
    }

    pub(crate) fn toggle_pause(&mut self, timestamp: i64) -> bool {
        let Some(active) = &mut self.active_task else {
            return false;
        };

        if active.paused {
            active.started += timestamp - active.checkpoint;
            active.checkpoint = timestamp;
            active.paused = false;
        } else {
            active.checkpoint = timestamp;
            active.paused = true;
        }
        true
    }

    pub(crate) fn end_tracking(&mut self, timestamp: i64) -> bool {
        let Some(active) = self.active_task.take() else {
            return false;
        };
        let ended = if active.paused {
            active.checkpoint
        } else {
            timestamp
        };
        self.tasks.push(Task::from_active(active, ended));
        true
    }

    pub(crate) fn save_task_note(&mut self, note: String) {
        if let Some(task) = self.tasks.last_mut() {
            task.note = note;
        }
    }

    pub(crate) fn add_project(&mut self, id: ProjectId, name: String) {
        self.projects.push(Project::new(id, name));
    }

    pub(crate) fn rename_project(&mut self, id: &ProjectId, name: String) {
        if let Some(project) = self.projects.iter_mut().find(|project| project.id == *id) {
            project.name = name;
        }
    }

    pub(crate) fn archive_project(&mut self, id: &ProjectId) {
        if let Some(project) = self.projects.iter_mut().find(|project| project.id == *id) {
            project.archived = true;
        }
    }

    pub(crate) fn unarchive_project(&mut self, id: &ProjectId) {
        if let Some(project) = self.projects.iter_mut().find(|project| project.id == *id) {
            project.archived = false;
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Range {
    Day,
    Week,
    Month,
    Year,
}
impl Range {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Day => "TODAY",
            Self::Week => "THIS WEEK",
            Self::Month => "THIS MONTH",
            Self::Year => "THIS YEAR",
        }
    }
}

pub(crate) fn validate_project_name(name: &str) -> Result<String> {
    let name = name.trim();
    (!name.is_empty())
        .then(|| name.to_owned())
        .ok_or(TrackerError::EmptyProjectName)
}

pub(crate) fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

pub(crate) fn duration(seconds: i64) -> String {
    let minutes = (seconds.max(0) + 30) / 60;
    if minutes >= 60 {
        format!("{} h {}", minutes / 60, minutes % 60)
    } else {
        format!("{minutes} min")
    }
}

pub(crate) fn in_range(task: &Task, range: Range, timestamp: i64) -> bool {
    let current = Local
        .timestamp_opt(timestamp, 0)
        .single()
        .unwrap_or_else(Local::now);
    let item = Local
        .timestamp_opt(task.ended, 0)
        .single()
        .unwrap_or_else(Local::now);
    match range {
        Range::Day => current.date_naive() == item.date_naive(),
        Range::Week => current.iso_week() == item.iso_week(),
        Range::Month => current.year() == item.year() && current.month() == item.month(),
        Range::Year => current.year() == item.year(),
    }
}
pub(crate) fn markdown(data: &Data, range: Range, timestamp: i64) -> String {
    let tasks: Vec<_> = data
        .tasks()
        .iter()
        .filter(|task| in_range(task, range, timestamp))
        .collect();
    let total: i64 = tasks.iter().map(|task| task.ended - task.started).sum();
    let mut report = format!(
        "# Tempo — {}\n\n**Total:** {}\n",
        range.name(),
        duration(total)
    );
    if tasks.is_empty() {
        report.push_str("\nNo completed tasks.\n");
    }
    for task in tasks {
        let date = Local
            .timestamp_opt(task.ended, 0)
            .single()
            .unwrap_or_else(Local::now)
            .format("%Y-%m-%d %H:%M");
        report.push_str(&format!(
            "\n- **{}** — {} ({})",
            data.project_name(task.project_id()),
            duration(task.ended - task.started),
            date
        ));
        if !task.note.trim().is_empty() {
            report.push_str(&format!(": {}", task.note.trim()));
        }
        report.push('\n');
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::TrackerError;

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
    fn duration_range_and_markdown_are_readable() {
        let mut data = Data::defaults();
        data.tasks.push(Task {
            id: TaskId::new("task-1"),
            project_id: ProjectId::new("project-1"),
            name: None,
            started: 0,
            ended: 2520,
            note: "Brief".into(),
        });
        assert_eq!(duration(2520), "42 min");
        assert!(in_range(&data.tasks()[0], Range::Year, 2520));
        assert!(markdown(&data, Range::Year, 2520).contains("Project Atlas"));
    }

    #[test]
    fn legacy_data_migrates_projects_and_tasks_without_losing_history() {
        let legacy = r#"{
            "tasks": [{"id": "task-1", "title": "Project Atlas"}],
            "sessions": [{"task_id": "task-1", "started": 10, "ended": 70, "note": "Brief"}],
            "active": null
        }"#;

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
        let legacy = r#"{
            "tasks": [{"id": "task-1", "title": "Project Atlas"}],
            "sessions": [],
            "active": {"task_id": "task-1", "started": 10, "checkpoint": 40}
        }"#;

        let data: Data = serde_json::from_str(legacy).unwrap();
        let active = data.active_task().unwrap();

        assert_eq!(data.project_name(active.project_id()), "Project Atlas");
    }
}
