use crate::{
    EvaluationState, HomeState, ProjectItem, Range, TaskItem, TrackingState,
    application::Tracker,
    domain::{self, Range as DomainRange},
};
use chrono::{Local, TimeZone};
use slint::{ModelRc, SharedString, VecModel};

pub(crate) fn home(tracker: &Tracker) -> HomeState {
    let projects = ModelRc::new(VecModel::from(
        tracker
            .data()
            .projects()
            .iter()
            .filter(|project| !project.archived())
            .map(|project| ProjectItem {
                id: project.id().as_str().into(),
                name: project.name().into(),
                completed: false,
                archived: false,
            })
            .collect::<Vec<_>>(),
    ));
    let last_task = tracker
        .data()
        .tasks()
        .last()
        .map(|task| {
            format!(
                "{}  ·  {}",
                tracker.data().project_name(task.project_id()),
                domain::duration(task.ended() - task.started())
            )
        })
        .unwrap_or_else(|| "No completed sessions yet".into())
        .into();
    HomeState {
        projects,
        last_task,
    }
}

pub(crate) fn project_settings(tracker: &Tracker) -> ModelRc<ProjectItem> {
    ModelRc::new(VecModel::from(
        tracker
            .data()
            .projects()
            .iter()
            .map(|project| ProjectItem {
                id: project.id().as_str().into(),
                name: project.name().into(),
                completed: false,
                archived: project.archived(),
            })
            .collect::<Vec<_>>(),
    ))
}

pub(crate) fn evaluation(tracker: &Tracker, timestamp: i64) -> EvaluationState {
    let matching: Vec<_> = tracker
        .data()
        .tasks()
        .iter()
        .filter(|task| domain::in_range(task, tracker.range(), timestamp))
        .collect();
    let total: i64 = matching
        .iter()
        .map(|task| task.ended() - task.started())
        .sum();
    let tasks = ModelRc::new(VecModel::from(
        matching
            .iter()
            .rev()
            .take(3)
            .map(|task| TaskItem {
                project: tracker.data().project_name(task.project_id()).into(),
                note: task.note().into(),
                duration: domain::duration(task.ended() - task.started()).into(),
                timestamp: Local
                    .timestamp_opt(task.ended(), 0)
                    .single()
                    .unwrap_or_else(Local::now)
                    .format("%b %-d, %H:%M")
                    .to_string()
                    .into(),
            })
            .collect::<Vec<_>>(),
    ));
    EvaluationState {
        tasks,
        total: format!("{}  {}", tracker.range().name(), domain::duration(total)).into(),
        range: slint_range(tracker.range()),
    }
}

pub(crate) fn tracking(tracker: &Tracker, timestamp: i64) -> TrackingState {
    let (active_task, elapsed, paused) = tracker.data().active_task().map_or_else(
        || (SharedString::new(), SharedString::from("00:00"), false),
        |active| {
            (
                tracker
                    .data()
                    .project_name(active.project_id())
                    .to_uppercase()
                    .into(),
                format_elapsed(active.elapsed_until(timestamp)).into(),
                active.paused(),
            )
        },
    );
    TrackingState {
        active_task,
        elapsed,
        paused,
    }
}

pub(crate) fn refresh(ui: &crate::AppWindow, tracker: &Tracker, timestamp: i64) {
    ui.set_home(home(tracker));
    ui.set_project_settings(project_settings(tracker));
    ui.set_evaluation(evaluation(tracker, timestamp));
    ui.set_tracking(tracking(tracker, timestamp));
}

pub(crate) fn domain_range(range: Range) -> DomainRange {
    match range {
        Range::Day => DomainRange::Day,
        Range::Week => DomainRange::Week,
        Range::Month => DomainRange::Month,
        Range::Year => DomainRange::Year,
    }
}

fn slint_range(range: DomainRange) -> Range {
    match range {
        DomainRange::Day => Range::Day,
        DomainRange::Week => Range::Week,
        DomainRange::Month => Range::Month,
        DomainRange::Year => Range::Year,
    }
}

fn format_elapsed(seconds: i64) -> String {
    format!("{:02}:{:02}", seconds.max(0) / 60, seconds.max(0) % 60)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{application::Tracker, domain};
    use slint::Model;

    #[test]
    fn projections_cover_empty_active_and_populated_states() {
        let path = std::env::temp_dir().join(format!(
            "tempo-presentation-test-{}-{}.json",
            std::process::id(),
            domain::now()
        ));
        let mut tracker = Tracker::at(path.clone());
        assert_eq!(home(&tracker).last_task, "No completed sessions yet");
        assert_eq!(tracking(&tracker, 0).elapsed, "00:00");

        tracker.start_tracking("project-1".into(), 10).unwrap();
        assert_eq!(tracking(&tracker, 70).elapsed, "01:00");
        tracker.toggle_tracking_pause(70).unwrap();
        assert!(tracking(&tracker, 90).paused);
        tracker.end_tracking(130).unwrap();
        assert_eq!(evaluation(&tracker, 130).tasks.row_count(), 1);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn archived_projects_are_hidden_only_from_home() {
        let path = std::env::temp_dir().join(format!(
            "tempo-presentation-archive-test-{}-{}.json",
            std::process::id(),
            domain::now()
        ));
        let mut tracker = Tracker::at(path.clone());

        tracker.archive_project("project-1".into()).unwrap();

        assert_eq!(home(&tracker).projects.row_count(), 3);
        let settings = project_settings(&tracker);
        assert_eq!(settings.row_count(), 4);
        assert!(settings.row_data(0).unwrap().archived);

        std::fs::remove_file(path).unwrap();
    }
}
