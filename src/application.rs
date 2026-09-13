//! Application-level orchestration for the tracker.
//!
//! [`Tracker`] coordinates domain mutations with SQLite persistence and holds
//! UI-only state such as the selected reporting range and evaluation selection.
//! It does not format data for Slint or handle widget callbacks.

use crate::{
    domain::{self, Data, ProjectId, Range, TaskId},
    error::Result,
    language::Language,
    persistence::SqliteStore,
    report,
};
#[cfg(test)]
use std::path::PathBuf;

/// Coordinates domain state, persistence, and application-only interaction state.
pub(crate) struct Tracker {
    data: Data,
    store: SqliteStore,
    range: Range,
    evaluating_project: Option<ProjectId>,
}

impl Tracker {
    /// Loads the default database and closes any restored active task in memory.
    pub(crate) fn load_default() -> Self {
        Self::load(SqliteStore::at(SqliteStore::default_path()))
    }
    fn load(store: SqliteStore) -> Self {
        let mut data = store.load_or_default();
        data.recover_active();
        Self {
            data,
            store,
            range: Range::Day,
            evaluating_project: None,
        }
    }
    #[cfg(test)]
    fn load_at(path: PathBuf) -> Self {
        Self::load(SqliteStore::at(path))
    }
    #[cfg(test)]
    pub(crate) fn at(path: PathBuf) -> Self {
        Self {
            data: Data::defaults(),
            store: SqliteStore::at(path),
            range: Range::Day,
            evaluating_project: None,
        }
    }
    pub(crate) fn data(&self) -> &Data {
        &self.data
    }

    pub(crate) fn range(&self) -> Range {
        self.range
    }

    /// Persists the complete current domain aggregate.
    pub(crate) fn save(&self) -> Result<()> {
        self.store.save(&self.data)
    }

    /// Starts tracking for a project and persists the resulting active task.
    pub(crate) fn start_tracking(&mut self, project_id: String, timestamp: i64) -> Result<()> {
        let mut next = self.data.clone();
        next.start_tracking(ProjectId::from(project_id), timestamp)?;
        self.store.save(&next)?;
        self.data = next;
        Ok(())
    }

    /// Checkpoints active, unpaused work and saves only when the checkpoint changed.
    pub(crate) fn tick(&mut self, timestamp: i64) -> Result<()> {
        if self.data.checkpoint_active(timestamp) {
            self.save()?;
        }
        Ok(())
    }

    /// Pauses or resumes active work and persists it when an active task exists.
    ///
    /// Returns whether the tracking state changed.
    pub(crate) fn toggle_tracking_pause(&mut self, timestamp: i64) -> Result<bool> {
        let mut next = self.data.clone();
        let changed = next.toggle_pause(timestamp);
        if changed {
            self.store.save(&next)?;
            self.data = next;
        }
        Ok(changed)
    }

    /// Finishes active work at `timestamp` and persists the completed task.
    ///
    /// Returns whether an active task was finished.
    pub(crate) fn end_tracking(&mut self, timestamp: i64) -> Result<bool> {
        let mut next = self.data.clone();
        let ended = next.end_tracking(timestamp);
        if ended {
            self.store.save(&next)?;
            self.data = next;
        }
        Ok(ended)
    }

    /// Sets the latest completed task's note, then persists the aggregate.
    pub(crate) fn save_task_note(&mut self, note: String) -> Result<()> {
        self.data.save_task_note(note);
        self.save()
    }

    /// Updates a completed task's note and persists only when the task exists.
    pub(crate) fn update_task_note(&mut self, id: String, note: String) -> Result<bool> {
        let updated = self.data.update_task_note(&TaskId::from(id), note);
        if updated {
            self.save()?;
        }
        Ok(updated)
    }

    /// Deletes one completed task and persists only when the task exists.
    pub(crate) fn delete_task(&mut self, id: String) -> Result<bool> {
        let deleted = self.data.delete_task(&TaskId::from(id));
        if deleted {
            self.save()?;
        }
        Ok(deleted)
    }

    /// Selects the in-memory range used by evaluation and report projections.
    pub(crate) fn choose_range(&mut self, range: Range) {
        self.range = range;
    }

    /// Selects a project whose tasks are shown in the evaluation detail view.
    pub(crate) fn select_evaluation_project(&mut self, project_id: String) {
        self.evaluating_project = Some(ProjectId::from(project_id));
    }

    /// Returns the project currently selected for evaluation detail, if any.
    pub(crate) fn evaluating_project(&self) -> Option<&ProjectId> {
        self.evaluating_project.as_ref()
    }

    /// Validates, renames, and saves an existing project.
    pub(crate) fn save_project(&mut self, id: String, name: &str) -> Result<()> {
        let name = domain::validate_project_name(name)?;
        self.update_projects(|data| data.rename_project(&ProjectId::from(id), name))
    }

    pub(crate) fn add_project(&mut self, name: &str, timestamp: i64) -> Result<()> {
        let name = domain::validate_project_name(name)?;
        self.update_projects(|data| {
            data.add_project(ProjectId::from(format!("project-{timestamp}")), name);
        })
    }

    /// Archives a project while retaining its recorded work, then saves.
    pub(crate) fn archive_project(&mut self, id: String) -> Result<()> {
        self.update_projects(|data| data.archive_project(&ProjectId::from(id)))
    }

    /// Restores an archived project to the active lifecycle state, then saves.
    pub(crate) fn unarchive_project(&mut self, id: String) -> Result<()> {
        self.update_projects(|data| data.unarchive_project(&ProjectId::from(id)))
    }

    /// Deletes a project and its work, then saves.
    pub(crate) fn delete_project(&mut self, id: String) -> Result<()> {
        self.update_projects(|data| data.delete_project(&ProjectId::from(id)))
    }

    /// Stage project changes so a failed save cannot leak into a later timer tick
    /// or make retrying creation insert the same project twice.
    fn update_projects(&mut self, update: impl FnOnce(&mut Data)) -> Result<()> {
        let mut next = self.data.clone();
        update(&mut next);
        self.store.save(&next)?;
        self.data = next;
        Ok(())
    }

    /// Renders the selected range and optional project scope without saving.
    pub(crate) fn export_report(
        &self,
        project_id: Option<&ProjectId>,
        timestamp: i64,
        language: Language,
        format: report::Format,
    ) -> std::result::Result<String, String> {
        report::render(
            &self.data, self.range, project_id, timestamp, language, format,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn application_actions_keep_range_and_project_data() {
        let path = std::env::temp_dir().join(format!(
            "tempo-application-test-{}-{}.sqlite",
            std::process::id(),
            domain::now()
        ));
        let mut tracker = Tracker::at(path.clone());
        tracker.choose_range(Range::Month);
        assert_eq!(tracker.range(), Range::Month);
        tracker.add_project("Review", 100).unwrap();
        assert_eq!(tracker.data().projects()[4].name(), "Review");
        tracker
            .save_project("project-1".into(), "Planning")
            .unwrap();

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn reopening_finishes_interrupted_work_without_resuming_either_task() {
        let path = std::env::temp_dir().join(format!(
            "tempo-application-recovery-test-{}-{}.sqlite",
            std::process::id(),
            domain::now()
        ));
        let mut tracker = Tracker::at(path.clone());
        tracker.start_tracking("project-1".into(), 10).unwrap();
        tracker.tick(40).unwrap();
        tracker.start_tracking("project-2".into(), 50).unwrap();
        tracker.tick(70).unwrap();

        let reopened = Tracker::load_at(path.clone());

        assert!(reopened.data().active_task().is_none());
        assert!(!reopened.data().has_interrupted_task());
        assert_eq!(reopened.data().tasks().len(), 2);
        assert_eq!(
            reopened.data().tasks()[0].ended() - reopened.data().tasks()[0].started(),
            40
        );
        assert_eq!(
            reopened.data().tasks()[1].ended() - reopened.data().tasks()[1].started(),
            20
        );

        std::fs::remove_file(path).unwrap();
    }
}
