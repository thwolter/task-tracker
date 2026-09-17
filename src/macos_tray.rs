//! macOS menu-bar timer integration.
//!
//! Slint owns the native `NSStatusItem`; this module only bridges its callbacks
//! to Tempo's existing application action boundary and keeps its projection in
//! step with the main window.

use crate::{AppActions, AppWindow, Page, TempoTray};
use slint::ComponentHandle;
use std::cell::RefCell;

thread_local! {
    // The platform removes a SystemTrayIcon when its component handle is
    // dropped, so retain the one application-wide instance for Tempo's life.
    static TRAY: RefCell<Option<TempoTray>> = const { RefCell::new(None) };
}

/// Creates the menu-bar extra and connects its native menu to AppActions.
pub(crate) fn install(ui: &AppWindow) {
    let tray = TempoTray::new().expect("Tempo's menu-bar tray can be created");
    let weak_ui = ui.as_weak();
    tray.on_open_tempo({
        let weak_ui = weak_ui.clone();
        move || open_tempo(&weak_ui)
    });
    tray.on_interrupt_with({
        let weak_ui = weak_ui.clone();
        move |project_id| {
            invoke(&weak_ui, |actions| {
                actions.invoke_start_tracking(project_id)
            })
        }
    });
    tray.on_end_tracking({
        let weak_ui = weak_ui.clone();
        move || end_tracking(&weak_ui)
    });
    tray.on_toggle_tracking_pause(move || {
        invoke(&weak_ui, |actions| actions.invoke_toggle_tracking_pause())
    });
    tray.on_quit(|| {
        let _ = slint::quit_event_loop();
    });

    TRAY.with(|stored| *stored.borrow_mut() = Some(tray));
    refresh(ui);
}

/// Mirrors the already-projected state rather than querying or mutating Tracker.
pub(crate) fn refresh(ui: &AppWindow) {
    TRAY.with(|stored| {
        let stored = stored.borrow();
        let Some(tray) = stored.as_ref() else {
            return;
        };
        tray.set_tracking(ui.get_tracking());
        tray.set_projects(ui.get_home_state().projects);
    });
}

fn invoke(action_ui: &slint::Weak<AppWindow>, invoke: impl FnOnce(&AppActions)) {
    let Some(ui) = action_ui.upgrade() else {
        return;
    };
    invoke(&ui.global::<AppActions>());
}

fn open_tempo(action_ui: &slint::Weak<AppWindow>) {
    let Some(ui) = action_ui.upgrade() else {
        return;
    };
    ui.global::<AppActions>().invoke_navigate(Page::Tracking);
    let _ = ui.show();
}

/// A tray action can originate while Tempo's window is hidden. The controller
/// selects the completion-note page; the tray then makes that required input
/// visible to the user.
fn end_tracking(action_ui: &slint::Weak<AppWindow>) {
    let Some(ui) = action_ui.upgrade() else {
        return;
    };
    ui.global::<AppActions>().invoke_end_tracking();
    let _ = ui.show();
}
