use crate::domain::Data;
use crate::error::Result;
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Stores the tracker data as one JSON file at a fixed filesystem path.
pub(crate) struct JsonStore {
    path: PathBuf,
}

impl JsonStore {
    /// Creates a store that reads from and writes to `path`.
    pub(crate) fn at(path: PathBuf) -> Self {
        Self { path }
    }

    /// Returns the platform-appropriate location of Tempo's default data file.
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
        base.join("Tempo").join("tempo.json")
    }

    /// Loads the saved data, returning the default data when it cannot be read
    /// or deserialized.
    pub(crate) fn load_or_default(&self) -> Data {
        fs::read_to_string(&self.path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_else(Data::defaults)
    }

    /// Serializes `data` as formatted JSON, creating parent directories first.
    pub(crate) fn save(&self, data: &Data) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let text = serde_json::to_string_pretty(data)?;
        fs::write(&self.path, text)?;
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
    use crate::domain::Data;

    #[test]
    fn load_falls_back_and_saved_json_round_trips() {
        let path = std::env::temp_dir().join(format!(
            "tempo-persistence-test-{}-{}.json",
            std::process::id(),
            crate::domain::now()
        ));

        let store = JsonStore::at(path.clone());

        assert_eq!(store.load_or_default().projects().len(), 4);

        store.save(&Data::defaults()).unwrap();
        assert_eq!(
            store.load_or_default().projects()[0].id().as_str(),
            "project-1"
        );
        assert!(
            fs::read_to_string(&path)
                .unwrap()
                .contains("\"active_task\": null")
        );

        fs::remove_file(path).unwrap();
    }
}
