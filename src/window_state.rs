//! Persistence for the main window's user-selected geometry.
//!
//! Window state is deliberately separate from the task database: failure to
//! read or write it must never prevent Tempo from opening or saving work.

use crate::{persistence::SqliteStore, AppWindow};
use serde::{Deserialize, Serialize};
use slint::{CloseRequestResponse, ComponentHandle, LogicalPosition, LogicalSize, Window};
use std::{
    fs,
    path::{Path, PathBuf},
};

const FILE_NAME: &str = "window-state.json";
const MINIMUM_WIDTH: f32 = 100.0;
const MINIMUM_HEIGHT: f32 = 100.0;

#[derive(Debug, Deserialize, Serialize)]
struct WindowState {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    maximized: bool,
}

impl WindowState {
    fn from_window(window: &Window) -> Self {
        let scale_factor = window.scale_factor();
        let position = window.position().to_logical(scale_factor);
        let size = window.size().to_logical(scale_factor);
        Self {
            x: position.x,
            y: position.y,
            width: size.width,
            height: size.height,
            maximized: window.is_maximized(),
        }
    }

    fn is_valid(&self) -> bool {
        self.x.is_finite()
            && self.y.is_finite()
            && self.width.is_finite()
            && self.height.is_finite()
            && self.width >= MINIMUM_WIDTH
            && self.height >= MINIMUM_HEIGHT
    }

    fn apply(&self, window: &Window) {
        window.set_size(LogicalSize::new(self.width, self.height));
        window.set_position(LogicalPosition::new(self.x, self.y));
        window.set_maximized(self.maximized);
    }
}

/// Restores a valid saved geometry before the window is first displayed.
pub(crate) fn restore(ui: &AppWindow) {
    let Ok(contents) = fs::read_to_string(default_path()) else {
        return;
    };
    let Ok(state) = serde_json::from_str::<WindowState>(&contents) else {
        return;
    };
    if state.is_valid() {
        state.apply(&ui.window());
    }
}

/// Records geometry when the user closes the main window without altering the
/// platform's default close behavior.
pub(crate) fn save_on_close(ui: &AppWindow) {
    let weak_ui = ui.as_weak();
    ui.window().on_close_requested(move || {
        if let Some(ui) = weak_ui.upgrade() {
            let _ = save(&default_path(), &WindowState::from_window(&ui.window()));
        }
        CloseRequestResponse::HideWindow
    });
}

fn default_path() -> PathBuf {
    SqliteStore::default_data_dir().join(FILE_NAME)
}

fn save(path: &Path, state: &WindowState) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let contents = serde_json::to_vec(state).expect("window state is serializable");
    fs::write(path, contents)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_round_trips_and_rejects_unusable_dimensions() {
        let path = std::env::temp_dir().join(format!(
            "tempo-window-state-test-{}-{}.json",
            std::process::id(),
            crate::domain::now()
        ));
        let state = WindowState {
            x: 30.0,
            y: 40.0,
            width: 800.0,
            height: 600.0,
            maximized: false,
        };

        save(&path, &state).unwrap();
        let restored: WindowState = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert!(restored.is_valid());
        assert_eq!(restored.width, 800.0);

        let too_small = WindowState {
            width: 10.0,
            ..restored
        };
        assert!(!too_small.is_valid());
        fs::remove_file(path).unwrap();
    }
}
