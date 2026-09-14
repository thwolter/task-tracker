//! Native data backup and restore interactions.

use super::UiController;
use crate::{AppWindow, Page};
use rfd::{FileDialog, MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};
use slint::ComponentHandle;

impl UiController {
    #[cfg(all(target_os = "macos", not(test)))]
    pub(super) fn show_keyboard_shortcuts(&self, ui: &AppWindow) {
        let strings = ui.global::<crate::NativeMenuStrings>();
        MessageDialog::new()
            .set_title(strings.get_keyboard_shortcuts())
            .set_description(strings.get_keyboard_shortcuts_description())
            .set_buttons(MessageButtons::Ok)
            .show();
    }

    #[cfg(any(not(target_os = "macos"), test))]
    pub(super) fn show_keyboard_shortcuts(&self, _ui: &AppWindow) {}

    pub(super) fn backup_data(&mut self, ui: &AppWindow) {
        let strings = ui.global::<crate::NativeMenuStrings>();
        let Some(destination) = FileDialog::new()
            .set_title(strings.get_backup_dialog_title())
            .add_filter(strings.get_tempo_backup_label(), &["sqlite"])
            .set_file_name("tempo-backup.sqlite")
            .save_file()
        else {
            return;
        };

        match self.tracker.backup_to(&destination) {
            Ok(()) => self.set_success(ui, strings.get_backup_completed_message()),
            Err(error) => self.set_error(
                ui,
                format!("{}: {error}", strings.get_backup_failed_message()),
            ),
        }
    }

    pub(super) fn restore_data(&mut self, ui: &AppWindow) {
        let strings = ui.global::<crate::NativeMenuStrings>();
        let Some(source) = FileDialog::new()
            .set_title(strings.get_restore_dialog_title())
            .add_filter(strings.get_tempo_backup_label(), &["sqlite"])
            .pick_file()
        else {
            return;
        };

        let confirmed = MessageDialog::new()
            .set_level(MessageLevel::Warning)
            .set_title(strings.get_restore_confirmation_title())
            .set_description(strings.get_restore_confirmation_description())
            .set_buttons(MessageButtons::YesNo)
            .show()
            == MessageDialogResult::Yes;
        if !confirmed {
            return;
        }

        match self.tracker.restore_from(&source) {
            Ok(()) => {
                self.export_context = None;
                ui.set_project_editor(Default::default());
                self.refresh(ui);
                ui.set_current_page(Page::Home);
                self.set_success(ui, strings.get_restore_completed_message());
            }
            Err(error) => self.set_error(
                ui,
                format!("{}: {error}", strings.get_restore_failed_message()),
            ),
        }
    }
}
