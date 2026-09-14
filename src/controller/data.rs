//! Native data backup and restore interactions.

use super::UiController;
use crate::{AppWindow, Page};
use rfd::{FileDialog, MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};
#[cfg(all(target_os = "macos", not(test)))]
use slint::ComponentHandle;

impl UiController {
    #[cfg(all(target_os = "macos", not(test)))]
    pub(super) fn poll_native_menu(&mut self, ui: &AppWindow) {
        while let Some(action) = crate::macos_menu::next_action() {
            match action {
                crate::macos_menu::NativeMenuAction::Settings => self.navigate(ui, Page::Settings),
                crate::macos_menu::NativeMenuAction::Backup => self.backup_data(ui),
                crate::macos_menu::NativeMenuAction::Restore => self.restore_data(ui),
                crate::macos_menu::NativeMenuAction::Navigate(page) => self.navigate(ui, page),
                crate::macos_menu::NativeMenuAction::StartProject(project_id) => {
                    self.start_tracking(ui, project_id.into())
                }
                crate::macos_menu::NativeMenuAction::KeyboardShortcuts => {
                    self.show_keyboard_shortcuts(ui)
                }
            }
        }
    }

    #[cfg(any(not(target_os = "macos"), test))]
    pub(super) fn poll_native_menu(&mut self, _ui: &AppWindow) {}

    #[cfg(all(target_os = "macos", not(test)))]
    fn show_keyboard_shortcuts(&self, ui: &AppWindow) {
        let strings = ui.global::<crate::NativeMenuStrings>();
        MessageDialog::new()
            .set_title(strings.get_keyboard_shortcuts())
            .set_description(strings.get_keyboard_shortcuts_description())
            .set_buttons(MessageButtons::Ok)
            .show();
    }

    pub(super) fn backup_data(&mut self, ui: &AppWindow) {
        let Some(destination) = FileDialog::new()
            .set_title(ui.get_backup_dialog_title())
            .add_filter(ui.get_tempo_backup_label(), &["sqlite"])
            .set_file_name("tempo-backup.sqlite")
            .save_file()
        else {
            return;
        };

        match self.tracker.backup_to(&destination) {
            Ok(()) => self.set_success(ui, ui.get_backup_completed_message()),
            Err(error) => {
                self.set_error(ui, format!("{}: {error}", ui.get_backup_failed_message()))
            }
        }
    }

    pub(super) fn restore_data(&mut self, ui: &AppWindow) {
        let Some(source) = FileDialog::new()
            .set_title(ui.get_restore_dialog_title())
            .add_filter(ui.get_tempo_backup_label(), &["sqlite"])
            .pick_file()
        else {
            return;
        };

        let confirmed = MessageDialog::new()
            .set_level(MessageLevel::Warning)
            .set_title(ui.get_restore_confirmation_title())
            .set_description(ui.get_restore_confirmation_description())
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
                self.set_success(ui, ui.get_restore_completed_message());
            }
            Err(error) => {
                self.set_error(ui, format!("{}: {error}", ui.get_restore_failed_message()))
            }
        }
    }
}
