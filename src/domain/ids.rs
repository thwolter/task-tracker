//! Distinct identifiers for projects and recorded tasks.
//!
//! They remain separate Rust types so a task identifier cannot be used where a
//! project identifier is required.

/// Identifies one project within the tracker data set.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProjectId(String);

impl ProjectId {
    pub(super) fn new(value: impl Into<String>) -> Self {
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

/// Identifies one completed or active task within the tracker data set.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TaskId(String);

impl TaskId {
    pub(super) fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for TaskId {
    fn from(value: String) -> Self {
        Self(value)
    }
}
