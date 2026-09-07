//! Completed tasks and the mutable state of an active tracking session.
//!
//! Timestamps are Unix seconds. An active task uses its checkpoint and paused
//! state to exclude paused intervals before it becomes an immutable completed
//! task.

use super::{ProjectId, TaskId};

/// A completed interval of tracked work associated with one project.
#[derive(Clone)]
pub(crate) struct Task {
    id: TaskId,
    project_id: ProjectId,
    name: Option<String>,
    started: i64,
    ended: i64,
    note: String,
}

impl Task {
    pub(super) fn from_active(active: ActiveTask, ended: i64) -> Self {
        Self {
            id: active.id,
            project_id: active.project_id,
            name: None,
            started: active.started,
            ended,
            note: String::new(),
        }
    }
    /// Reconstructs a completed task from persistence without changing its timestamps.
    pub(crate) fn from_storage(
        id: TaskId,
        project_id: ProjectId,
        name: Option<String>,
        started: i64,
        ended: i64,
        note: String,
    ) -> Self {
        Self {
            id,
            project_id,
            name,
            started,
            ended,
            note,
        }
    }
    pub(crate) fn id(&self) -> &TaskId {
        &self.id
    }
    pub(crate) fn name(&self) -> Option<&str> {
        self.name.as_deref()
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
    pub(super) fn set_note(&mut self, note: String) {
        self.note = note;
    }
}

/// The single task currently being tracked, if the aggregate has one.
///
/// When paused, `checkpoint` marks the end of counted work; resuming shifts
/// `started` forward to exclude the paused interval from the final duration.
#[derive(Clone)]
pub(crate) struct ActiveTask {
    id: TaskId,
    project_id: ProjectId,
    started: i64,
    checkpoint: i64,
    paused: bool,
}

impl ActiveTask {
    pub(super) fn new(id: TaskId, project_id: ProjectId, timestamp: i64) -> Self {
        Self {
            id,
            project_id,
            started: timestamp,
            checkpoint: timestamp,
            paused: false,
        }
    }
    /// Reconstructs an active task from its last persisted tracking state.
    pub(crate) fn from_storage(
        id: TaskId,
        project_id: ProjectId,
        started: i64,
        checkpoint: i64,
        paused: bool,
    ) -> Self {
        Self {
            id,
            project_id,
            started,
            checkpoint,
            paused,
        }
    }
    pub(crate) fn id(&self) -> &TaskId {
        &self.id
    }
    pub(crate) fn project_id(&self) -> &ProjectId {
        &self.project_id
    }
    pub(crate) fn started(&self) -> i64 {
        self.started
    }
    pub(crate) fn checkpoint(&self) -> i64 {
        self.checkpoint
    }
    pub(crate) fn paused(&self) -> bool {
        self.paused
    }
    /// Returns elapsed tracked seconds at `timestamp`, excluding a pause in progress.
    pub(crate) fn elapsed_until(&self, timestamp: i64) -> i64 {
        self.checkpoint - self.started
            + if self.paused {
                0
            } else {
                timestamp - self.checkpoint
            }
    }
    /// Moves the elapsed-work checkpoint when tracking is not paused.
    ///
    /// Returns `false` without changing state while paused.
    pub(super) fn checkpoint_at(&mut self, timestamp: i64) -> bool {
        if self.paused {
            return false;
        }
        self.checkpoint = timestamp;
        true
    }
    /// Pauses or resumes at `timestamp`, excluding a completed pause on resume.
    pub(super) fn toggle_pause(&mut self, timestamp: i64) {
        if self.paused {
            self.started += timestamp - self.checkpoint;
            self.checkpoint = timestamp;
            self.paused = false;
        } else {
            self.checkpoint = timestamp;
            self.paused = true;
        }
    }
    /// Finishes tracking, using the checkpoint rather than `timestamp` if paused.
    pub(super) fn into_task(self, timestamp: i64) -> Task {
        let ended = if self.paused {
            self.checkpoint
        } else {
            timestamp
        };
        Task::from_active(self, ended)
    }
    /// Turns restored active work into a completed task at its last checkpoint.
    pub(super) fn recover(self) -> Task {
        let ended = self.checkpoint;
        Task::from_active(self, ended)
    }
}
