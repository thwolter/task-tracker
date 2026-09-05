use crate::{
    AppWindow, Page, TaskDialogState,
    application::{SharedTracker, TaskDialog},
    domain, persistence, presentation,
};
use rfd::FileDialog;
use slint::{ComponentHandle, SharedString, Timer, TimerMode, Weak};
use std::time::Duration;

pub(crate) fn bind(ui: &AppWindow, tracker: SharedTracker) -> Timer {
    let controller = UiController::new(ui, tracker);
    controller.refresh(ui);
    controller.persist_initial(ui);
    let timer = controller.start_timer();
    bind_callbacks(ui, &controller);
    timer
}

#[derive(Clone)]
struct UiController {
    ui: Weak<AppWindow>,
    tracker: SharedTracker,
}

impl UiController {
    fn new(ui: &AppWindow, tracker: SharedTracker) -> Self {
        Self {
            ui: ui.as_weak(),
            tracker,
        }
    }

    fn start_timer(&self) -> Timer {
        let timer = Timer::default();
        let controller = self.clone();
        timer.start(TimerMode::Repeated, Duration::from_secs(1), move || {
            controller.tick();
        });
        timer
    }

    fn tick(&self) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        let result = self.tracker.borrow_mut().tick(domain::now());
        if let Err(error) = result {
            self.set_status(&ui, format!("Could not save data: {error}"));
        }
        self.refresh(&ui);
    }

    fn start_task(&self, id: SharedString) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        let result = self
            .tracker
            .borrow_mut()
            .start_task(id.to_string(), domain::now());
        if let Err(error) = result {
            self.set_status(&ui, format!("Could not save data: {error}"));
        }
        ui.set_page(Page::Tracking);
        self.refresh(&ui);
    }

    fn open_last_session(&self) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        if self.tracker.borrow().data().has_sessions() {
            ui.set_page(Page::Note);
        }
    }

    fn end_task(&self) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        match self.tracker.borrow_mut().end_task(domain::now()) {
            Ok(true) => ui.set_page(Page::Note),
            Ok(false) => {}
            Err(error) => self.set_status(&ui, format!("Could not save data: {error}")),
        }
        self.refresh(&ui);
    }

    fn save_note(&self, note: SharedString) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        let result = self.tracker.borrow_mut().save_note(note.to_string());
        if let Err(error) = result {
            self.set_status(&ui, format!("Could not save data: {error}"));
        }
        ui.set_page(Page::Home);
        self.refresh(&ui);
    }

    fn skip_note(&self) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        let result = self.tracker.borrow().skip_note();
        if let Err(error) = result {
            self.set_status(&ui, format!("Could not save data: {error}"));
        }
        ui.set_page(Page::Home);
        self.refresh(&ui);
    }

    fn choose_range(&self, range: crate::Range) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        self.tracker
            .borrow_mut()
            .choose_range(presentation::domain_range(range));
        self.refresh(&ui);
    }

    fn open_add_task(&self) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        let dialog = self.tracker.borrow_mut().begin_add_task();
        self.show_task_dialog(&ui, dialog);
    }

    fn open_rename_task(&self, id: SharedString) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        let dialog = self.tracker.borrow_mut().begin_rename_task(id.to_string());
        self.show_task_dialog(&ui, dialog);
    }

    fn save_task(&self, name: SharedString) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        let result = self
            .tracker
            .borrow_mut()
            .save_task(name.as_str(), domain::now());
        match result {
            Ok(()) => {
                self.dismiss_task_dialog(&ui);
                self.refresh(&ui);
            }
            Err(error) => self.set_status(&ui, error.to_string()),
        }
    }

    fn close_task_dialog(&self) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        self.tracker.borrow_mut().cancel_task_dialog();
        self.dismiss_task_dialog(&ui);
    }

    fn export_markdown(&self) {
        let Some(ui) = self.ui.upgrade() else {
            return;
        };
        let Some(path) = FileDialog::new()
            .add_filter("Markdown", &["md"])
            .set_file_name("tempo-report.md")
            .save_file()
        else {
            return;
        };
        let report = self.tracker.borrow().report(domain::now());
        match persistence::export_markdown(&path, &report) {
            Ok(()) => self.set_status(&ui, "Markdown exported"),
            Err(error) => self.set_status(&ui, format!("Export failed: {error}")),
        }
    }

    fn refresh(&self, ui: &AppWindow) {
        presentation::refresh(ui, &self.tracker.borrow(), domain::now());
    }

    fn persist_initial(&self, ui: &AppWindow) {
        if let Err(error) = self.tracker.borrow().save() {
            self.set_status(ui, format!("Could not save data: {error}"));
        }
    }

    fn set_status(&self, ui: &AppWindow, message: impl Into<SharedString>) {
        ui.set_status(message.into());
    }

    fn show_task_dialog(&self, ui: &AppWindow, dialog: TaskDialog) {
        ui.set_task_dialog(TaskDialogState {
            open: true,
            rename_mode: dialog.rename_mode,
            initial_draft: dialog.initial_draft.into(),
        });
    }

    fn dismiss_task_dialog(&self, ui: &AppWindow) {
        ui.set_task_dialog(TaskDialogState {
            open: false,
            rename_mode: false,
            initial_draft: SharedString::new(),
        });
    }
}

fn bind_callbacks(ui: &AppWindow, controller: &UiController) {
    let start_task = controller.clone();
    ui.on_start_task(move |id| start_task.start_task(id));

    let open_last_session = controller.clone();
    ui.on_open_last_session(move || open_last_session.open_last_session());

    let end_task = controller.clone();
    ui.on_end_task(move || end_task.end_task());

    let save_note = controller.clone();
    ui.on_save_note(move |note| save_note.save_note(note));

    let skip_note = controller.clone();
    ui.on_skip_note(move || skip_note.skip_note());

    let choose_range = controller.clone();
    ui.on_choose_range(move |range| choose_range.choose_range(range));

    let open_add_task = controller.clone();
    ui.on_open_add_task(move || open_add_task.open_add_task());

    let open_rename_task = controller.clone();
    ui.on_open_rename_task(move |id| open_rename_task.open_rename_task(id));

    let save_task = controller.clone();
    ui.on_save_task(move |name| save_task.save_task(name));

    let close_task_dialog = controller.clone();
    ui.on_close_task_dialog(move || close_task_dialog.close_task_dialog());

    let export_markdown = controller.clone();
    ui.on_export_markdown(move || export_markdown.export_markdown());
}

#[cfg(test)]
mod tests {
    use crate::{AppWindow, Page};

    #[test]
    fn settings_home_callback_returns_to_home() {
        i_slint_backend_testing::init_no_event_loop();
        let ui = AppWindow::new().unwrap();
        ui.set_page(Page::Settings);
        ui.invoke_open_home_view();
        assert_eq!(ui.get_page(), Page::Home);
    }
}
