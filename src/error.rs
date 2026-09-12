use thiserror::Error;

#[derive(Debug, Error)]
pub enum TrackerError {
    #[error("Project name cannot be empty")]
    EmptyProjectName,

    #[error("Finish the interrupted task before starting another one")]
    SecondaryTaskAlreadyActive,

    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, TrackerError>;
