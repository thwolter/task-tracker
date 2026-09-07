//! Projects and the validation applied to their editable names.
//!
//! A project may be archived while its previously recorded tasks remain part
//! of the tracker history. Persistence supplies stable project identifiers.

use super::ProjectId;
use crate::error::{Result, TrackerError};

/// A named bucket to which tracked tasks belong.
///
/// Archiving affects project lifecycle state without deleting its historical tasks.
#[derive(Clone)]
pub(crate) struct Project {
    id: ProjectId,
    name: String,
    archived: bool,
}

impl Project {
    pub(super) fn new(id: ProjectId, name: String) -> Self {
        Self {
            id,
            name,
            archived: false,
        }
    }
    /// Reconstructs a project including its persisted archive state.
    pub(crate) fn from_storage(id: ProjectId, name: String, archived: bool) -> Self {
        Self { id, name, archived }
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
    pub(super) fn rename(&mut self, name: String) {
        self.name = name;
    }
    pub(super) fn archive(&mut self) {
        self.archived = true;
    }
    pub(super) fn unarchive(&mut self) {
        self.archived = false;
    }
}

/// Trims a project name and rejects an empty result.
pub(crate) fn validate_project_name(name: &str) -> Result<String> {
    let name = name.trim();
    (!name.is_empty())
        .then(|| name.to_owned())
        .ok_or(TrackerError::EmptyProjectName)
}
