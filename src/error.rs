use thiserror::Error;

#[derive(Debug, Error)]
pub enum TrackerError {
    #[error("Task name cannot be empty")]
    EmptyTaskName,

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, TrackerError>;
