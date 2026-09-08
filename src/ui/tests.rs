#[cfg(test)]
mod tests {
    use crate::ui::bindings::bind;
    use crate::{AppWindow, Page, application::Tracker, language::Language};
    use std::{cell::RefCell, rc::Rc};

    #[test]
    fn settings_home_callback_returns_to_home() {
        i_slint_backend_testing::init_no_event_loop();
        let ui = AppWindow::new().unwrap();
        assert!(slint::select_bundled_translation("de").is_ok());
        ui.set_page(Page::Settings);
        ui.invoke_open_home_view();
        assert_eq!(ui.get_page(), Page::Home);
    }

    #[test]
    fn project_editor_is_an_exclusive_page_and_returns_to_settings() {
        i_slint_backend_testing::init_no_event_loop();
        let path = std::env::temp_dir().join(format!(
            "tempo-ui-test-{}-{}.sqlite",
            std::process::id(),
            crate::domain::now()
        ));
        let ui = AppWindow::new().unwrap();
        let tracker = Rc::new(RefCell::new(Tracker::at(path.clone())));
        let _timer = bind(&ui, tracker, Language::English);

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
            "tempo-ui-unarchive-test-{}-{}.sqlite",
            std::process::id(),
            crate::domain::now()
        ));
        let ui = AppWindow::new().unwrap();
        let tracker = Rc::new(RefCell::new(Tracker::at(path.clone())));
        let _timer = bind(&ui, tracker.clone(), Language::English);

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

    #[test]
    fn evaluation_task_callbacks_refresh_after_releasing_tracker_mutation_borrow() {
        i_slint_backend_testing::init_no_event_loop();
        let path = std::env::temp_dir().join(format!(
            "tempo-ui-evaluation-task-test-{}-{}.sqlite",
            std::process::id(),
            crate::domain::now()
        ));
        let ui = AppWindow::new().unwrap();
        let tracker = Rc::new(RefCell::new(Tracker::at(path.clone())));
        let _timer = bind(&ui, tracker.clone(), Language::English);
        let task_id = {
            let mut tracker = tracker.borrow_mut();
            tracker.start_tracking("project-1".into(), 1).unwrap();
            tracker.end_tracking(2).unwrap();
            tracker.data().tasks()[0].id().as_str().to_owned()
        };

        ui.invoke_open_drilldown("project-1".into());
        ui.invoke_update_evaluation_task(task_id.clone().into(), "Updated note".into());
        assert_eq!(tracker.borrow().data().tasks()[0].note(), "Updated note");

        ui.invoke_delete_evaluation_task(task_id.into());
        assert!(tracker.borrow().data().tasks().is_empty());

        std::fs::remove_file(path).unwrap();
    }
}
