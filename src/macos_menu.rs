//! macOS application menu integration.
//!
//! Slint installs its own application menu whenever a Slint `MenuBar` is present.
//! Tempo owns the native menu instead so Settings and Quit live in one standard
//! application menu and the View menu can mirror the current projects.

use crate::{AppActions, AppWindow, NativeMenuStrings, Page};
use muda::{
    accelerator::{Accelerator, Code, Modifiers, CMD_OR_CTRL},
    Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu,
};
use slint::{ComponentHandle, Model};
use std::cell::RefCell;

const SETTINGS: &str = "tempo.settings";
const ABOUT: &str = "tempo.about";
const BACKUP: &str = "tempo.backup";
const RESTORE: &str = "tempo.restore";
const HOME: &str = "tempo.home";
const EVALUATION: &str = "tempo.evaluation";
const KEYBOARD_SHORTCUTS: &str = "tempo.keyboard-shortcuts";
const PROJECT_PREFIX: &str = "tempo.project:";

thread_local! {
    static MENU: RefCell<Option<Menu>> = const { RefCell::new(None) };
}

/// Installs Tempo's single native application menu.
pub(crate) fn install(ui: &AppWindow) {
    let weak_ui = ui.as_weak();
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        let weak_ui = weak_ui.clone();
        let _ = slint::invoke_from_event_loop(move || {
            let Some(ui) = weak_ui.upgrade() else {
                return;
            };
            dispatch(&ui, event.id.as_ref());
        });
    }));
    refresh(ui);
}

/// Rebuilds the menu when the projected project list changes.
pub(crate) fn refresh(ui: &AppWindow) {
    let menu = build_menu(ui);
    menu.init_for_nsapp();
    MENU.with(|stored| *stored.borrow_mut() = Some(menu));
}

/// Forwards an event delivered by the native menu to the established UI intent boundary.
fn dispatch(ui: &AppWindow, id: &str) {
    let actions = ui.global::<AppActions>();
    match id {
        ABOUT => actions.invoke_show_about(),
        SETTINGS => actions.invoke_navigate(Page::Settings),
        BACKUP => actions.invoke_backup_data(),
        RESTORE => actions.invoke_restore_data(),
        HOME => actions.invoke_navigate(Page::Home),
        EVALUATION => actions.invoke_navigate(Page::Evaluation),
        KEYBOARD_SHORTCUTS => actions.invoke_show_keyboard_shortcuts(),
        _ => {
            if let Some(project_id) = id.strip_prefix(PROJECT_PREFIX) {
                actions.invoke_start_tracking(project_id.into());
            }
        }
    }
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
    append(
        &application,
        &MenuItem::with_id(ABOUT, strings.get_about(), true, None),
    );
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

    let projects = ui.get_home_state().projects;
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
