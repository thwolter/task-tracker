#[cfg(test)]
mod tests {
    use crate::ui::bindings::bind;
    use crate::{AppWindow, Page, application::Tracker};
    use std::{cell::RefCell, rc::Rc};

    #[test]
    fn settings_home_callback_returns_to_home() {
        i_slint_backend_testing::init_no_event_loop();
        let ui = AppWindow::new().unwrap();
        ui.set_page(Page::Settings);
        ui.invoke_open_home_view();
        assert_eq!(ui.get_page(), Page::Home);
    }

    #[test]
    fn project_editor_is_an_exclusive_page_and_returns_to_settings() {
        i_slint_backend_testing::init_no_event_loop();
        let path = std::env::temp_dir().join(format!(
            "tempo-ui-test-{}-{}.json",
            std::process::id(),
            crate::domain::now()
        ));
        let ui = AppWindow::new().unwrap();
        let tracker = Rc::new(RefCell::new(Tracker::at(path.clone())));
        let _timer = bind(&ui, tracker);

        ui.invoke_open_add_project();
        assert_eq!(ui.get_page(), Page::ProjectEditor);

        ui.invoke_close_project_dialog();
        assert_eq!(ui.get_page(), Page::Settings);

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn unarchive_project_callback_restores_project_to_home() {
        i_slint_backend_testing::init_no_event_loop();
        let path = std::env::temp_dir().join(format!(
            "tempo-ui-unarchive-test-{}-{}.json",
            std::process::id(),
            crate::domain::now()
        ));
        let ui = AppWindow::new().unwrap();
        let tracker = Rc::new(RefCell::new(Tracker::at(path.clone())));
        let _timer = bind(&ui, tracker.clone());

        ui.invoke_open_rename_project("project-1".into());
        ui.invoke_archive_project("project-1".into());
        assert!(tracker.borrow().data().projects()[0].archived());

        ui.invoke_open_rename_project("project-1".into());
        assert!(ui.get_project_dialog().archived);

        ui.invoke_unarchive_project("project-1".into());

        assert_eq!(ui.get_page(), Page::Settings);
        assert!(!tracker.borrow().data().projects()[0].archived());

        std::fs::remove_file(path).unwrap();
    }
}
