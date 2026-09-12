//! SQLite persistence for the complete tracker aggregate.
//!
//! The store owns schema initialization and maps SQLite rows to domain values;
//! it does not enforce domain lifecycle rules. Each save replaces all stored
//! aggregate rows inside one transaction.

use crate::{
    domain::{ActiveTask, Data, Project, ProjectId, Task, TaskId},
    error::Result,
};
use rusqlite::{Connection, OptionalExtension, params};
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Stores tracker data in one SQLite database at a fixed filesystem path.
pub(crate) struct SqliteStore {
    path: PathBuf,
}

impl SqliteStore {
    /// Creates a store that reads from and writes to `path`.
    pub(crate) fn at(path: PathBuf) -> Self {
        Self { path }
    }

    /// Returns the platform-appropriate location of Tempo's default database.
    ///
    /// Falls back to the system temporary directory when no suitable data-home
    /// environment variable is available.
    pub(crate) fn default_path() -> PathBuf {
        let base = if cfg!(target_os = "windows") {
            std::env::var_os("APPDATA").map(PathBuf::from)
        } else if cfg!(target_os = "macos") {
            std::env::var_os("HOME")
                .map(|path| PathBuf::from(path).join("Library/Application Support"))
        } else {
            std::env::var_os("XDG_DATA_HOME")
                .map(PathBuf::from)
                .or_else(|| {
                    std::env::var_os("HOME").map(|path| PathBuf::from(path).join(".local/share"))
                })
        }
        .unwrap_or_else(std::env::temp_dir);
        base.join("Tempo").join("tempo.sqlite")
    }

    fn connection(&self) -> Result<Connection> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let connection = Connection::open(&self.path)?;
        connection.execute_batch(
            "
            PRAGMA foreign_keys = ON;
            CREATE TABLE IF NOT EXISTS projects (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                archived INTEGER NOT NULL CHECK (archived IN (0, 1))
            );
            CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY NOT NULL,
                project_id TEXT NOT NULL REFERENCES projects(id),
                name TEXT,
                started INTEGER NOT NULL,
                ended INTEGER NOT NULL,
                note TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS active_task (
                singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
                id TEXT NOT NULL,
                project_id TEXT NOT NULL REFERENCES projects(id),
                started INTEGER NOT NULL,
                checkpoint INTEGER NOT NULL,
                paused INTEGER NOT NULL CHECK (paused IN (0, 1))
            );
            CREATE TABLE IF NOT EXISTS interrupted_task (
                singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
                id TEXT NOT NULL,
                project_id TEXT NOT NULL REFERENCES projects(id),
                started INTEGER NOT NULL,
                checkpoint INTEGER NOT NULL,
                paused INTEGER NOT NULL CHECK (paused IN (0, 1))
            );
            ",
        )?;
        Ok(connection)
    }

    /// Loads saved data, returning defaults when the database is empty or unreadable.
    ///
    /// Errors from opening, initializing, or reading the database are
    /// intentionally not surfaced through this startup-oriented operation.
    pub(crate) fn load_or_default(&self) -> Data {
        self.load().unwrap_or_else(|_| Data::defaults())
    }

    fn load(&self) -> Result<Data> {
        let connection = self.connection()?;
        let mut statement =
            connection.prepare("SELECT id, name, archived FROM projects ORDER BY rowid")?;
        let projects = statement
            .query_map([], |row| {
                Ok(Project::from_storage(
                    ProjectId::from(row.get::<_, String>(0)?),
                    row.get(1)?,
                    row.get::<_, i64>(2)? != 0,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        drop(statement);

        if projects.is_empty() {
            return Ok(Data::defaults());
        }

        let mut statement = connection.prepare(
            "SELECT id, project_id, name, started, ended, note FROM tasks ORDER BY rowid",
        )?;
        let tasks = statement
            .query_map([], |row| {
                Ok(Task::from_storage(
                    TaskId::from(row.get::<_, String>(0)?),
                    ProjectId::from(row.get::<_, String>(1)?),
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        let active_task = connection
            .query_row(
                "SELECT id, project_id, started, checkpoint, paused FROM active_task WHERE singleton = 1",
                [],
                |row| {
                    Ok(ActiveTask::from_storage(
                        TaskId::from(row.get::<_, String>(0)?),
                        ProjectId::from(row.get::<_, String>(1)?),
                        row.get(2)?,
                        row.get(3)?,
                        row.get::<_, i64>(4)? != 0,
                    ))
                },
            )
            .optional()?;

        let interrupted_task = connection
            .query_row(
                "SELECT id, project_id, started, checkpoint, paused FROM interrupted_task WHERE singleton = 1",
                [],
                |row| {
                    Ok(ActiveTask::from_storage(
                        TaskId::from(row.get::<_, String>(0)?),
                        ProjectId::from(row.get::<_, String>(1)?),
                        row.get(2)?,
                        row.get(3)?,
                        row.get::<_, i64>(4)? != 0,
                    ))
                },
            )
            .optional()?;

        Ok(Data::from_storage(
            projects,
            tasks,
            active_task,
            interrupted_task,
        ))
    }

    /// Replaces the stored tracker state in one transaction.
    pub(crate) fn save(&self, data: &Data) -> Result<()> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        transaction.execute("DELETE FROM active_task", [])?;
        transaction.execute("DELETE FROM interrupted_task", [])?;
        transaction.execute("DELETE FROM tasks", [])?;
        transaction.execute("DELETE FROM projects", [])?;

        for project in data.projects() {
            transaction.execute(
                "INSERT INTO projects (id, name, archived) VALUES (?1, ?2, ?3)",
                params![
                    project.id().as_str(),
                    project.name(),
                    project.archived() as i64
                ],
            )?;
        }
        for task in data.tasks() {
            transaction.execute(
                "INSERT INTO tasks (id, project_id, name, started, ended, note) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    task.id().as_str(),
                    task.project_id().as_str(),
                    task.name(),
                    task.started(),
                    task.ended(),
                    task.note(),
                ],
            )?;
        }
        if let Some(active) = data.active_task() {
            transaction.execute(
                "INSERT INTO active_task (singleton, id, project_id, started, checkpoint, paused) VALUES (1, ?1, ?2, ?3, ?4, ?5)",
                params![
                    active.id().as_str(),
                    active.project_id().as_str(),
                    active.started(),
                    active.checkpoint(),
                    active.paused() as i64,
                ],
            )?;
        }
        if let Some(interrupted) = data.interrupted_task() {
            transaction.execute(
                "INSERT INTO interrupted_task (singleton, id, project_id, started, checkpoint, paused) VALUES (1, ?1, ?2, ?3, ?4, ?5)",
                params![
                    interrupted.id().as_str(),
                    interrupted.project_id().as_str(),
                    interrupted.started(),
                    interrupted.checkpoint(),
                    interrupted.paused() as i64,
                ],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }
}

/// Writes an already-rendered Markdown report to `path`.
pub(crate) fn export_markdown(path: &Path, report: &str) -> Result<()> {
    fs::write(path, report)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_falls_back_and_saved_database_round_trips() {
        let path = std::env::temp_dir().join(format!(
            "tempo-persistence-test-{}-{}.sqlite",
            std::process::id(),
            crate::domain::now()
        ));
        let store = SqliteStore::at(path.clone());

        assert_eq!(store.load_or_default().projects().len(), 4);
        let mut data = Data::defaults();
        data.start_tracking(ProjectId::from("project-1".to_owned()), 10)
            .unwrap();
        assert!(data.end_tracking(40));
        data.save_task_note("Planning".into());
        data.start_tracking(ProjectId::from("project-1".to_owned()), 45)
            .unwrap();
        data.start_tracking(ProjectId::from("project-2".to_owned()), 50)
            .unwrap();
        assert!(data.toggle_pause(60));

        store.save(&data).unwrap();
        let loaded = store.load_or_default();
        assert_eq!(loaded.projects()[0].id().as_str(), "project-1");
        assert_eq!(loaded.tasks().len(), 1);
        assert_eq!(loaded.tasks()[0].note(), "Planning");
        assert_eq!(
            loaded.active_task().unwrap().project_id().as_str(),
            "project-2"
        );
        assert!(loaded.active_task().unwrap().paused());
        assert!(loaded.has_interrupted_task());
        assert_eq!(
            loaded.interrupted_task().unwrap().project_id().as_str(),
            "project-1"
        );
        assert!(path.exists());

        fs::remove_file(path).unwrap();
    }
}
