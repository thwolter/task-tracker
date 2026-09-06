#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod application;
mod domain;
mod error;
mod persistence;
mod presentation;
mod ui;

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let ui = AppWindow::new()?;

    let tracker = application::Tracker::load_default();
    let _timer = ui::bind(&ui, tracker);

    ui.run()
}
