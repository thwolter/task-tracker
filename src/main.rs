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
