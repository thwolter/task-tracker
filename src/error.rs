use thiserror::Error;

#[derive(Debug, Error)]
pub enum TrackerError {
    #[error("Project name cannot be empty")]
    EmptyProjectName,

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, TrackerError>;
