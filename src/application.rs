//! Application-level orchestration for the tracker.
//!
//! [`Tracker`] coordinates domain mutations with SQLite persistence and holds
//! UI-only state such as the selected reporting range and open project editor.
//! It does not format data for Slint or handle widget callbacks.

use crate::{
    domain::{self, Data, ProjectId, Range, TaskId},
    error::Result,
    language::Language,
    persistence::SqliteStore,
    report,
};
use std::{cell::RefCell, rc::Rc};

#[cfg(test)]
use std::path::PathBuf;

/// Shared mutable access to the single tracker instance used by the UI.
pub(crate) type SharedTracker = Rc<RefCell<Tracker>>;

/// Coordinates domain state, persistence, and application-only interaction state.
pub(crate) struct Tracker {
    data: Data,
    store: SqliteStore,
    range: Range,
    editing_project_id: Option<ProjectId>,
    evaluating_project: Option<ProjectId>,
}

impl Tracker {
    /// Loads the default database, closes any restored active task, and saves that recovery.
    pub(crate) fn load_default() -> SharedTracker {
        let store = SqliteStore::at(SqliteStore::default_path());
        let mut data = store.load_or_default();
        data.recover_active();
        let tracker = Rc::new(RefCell::new(Self {
            data,
            store,
            range: Range::Day,
            editing_project_id: None,
            evaluating_project: None,
        }));
        let _ = tracker.borrow().save();
        tracker
    }
    #[cfg(test)]
    pub(crate) fn at(path: PathBuf) -> Self {
        Self {
            data: Data::defaults(),
            store: SqliteStore::at(path),
            range: Range::Day,
            editing_project_id: None,
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
        self.data
            .start_tracking(ProjectId::from(project_id), timestamp);
        self.save()
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
        let changed = self.data.toggle_pause(timestamp);
        if changed {
            self.save()?;
        }
        Ok(changed)
    }

    /// Finishes active work at `timestamp` and persists the completed task.
    ///
    /// Returns whether an active task was finished.
    pub(crate) fn end_tracking(&mut self, timestamp: i64) -> Result<bool> {
        let ended = self.data.end_tracking(timestamp);
        if ended {
            self.save()?;
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

    /// Clears any pending rename and returns an empty project-editor state.
    pub(crate) fn begin_add_project(&mut self) -> ProjectDialog {
        self.editing_project_id = None;
        ProjectDialog {
            rename_mode: false,
            initial_draft: String::new(),
            project_id: String::new(),
            archived: false,
        }
    }

    /// Selects a project for editing and returns its current editor state.
    ///
    /// An unknown identifier produces an empty draft but is retained as the
    /// pending identifier until the dialog is saved or cancelled.
    pub(crate) fn begin_rename_project(&mut self, id: String) -> ProjectDialog {
        let initial_draft = self
            .data
            .project_name_by_str(&id)
            .unwrap_or_default()
            .to_owned();
        let archived = self
            .data
            .projects()
            .iter()
            .find(|project| project.id().as_str() == id)
            .is_some_and(|project| project.archived());
        self.editing_project_id = Some(ProjectId::from(id));
        ProjectDialog {
            rename_mode: true,
            initial_draft,
            project_id: self
                .editing_project_id
                .as_ref()
                .map(|id| id.as_str().to_owned())
                .unwrap_or_default(),
            archived,
        }
    }

    /// Validates and saves either the pending rename or a new timestamp-based project.
    ///
    /// A successful save clears the pending rename; validation errors leave it
    /// available for further editing.
    pub(crate) fn save_project(&mut self, name: &str, timestamp: i64) -> Result<()> {
        let name = domain::validate_project_name(name)?;
        if let Some(id) = self.editing_project_id.take() {
            self.data.rename_project(&id, name);
        } else {
            self.data
                .add_project(ProjectId::from(format!("project-{timestamp}")), name);
        }
        self.save()?;
        Ok(())
    }

    /// Discards any pending project rename.
    pub(crate) fn cancel_project_dialog(&mut self) {
        self.editing_project_id = None;
    }

    /// Archives a project while retaining its recorded work, then saves.
    pub(crate) fn archive_project(&mut self, id: String) -> Result<()> {
        self.data.archive_project(&ProjectId::from(id));
        self.save()
    }

    /// Restores an archived project to the active lifecycle state, then saves.
    pub(crate) fn unarchive_project(&mut self, id: String) -> Result<()> {
        self.data.unarchive_project(&ProjectId::from(id));
        self.save()
    }

    /// Deletes a project and its work, clears pending editor state, then saves.
    pub(crate) fn delete_project(&mut self, id: String) -> Result<()> {
        let id = ProjectId::from(id);
        self.data.delete_project(&id);
        self.editing_project_id = None;
        self.save()
    }

    /// Renders the selected range as a localized Markdown report without saving.
    pub(crate) fn report(&self, timestamp: i64, language: Language) -> String {
        report::markdown(&self.data, self.range, timestamp, language)
    }
}

/// The UI-facing state needed to render the project editor.
pub(crate) struct ProjectDialog {
    pub(crate) rename_mode: bool,
    pub(crate) initial_draft: String,
    pub(crate) project_id: String,
    pub(crate) archived: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn application_actions_keep_range_and_project_editing_state() {
        let path = std::env::temp_dir().join(format!(
            "tempo-application-test-{}-{}.sqlite",
            std::process::id(),
            domain::now()
        ));
        let mut tracker = Tracker::at(path.clone());
        tracker.choose_range(Range::Month);
        assert_eq!(tracker.range(), Range::Month);
        assert!(!tracker.begin_add_project().rename_mode);
        tracker.save_project("Review", 100).unwrap();
        assert_eq!(tracker.data().projects()[4].name(), "Review");
        let dialog = tracker.begin_rename_project("project-1".into());
        assert!(dialog.rename_mode);
        assert_eq!(dialog.initial_draft, "Project Atlas");
        tracker.save_project("Planning", 101).unwrap();
        assert_eq!(
            tracker.data().project_name_by_str("project-1"),
            Some("Planning")
        );
        std::fs::remove_file(path).unwrap();
    }
}
