//! SQLite persistence for the complete tracker aggregate.
//!
//! The store owns schema initialization and maps SQLite rows to domain values;
//! it does not enforce domain lifecycle rules. Each save replaces all stored
//! aggregate rows inside one transaction.

use crate::{
    domain::{ActiveTask, Data, Project, ProjectId, Task, TaskId},
    error::{Result, TrackerError},
};
use rusqlite::{params, Connection, OpenFlags, OptionalExtension, MAIN_DB};
use std::{
    fs::{self, OpenOptions},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
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
        Self::default_data_dir().join("tempo.sqlite")
    }

    /// Returns the platform-appropriate directory for Tempo's local state.
    pub(crate) fn default_data_dir() -> PathBuf {
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
        base.join("Tempo")
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

    /// Creates a consistent SQLite snapshot at the selected destination.
    pub(crate) fn backup_to(&self, destination: &Path) -> Result<()> {
        self.connection()?.backup(MAIN_DB, destination, None)?;
        Ok(())
    }

    /// Reads a user-provided backup without initializing or modifying it.
    pub(crate) fn load_backup(path: &Path) -> Result<Data> {
        let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|_| TrackerError::InvalidBackup)?;
        Self::validate_backup_schema(&connection).map_err(|_| TrackerError::InvalidBackup)?;
        Self::load_from_connection(&connection, true).map_err(|_| TrackerError::InvalidBackup)
    }

    /// Atomically replaces the live database with a fully validated aggregate.
    ///
    /// The replacement is written to a sibling file first, so failures leave the
    /// current database untouched until the final rename.
    pub(crate) fn replace_with(&self, data: &Data) -> Result<()> {
        let staged = StagedDatabase::new(&self.path)?;
        let staged_store = Self::at(staged.path().to_owned());
        staged_store.save(data)?;
        Self::load_backup(staged.path())?;
        fs::rename(staged.path(), &self.path)?;
        staged.commit();
        Ok(())
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
        Self::load_from_connection(&connection, false)
    }

    fn load_from_connection(connection: &Connection, require_projects: bool) -> Result<Data> {
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

        if projects.is_empty() && !require_projects {
            return Ok(Data::defaults());
        }
        if projects.is_empty() {
            return Err(TrackerError::InvalidBackup);
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

    fn validate_backup_schema(connection: &Connection) -> Result<()> {
        for table in ["projects", "tasks", "active_task", "interrupted_task"] {
            let exists = connection.query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
                [table],
                |row| row.get::<_, i64>(0),
            )?;
            if exists == 0 {
                return Err(TrackerError::InvalidBackup);
            }
        }

        for table in ["tasks", "active_task", "interrupted_task"] {
            let mut statement = connection.prepare(&format!("PRAGMA foreign_key_list({table})"))?;
            let mut rows = statement.query([])?;
            let mut references_projects = false;
            while let Some(row) = rows.next()? {
                if row.get::<_, String>(2)? == "projects" {
                    references_projects = true;
                }
            }
            if !references_projects {
                return Err(TrackerError::InvalidBackup);
            }
        }

        let mut statement = connection.prepare("PRAGMA foreign_key_check")?;
        if statement.query([])?.next()?.is_some() {
            return Err(TrackerError::InvalidBackup);
        }
        Ok(())
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

struct StagedDatabase {
    path: PathBuf,
    committed: bool,
}

impl StagedDatabase {
    fn new(live_path: &Path) -> Result<Self> {
        let parent = live_path
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent)?;
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("the system clock is after the Unix epoch")
            .as_nanos();
        let stem = live_path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("tempo");

        for attempt in 0..100 {
            let path = parent.join(format!(
                ".{stem}.restore-{}-{timestamp}-{attempt}.sqlite",
                std::process::id()
            ));
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(_) => {
                    return Ok(Self {
                        path,
                        committed: false,
                    });
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error.into()),
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "could not reserve a temporary restore database",
        )
        .into())
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn commit(mut self) {
        self.committed = true;
    }
}

impl Drop for StagedDatabase {
    fn drop(&mut self) {
        if !self.committed {
            let _ = fs::remove_file(&self.path);
        }
    }
}

/// Writes already-rendered export content to `path`.
pub(crate) fn export_report(path: &Path, contents: &str) -> Result<()> {
    fs::write(path, contents)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "tempo-{name}-{}-{}.sqlite",
            std::process::id(),
            crate::domain::now()
        ))
    }

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

    #[test]
    fn backup_round_trips_a_consistent_sqlite_snapshot() {
        let path = test_path("backup-source");
        let backup = test_path("backup-destination");
        let store = SqliteStore::at(path.clone());
        let mut data = Data::defaults();
        data.start_tracking(ProjectId::from("project-1".to_owned()), 10)
            .unwrap();
        assert!(data.end_tracking(40));
        data.save_task_note("Planning".into());
        store.save(&data).unwrap();

        store.backup_to(&backup).unwrap();
        let restored = SqliteStore::load_backup(&backup).unwrap();
        assert_eq!(restored.tasks()[0].note(), "Planning");

        fs::remove_file(path).unwrap();
        fs::remove_file(backup).unwrap();
    }

    #[test]
    fn invalid_backup_is_rejected_without_changing_live_data() {
        let path = test_path("invalid-live");
        let invalid = test_path("invalid-source");
        let store = SqliteStore::at(path.clone());
        let mut data = Data::defaults();
        data.start_tracking(ProjectId::from("project-1".to_owned()), 10)
            .unwrap();
        assert!(data.end_tracking(40));
        data.save_task_note("Keep me".into());
        store.save(&data).unwrap();
        fs::write(&invalid, "not a SQLite database").unwrap();

        assert!(matches!(
            SqliteStore::load_backup(&invalid),
            Err(TrackerError::InvalidBackup)
        ));
        assert_eq!(store.load_or_default().tasks()[0].note(), "Keep me");

        fs::remove_file(path).unwrap();
        fs::remove_file(invalid).unwrap();
    }

    #[test]
    fn backup_with_broken_foreign_keys_is_rejected() {
        let path = test_path("broken-foreign-key");
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "
                PRAGMA foreign_keys = OFF;
                CREATE TABLE projects (id TEXT PRIMARY KEY NOT NULL, name TEXT NOT NULL, archived INTEGER NOT NULL);
                CREATE TABLE tasks (id TEXT PRIMARY KEY NOT NULL, project_id TEXT NOT NULL REFERENCES projects(id), name TEXT, started INTEGER NOT NULL, ended INTEGER NOT NULL, note TEXT NOT NULL);
                CREATE TABLE active_task (singleton INTEGER PRIMARY KEY, id TEXT NOT NULL, project_id TEXT NOT NULL REFERENCES projects(id), started INTEGER NOT NULL, checkpoint INTEGER NOT NULL, paused INTEGER NOT NULL);
                CREATE TABLE interrupted_task (singleton INTEGER PRIMARY KEY, id TEXT NOT NULL, project_id TEXT NOT NULL REFERENCES projects(id), started INTEGER NOT NULL, checkpoint INTEGER NOT NULL, paused INTEGER NOT NULL);
                INSERT INTO projects (id, name, archived) VALUES ('project-1', 'Planning', 0);
                INSERT INTO tasks (id, project_id, name, started, ended, note) VALUES ('task-1', 'missing-project', NULL, 10, 20, 'Broken');
                ",
            )
            .unwrap();
        drop(connection);

        assert!(matches!(
            SqliteStore::load_backup(&path),
            Err(TrackerError::InvalidBackup)
        ));
        fs::remove_file(path).unwrap();
    }
}
