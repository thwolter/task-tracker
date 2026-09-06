use crate::{
    domain::{self, Data, ProjectId, Range},
    error::Result,
    persistence::SqliteStore,
};
use std::{cell::RefCell, rc::Rc};

#[cfg(test)]
use std::path::PathBuf;

pub(crate) type SharedTracker = Rc<RefCell<Tracker>>;

pub(crate) struct Tracker {
    data: Data,
    store: SqliteStore,
    range: Range,
    editing_project_id: Option<ProjectId>,
}

impl Tracker {
    pub(crate) fn load_default() -> SharedTracker {
        let store = SqliteStore::at(SqliteStore::default_path());
        let mut data = store.load_or_default();
        data.recover_active();
        let tracker = Rc::new(RefCell::new(Self {
            data,
            store,
            range: Range::Day,
            editing_project_id: None,
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
        }
    }
    pub(crate) fn data(&self) -> &Data {
        &self.data
    }

    pub(crate) fn range(&self) -> Range {
        self.range
    }

    pub(crate) fn save(&self) -> Result<()> {
        self.store.save(&self.data)
    }

    pub(crate) fn start_tracking(&mut self, project_id: String, timestamp: i64) -> Result<()> {
        self.data
            .start_tracking(ProjectId::from(project_id), timestamp);
        self.save()
    }

    pub(crate) fn tick(&mut self, timestamp: i64) -> Result<()> {
        if self.data.checkpoint_active(timestamp) {
            self.save()?;
        }
        Ok(())
    }

    pub(crate) fn toggle_tracking_pause(&mut self, timestamp: i64) -> Result<bool> {
        let changed = self.data.toggle_pause(timestamp);
        if changed {
            self.save()?;
        }
        Ok(changed)
    }

    pub(crate) fn end_tracking(&mut self, timestamp: i64) -> Result<bool> {
        let ended = self.data.end_tracking(timestamp);
        if ended {
            self.save()?;
        }
        Ok(ended)
    }

    pub(crate) fn save_task_note(&mut self, note: String) -> Result<()> {
        self.data.save_task_note(note);
        self.save()
    }

    pub(crate) fn choose_range(&mut self, range: Range) {
        self.range = range;
    }

    pub(crate) fn begin_add_project(&mut self) -> ProjectDialog {
        self.editing_project_id = None;
        ProjectDialog {
            rename_mode: false,
            initial_draft: String::new(),
            project_id: String::new(),
            archived: false,
        }
    }

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

    pub(crate) fn cancel_project_dialog(&mut self) {
        self.editing_project_id = None;
    }

    pub(crate) fn archive_project(&mut self, id: String) -> Result<()> {
        self.data.archive_project(&ProjectId::from(id));
        self.save()
    }

    pub(crate) fn unarchive_project(&mut self, id: String) -> Result<()> {
        self.data.unarchive_project(&ProjectId::from(id));
        self.save()
    }

    pub(crate) fn delete_project(&mut self, id: String) -> Result<()> {
        let id = ProjectId::from(id);
        self.data.delete_project(&id);
        self.editing_project_id = None;
        self.save()
    }

    pub(crate) fn report(&self, timestamp: i64) -> String {
        domain::markdown(&self.data, self.range, timestamp)
    }
}

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
