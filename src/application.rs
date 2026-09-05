use crate::{
    domain::{self, Data, Range},
    error::Result,
    persistence::JsonStore,
};
use std::{cell::RefCell, rc::Rc};

#[cfg(test)]
use std::path::PathBuf;

pub(crate) type SharedTracker = Rc<RefCell<Tracker>>;

pub(crate) struct Tracker {
    data: Data,
    store: JsonStore,
    range: Range,
    editing_id: Option<String>,
}

impl Tracker {
    pub(crate) fn load_default() -> SharedTracker {
        let store = JsonStore::at(JsonStore::default_path());
        let mut data = store.load_or_default();
        data.recover_active();
        let tracker = Rc::new(RefCell::new(Self {
            data,
            store,
            range: Range::Day,
            editing_id: None,
        }));
        let _ = tracker.borrow().save();
        tracker
    }
    #[cfg(test)]
    pub(crate) fn at(path: PathBuf) -> Self {
        Self {
            data: Data::defaults(),
            store: JsonStore::at(path),
            range: Range::Day,
            editing_id: None,
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

    pub(crate) fn start_task(&mut self, id: String, timestamp: i64) -> Result<()> {
        self.data.start_task(id, timestamp);
        self.save()
    }

    pub(crate) fn tick(&mut self, timestamp: i64) -> Result<()> {
        if self.data.checkpoint_active(timestamp) {
            self.save()?;
        }
        Ok(())
    }

    pub(crate) fn end_task(&mut self, timestamp: i64) -> Result<bool> {
        let ended = self.data.end_active(timestamp);
        if ended {
            self.save()?;
        }
        Ok(ended)
    }

    pub(crate) fn save_note(&mut self, note: String) -> Result<()> {
        self.data.save_note(note);
        self.save()
    }

    pub(crate) fn choose_range(&mut self, range: Range) {
        self.range = range;
    }

    pub(crate) fn begin_add_task(&mut self) -> ProjectDialog {
        self.editing_id = None;
        ProjectDialog {
            rename_mode: false,
            initial_draft: String::new(),
        }
    }

    pub(crate) fn begin_rename_task(&mut self, id: String) -> ProjectDialog {
        let initial_draft = self.data.task_title(&id).unwrap_or_default().to_owned();
        self.editing_id = Some(id);
        ProjectDialog {
            rename_mode: true,
            initial_draft,
        }
    }

    pub(crate) fn save_task(&mut self, name: &str, timestamp: i64) -> Result<()> {
        let name = domain::validate_task_name(name)?;
        if let Some(id) = self.editing_id.take() {
            self.data.rename_task(&id, name);
        } else {
            self.data.add_task(format!("task-{timestamp}"), name);
        }
        self.save()?;
        Ok(())
    }

    pub(crate) fn cancel_task_dialog(&mut self) {
        self.editing_id = None;
    }

    pub(crate) fn report(&self, timestamp: i64) -> String {
        domain::markdown(&self.data, self.range, timestamp)
    }
}

pub(crate) struct ProjectDialog {
    pub(crate) rename_mode: bool,
    pub(crate) initial_draft: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn application_actions_keep_range_and_task_editing_state() {
        let path = std::env::temp_dir().join(format!(
            "tempo-application-test-{}-{}.json",
            std::process::id(),
            domain::now()
        ));
        let mut tracker = Tracker::at(path.clone());
        tracker.choose_range(Range::Month);
        assert_eq!(tracker.range(), Range::Month);
        assert!(!tracker.begin_add_task().rename_mode);
        tracker.save_task("Review", 100).unwrap();
        assert_eq!(tracker.data().tasks()[4].title(), "Review");
        let dialog = tracker.begin_rename_task("task-1".into());
        assert!(dialog.rename_mode);
        assert_eq!(dialog.initial_draft, "Project Atlas");
        tracker.save_task("Planning", 101).unwrap();
        assert_eq!(tracker.data().task_title("task-1"), Some("Planning"));
        std::fs::remove_file(path).unwrap();
    }
}
