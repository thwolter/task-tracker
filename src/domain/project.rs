use crate::error::{Result, TrackerError};
use serde::{Deserialize, Serialize};

use super::ProjectId;

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Project {
    id: ProjectId,
    name: String,
    #[serde(default)]
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

pub(crate) fn validate_project_name(name: &str) -> Result<String> {
    let name = name.trim();
    (!name.is_empty())
        .then(|| name.to_owned())
        .ok_or(TrackerError::EmptyProjectName)
}
