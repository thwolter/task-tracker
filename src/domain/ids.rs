use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
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
