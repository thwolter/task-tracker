use chrono::{Datelike, Local, TimeZone};
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Task {
    id: String,
    title: String,
}
impl Task {
    fn new(id: String, title: String) -> Self {
        Self { id, title }
    }
    pub(crate) fn id(&self) -> &str {
        &self.id
    }
    pub(crate) fn title(&self) -> &str {
        &self.title
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Session {
    task_id: String,
    started: i64,
    ended: i64,
    note: String,
}
impl Session {
    pub(crate) fn task_id(&self) -> &str {
        &self.task_id
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
pub(crate) struct ActiveSession {
    task_id: String,
    started: i64,
    checkpoint: i64,
}
impl ActiveSession {
    pub(crate) fn task_id(&self) -> &str {
        &self.task_id
    }
    pub(crate) fn started(&self) -> i64 {
        self.started
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Data {
    tasks: Vec<Task>,
    sessions: Vec<Session>,
    active: Option<ActiveSession>,
}
impl Data {
    pub(crate) fn defaults() -> Self {
        Self {
            tasks: ["Project Atlas", "Admin", "Writing", "Personal"]
                .into_iter()
                .enumerate()
                .map(|(index, title)| Task::new(format!("task-{}", index + 1), title.into()))
                .collect(),
            sessions: Vec::new(),
            active: None,
        }
    }
    pub(crate) fn tasks(&self) -> &[Task] {
        &self.tasks
    }
    pub(crate) fn sessions(&self) -> &[Session] {
        &self.sessions
    }
    pub(crate) fn active(&self) -> Option<&ActiveSession> {
        self.active.as_ref()
    }
    pub(crate) fn has_sessions(&self) -> bool {
        !self.sessions.is_empty()
    }
    pub(crate) fn task_name(&self, id: &str) -> String {
        self.tasks
            .iter()
            .find(|task| task.id == id)
            .map(|task| task.title.clone())
            .unwrap_or_else(|| "Deleted task".into())
    }
    pub(crate) fn task_title(&self, id: &str) -> Option<&str> {
        self.tasks
            .iter()
            .find(|task| task.id == id)
            .map(Task::title)
    }
    pub(crate) fn recover_active(&mut self) {
        if let Some(active) = self.active.take() {
            self.sessions.push(Session {
                task_id: active.task_id,
                started: active.started,
                ended: active.checkpoint,
                note: String::new(),
            });
        }
    }
    pub(crate) fn start_task(&mut self, task_id: String, timestamp: i64) {
        self.active = Some(ActiveSession {
            task_id,
            started: timestamp,
            checkpoint: timestamp,
        });
    }
    pub(crate) fn checkpoint_active(&mut self, timestamp: i64) -> bool {
        if let Some(active) = &mut self.active {
            active.checkpoint = timestamp;
            true
        } else {
            false
        }
    }
    pub(crate) fn end_active(&mut self, timestamp: i64) -> bool {
        let Some(active) = self.active.take() else {
            return false;
        };
        self.sessions.push(Session {
            task_id: active.task_id,
            started: active.started,
            ended: timestamp,
            note: String::new(),
        });
        true
    }
    pub(crate) fn save_note(&mut self, note: String) {
        if let Some(session) = self.sessions.last_mut() {
            session.note = note;
        }
    }
    pub(crate) fn add_task(&mut self, id: String, title: String) {
        self.tasks.push(Task::new(id, title));
    }
    pub(crate) fn rename_task(&mut self, id: &str, title: String) {
        if let Some(task) = self.tasks.iter_mut().find(|task| task.id == id) {
            task.title = title;
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

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct TaskNameError;
impl fmt::Display for TaskNameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Task name cannot be empty")
    }
}
pub(crate) fn validate_task_name(name: &str) -> Result<String, TaskNameError> {
    let name = name.trim();
    (!name.is_empty())
        .then(|| name.to_owned())
        .ok_or(TaskNameError)
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
        format!("{} h {} min", minutes / 60, minutes % 60)
    } else {
        format!("{minutes} min")
    }
}
pub(crate) fn in_range(session: &Session, range: Range, timestamp: i64) -> bool {
    let current = Local
        .timestamp_opt(timestamp, 0)
        .single()
        .unwrap_or_else(Local::now);
    let item = Local
        .timestamp_opt(session.ended, 0)
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
    let sessions: Vec<_> = data
        .sessions()
        .iter()
        .filter(|session| in_range(session, range, timestamp))
        .collect();
    let total: i64 = sessions
        .iter()
        .map(|session| session.ended - session.started)
        .sum();
    let mut report = format!(
        "# Tempo — {}\n\n**Total:** {}\n",
        range.name(),
        duration(total)
    );
    if sessions.is_empty() {
        report.push_str("\nNo completed sessions.\n");
    }
    for session in sessions {
        let date = Local
            .timestamp_opt(session.ended, 0)
            .single()
            .unwrap_or_else(Local::now)
            .format("%Y-%m-%d %H:%M");
        report.push_str(&format!(
            "\n- **{}** — {} ({})",
            data.task_name(&session.task_id),
            duration(session.ended - session.started),
            date
        ));
        if !session.note.trim().is_empty() {
            report.push_str(&format!(": {}", session.note.trim()));
        }
        report.push('\n');
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn task_actions_recover_and_complete_sessions() {
        let mut data = Data::defaults();
        data.start_task("task-2".into(), 10);
        assert!(data.checkpoint_active(40));
        data.recover_active();
        assert!(data.active().is_none());
        assert_eq!(
            data.sessions()[0].ended() - data.sessions()[0].started(),
            30
        );
        data.start_task("task-1".into(), 50);
        assert!(data.end_active(80));
        data.save_note("Brief".into());
        assert_eq!(data.sessions()[1].note(), "Brief");
    }
    #[test]
    fn task_names_are_validated_and_tasks_can_be_renamed() {
        let mut data = Data::defaults();
        assert_eq!(validate_task_name("  "), Err(TaskNameError));
        data.rename_task("task-1", validate_task_name("  Planning ").unwrap());
        data.add_task("task-5".into(), validate_task_name("Review").unwrap());
        assert_eq!(data.task_title("task-1"), Some("Planning"));
        assert_eq!(data.tasks()[4].title(), "Review");
    }
    #[test]
    fn duration_range_and_markdown_are_readable() {
        let mut data = Data::defaults();
        data.sessions.push(Session {
            task_id: "task-1".into(),
            started: 0,
            ended: 2520,
            note: "Brief".into(),
        });
        assert_eq!(duration(2520), "42 min");
        assert!(in_range(&data.sessions()[0], Range::Year, 2520));
        assert!(markdown(&data, Range::Year, 2520).contains("Project Atlas"));
    }
}
