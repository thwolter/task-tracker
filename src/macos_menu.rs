//! macOS application menu integration.
//!
//! Slint installs its own application menu whenever a Slint `MenuBar` is present.
//! Tempo owns the native menu instead so Settings and Quit live in one standard
//! application menu and the View menu can mirror the current projects.

use crate::{AppWindow, NativeMenuStrings, Page};
use muda::{
    accelerator::{Accelerator, Code, Modifiers, CMD_OR_CTRL},
    Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu,
};
use slint::{ComponentHandle, Model};
use std::cell::RefCell;

const SETTINGS: &str = "tempo.settings";
const BACKUP: &str = "tempo.backup";
const RESTORE: &str = "tempo.restore";
const HOME: &str = "tempo.home";
const EVALUATION: &str = "tempo.evaluation";
const KEYBOARD_SHORTCUTS: &str = "tempo.keyboard-shortcuts";
const PROJECT_PREFIX: &str = "tempo.project:";

thread_local! {
    static MENU: RefCell<Option<Menu>> = const { RefCell::new(None) };
}

pub(crate) enum NativeMenuAction {
    Settings,
    Backup,
    Restore,
    Navigate(Page),
    StartProject(String),
    KeyboardShortcuts,
}

/// Installs Tempo's single native application menu.
pub(crate) fn install(ui: &AppWindow) {
    refresh(ui);
}

/// Rebuilds the menu when the projected project list changes.
pub(crate) fn refresh(ui: &AppWindow) {
    let menu = build_menu(ui);
    menu.init_for_nsapp();
    MENU.with(|stored| *stored.borrow_mut() = Some(menu));
}

/// Drains native menu events on the Slint UI thread.
pub(crate) fn next_action() -> Option<NativeMenuAction> {
    for event in MenuEvent::receiver().try_iter() {
        let id = event.id.as_ref();
        let action = match id {
            SETTINGS => NativeMenuAction::Settings,
            BACKUP => NativeMenuAction::Backup,
            RESTORE => NativeMenuAction::Restore,
            HOME => NativeMenuAction::Navigate(Page::Home),
            EVALUATION => NativeMenuAction::Navigate(Page::Evaluation),
            KEYBOARD_SHORTCUTS => NativeMenuAction::KeyboardShortcuts,
            _ => match id.strip_prefix(PROJECT_PREFIX) {
                Some(project_id) => NativeMenuAction::StartProject(project_id.to_owned()),
                None => continue,
            },
        };
        return Some(action);
    }
    None
}

fn build_menu(ui: &AppWindow) -> Menu {
    let strings = ui.global::<NativeMenuStrings>();
    let menu = Menu::new();

    let application = Submenu::new(strings.get_application(), true);
    append(
        &application,
        &MenuItem::with_id(
            SETTINGS,
            strings.get_settings(),
            true,
            Some(Accelerator::new(Some(CMD_OR_CTRL), Code::Comma)),
        ),
    );
    append(&application, &PredefinedMenuItem::separator());
    append(&application, &PredefinedMenuItem::about(None, None));
    append(&application, &PredefinedMenuItem::separator());
    append(&application, &PredefinedMenuItem::services(None));
    append(&application, &PredefinedMenuItem::separator());
    append(&application, &PredefinedMenuItem::hide(None));
    append(&application, &PredefinedMenuItem::hide_others(None));
    append(&application, &PredefinedMenuItem::show_all(None));
    append(&application, &PredefinedMenuItem::separator());
    append(&application, &PredefinedMenuItem::quit(None));
    append(&menu, &application);

    let file = Submenu::new(strings.get_file(), true);
    append(
        &file,
        &MenuItem::with_id(
            BACKUP,
            strings.get_backup(),
            true,
            Some(Accelerator::new(Some(CMD_OR_CTRL), Code::KeyB)),
        ),
    );
    append(
        &file,
        &MenuItem::with_id(
            RESTORE,
            strings.get_restore(),
            true,
            Some(Accelerator::new(
                Some(CMD_OR_CTRL | Modifiers::SHIFT),
                Code::KeyR,
            )),
        ),
    );
    append(&menu, &file);

    let view = Submenu::new(strings.get_view(), true);
    append(
        &view,
        &MenuItem::with_id(HOME, strings.get_home(), true, None),
    );
    append(
        &view,
        &MenuItem::with_id(
            EVALUATION,
            strings.get_evaluation(),
            true,
            Some(Accelerator::new(Some(CMD_OR_CTRL), Code::KeyE)),
        ),
    );

    let projects = ui.get_home().projects;
    if projects.row_count() > 0 {
        append(&view, &PredefinedMenuItem::separator());
        append(&view, &MenuItem::new(strings.get_projects(), false, None));
        for index in 0..projects.row_count() {
            let Some(project) = projects.row_data(index) else {
                continue;
            };
            let accelerator = project_accelerator(index);
            append(
                &view,
                &MenuItem::with_id(
                    format!("{PROJECT_PREFIX}{}", project.id),
                    project.name,
                    true,
                    accelerator,
                ),
            );
        }
    }
    append(&menu, &view);

    let help = Submenu::new(strings.get_help(), true);
    append(
        &help,
        &MenuItem::with_id(
            KEYBOARD_SHORTCUTS,
            strings.get_keyboard_shortcuts(),
            true,
            None,
        ),
    );
    append(&menu, &help);
    menu
}

fn project_accelerator(index: usize) -> Option<Accelerator> {
    let code = match index {
        0 => Code::Digit1,
        1 => Code::Digit2,
        2 => Code::Digit3,
        3 => Code::Digit4,
        4 => Code::Digit5,
        5 => Code::Digit6,
        6 => Code::Digit7,
        7 => Code::Digit8,
        8 => Code::Digit9,
        _ => return None,
    };
    Some(Accelerator::new(Some(CMD_OR_CTRL), code))
}

trait MenuContainer {
    fn append_item(&self, item: &dyn muda::IsMenuItem);
}

impl MenuContainer for Menu {
    fn append_item(&self, item: &dyn muda::IsMenuItem) {
        self.append(item)
            .expect("Tempo's native menu definition is valid");
    }
}

impl MenuContainer for Submenu {
    fn append_item(&self, item: &dyn muda::IsMenuItem) {
        self.append(item)
            .expect("Tempo's native menu definition is valid");
    }
}

fn append(menu: &impl MenuContainer, item: &dyn muda::IsMenuItem) {
    menu.append_item(item);
}
