use crate::domain::Data;
use std::{
    error::Error,
    fmt, fs,
    path::{Path, PathBuf},
};

pub(crate) struct JsonStore {
    path: PathBuf,
}
impl JsonStore {
    pub(crate) fn at(path: PathBuf) -> Self {
        Self { path }
    }
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
        base.join("Tempo").join("tempo.json")
    }
    pub(crate) fn load_or_default(&self) -> Data {
        fs::read_to_string(&self.path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_else(Data::defaults)
    }
    pub(crate) fn save(&self, data: &Data) -> Result<(), PersistenceError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let text = serde_json::to_string_pretty(data)?;
        fs::write(&self.path, text)?;
        Ok(())
    }
}
pub(crate) fn export_markdown(path: &Path, report: &str) -> Result<(), PersistenceError> {
    fs::write(path, report)?;
    Ok(())
}
#[derive(Debug)]
pub(crate) enum PersistenceError {
    Io(std::io::Error),
    Json(serde_json::Error),
}
impl fmt::Display for PersistenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => error.fmt(f),
            Self::Json(error) => error.fmt(f),
        }
    }
}
impl Error for PersistenceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Json(error) => Some(error),
        }
    }
}
impl From<std::io::Error> for PersistenceError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}
impl From<serde_json::Error> for PersistenceError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Data;
    #[test]
    fn load_falls_back_and_saved_json_round_trips() {
        let path = std::env::temp_dir().join(format!(
            "tempo-persistence-test-{}-{}.json",
            std::process::id(),
            crate::domain::now()
        ));
        let store = JsonStore::at(path.clone());
        assert_eq!(store.load_or_default().tasks().len(), 4);
        store.save(&Data::defaults()).unwrap();
        assert_eq!(store.load_or_default().tasks()[0].id(), "task-1");
        assert!(
            fs::read_to_string(&path)
                .unwrap()
                .contains("\"active\": null")
        );
        fs::remove_file(path).unwrap();
    }
}
