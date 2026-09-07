//! Projections from application state into Slint view models.
//!
//! This module owns UI-ready formatting, range grouping, and filtering. It
//! reads [`Tracker`] state but does not mutate domain state or persist data.

use crate::{
    EvaluationState, EvaluationTaskItem, EvaluationTasksState, HomeState, ProjectItem,
    ProjectTotalItem, Range, TaskItem, TrackingState,
    application::Tracker,
    domain::{self, Data, ProjectId, Range as DomainRange, Task},
    language::Language,
};
use chrono::{Datelike, Local, TimeZone, Weekday};
use slint::{ModelRc, SharedString, VecModel};

/// Builds the Home-page snapshot, omitting archived projects.
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
        .map(|task| task_item(tracker.data(), task))
        .unwrap_or_else(|| TaskItem {
            project: SharedString::new(),
            note: SharedString::new(),
            duration_seconds: 0,
            timestamp: SharedString::new(),
        });
    HomeState {
        projects,
        last_task,
        has_last_task: tracker.data().has_tasks(),
    }
}

fn task_item(data: &Data, task: &Task) -> TaskItem {
    TaskItem {
        project: data.project_name(task.project_id()).into(),
        note: task.note().into(),
        duration_seconds: slint_seconds(task.ended() - task.started()),
        timestamp: Local
            .timestamp_opt(task.ended(), 0)
            .single()
            .unwrap_or_else(Local::now)
            .format("%b %-d, %H:%M")
            .to_string()
            .into(),
    }
}

/// Builds the Settings project list, including archived projects and their state.
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

/// Builds the selected calendar range's totals and up to three newest tasks.
pub(crate) fn evaluation(tracker: &Tracker, timestamp: i64, language: Language) -> EvaluationState {
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
    let project_totals = project_totals(tracker.data(), &matching);
    let tasks = ModelRc::new(VecModel::from(
        matching
            .iter()
            .rev()
            .take(3)
            .map(|task| EvaluationTaskItem {
                id: task.id().as_str().into(),
                project: tracker.data().project_name(task.project_id()).into(),
                note: task.note().into(),
                duration_seconds: slint_seconds(task.ended() - task.started()),
                completed_label: localized_completion_date(task, language).into(),
            })
            .collect::<Vec<_>>(),
    ));
    EvaluationState {
        tasks,
        total_seconds: slint_seconds(total),
        range: slint_range(tracker.range()),
        project_totals,
    }
}

/// Builds the selected project's complete task list for the current evaluation range.
pub(crate) fn evaluation_tasks(
    tracker: &Tracker,
    timestamp: i64,
    project_id: &ProjectId,
    language: Language,
) -> EvaluationTasksState {
    let matching: Vec<_> = tracker
        .data()
        .tasks()
        .iter()
        .filter(|task| domain::in_range(task, tracker.range(), timestamp))
        .filter(|task| task.project_id() == project_id)
        .collect();
    let total: i64 = matching
        .iter()
        .map(|task| task.ended() - task.started())
        .sum();

    EvaluationTasksState {
        project: tracker.data().project_name(project_id).into(),
        tasks: ModelRc::new(VecModel::from(
            matching
                .iter()
                .rev()
                .map(|task| EvaluationTaskItem {
                    id: task.id().as_str().into(),
                    project: tracker.data().project_name(task.project_id()).into(),
                    note: task.note().into(),
                    duration_seconds: slint_seconds(task.ended() - task.started()),
                    completed_label: localized_completion_date(task, language).into(),
                })
                .collect::<Vec<_>>(),
        )),
        total_seconds: slint_seconds(total),
        range: slint_range(tracker.range()),
    }
}

fn project_totals(data: &Data, tasks: &[&Task]) -> ModelRc<ProjectTotalItem> {
    let mut totals = Vec::new();
    for task in tasks {
        let duration = task.ended() - task.started();
        if let Some((_, total, num_tasks)) = totals
            .iter_mut()
            .find(|(project_id, _, _)| *project_id == task.project_id())
        {
            *total += duration;
            *num_tasks += 1;
        } else {
            totals.push((task.project_id(), duration, 1));
        }
    }

    ModelRc::new(VecModel::from(
        totals
            .into_iter()
            .map(|(project_id, total, num_tasks)| ProjectTotalItem {
                id: project_id.as_str().into(),
                project: data.project_name(project_id).into(),
                duration_seconds: slint_seconds(total),
                num_tasks,
            })
            .collect::<Vec<_>>(),
    ))
}

fn slint_seconds(seconds: i64) -> i32 {
    seconds.clamp(0, i64::from(i32::MAX)) as i32
}

fn localized_completion_date(task: &Task, language: Language) -> String {
    let completed_at = Local
        .timestamp_opt(task.ended(), 0)
        .single()
        .unwrap_or_else(Local::now);
    let weekday = localized_weekday(completed_at.weekday(), language);
    let month = localized_month(completed_at.month(), language);
    match language {
        Language::English => format!("{weekday}, {} {month}", completed_at.day()),
        Language::German => format!("{weekday}, {}. {month}", completed_at.day()),
    }
    .to_uppercase()
}

fn localized_weekday(weekday: Weekday, language: Language) -> &'static str {
    match (language, weekday) {
        (Language::English, Weekday::Mon) => "Mon",
        (Language::English, Weekday::Tue) => "Tue",
        (Language::English, Weekday::Wed) => "Wed",
        (Language::English, Weekday::Thu) => "Thu",
        (Language::English, Weekday::Fri) => "Fri",
        (Language::English, Weekday::Sat) => "Sat",
        (Language::English, Weekday::Sun) => "Sun",
        (Language::German, Weekday::Mon) => "Mo.",
        (Language::German, Weekday::Tue) => "Di.",
        (Language::German, Weekday::Wed) => "Mi.",
        (Language::German, Weekday::Thu) => "Do.",
        (Language::German, Weekday::Fri) => "Fr.",
        (Language::German, Weekday::Sat) => "Sa.",
        (Language::German, Weekday::Sun) => "So.",
    }
}

fn localized_month(month: u32, language: Language) -> &'static str {
    match (language, month) {
        (Language::English, 1) => "Jan",
        (Language::English, 2) => "Feb",
        (Language::English, 3) => "Mar",
        (Language::English, 4) => "Apr",
        (Language::English, 5) => "May",
        (Language::English, 6) => "Jun",
        (Language::English, 7) => "Jul",
        (Language::English, 8) => "Aug",
        (Language::English, 9) => "Sep",
        (Language::English, 10) => "Oct",
        (Language::English, 11) => "Nov",
        (Language::English, 12) => "Dec",
        (Language::German, 1) => "Jan.",
        (Language::German, 2) => "Feb.",
        (Language::German, 3) => "März",
        (Language::German, 4) => "Apr.",
        (Language::German, 5) => "Mai",
        (Language::German, 6) => "Juni",
        (Language::German, 7) => "Juli",
        (Language::German, 8) => "Aug.",
        (Language::German, 9) => "Sept.",
        (Language::German, 10) => "Okt.",
        (Language::German, 11) => "Nov.",
        (Language::German, 12) => "Dez.",
        (_, _) => unreachable!("chrono months are always in 1..=12"),
    }
}

/// Builds the active-tracking display, or the empty state when no task is active.
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

/// Replaces every Slint state projection with a consistent tracker snapshot.
pub(crate) fn refresh(
    ui: &crate::AppWindow,
    tracker: &Tracker,
    timestamp: i64,
    language: Language,
) {
    ui.set_home(home(tracker));
    ui.set_project_settings(project_settings(tracker));
    ui.set_evaluation(evaluation(tracker, timestamp, language));
    if let Some(project) = tracker.evaluating_project() {
        ui.set_evaluation_tasks(evaluation_tasks(tracker, timestamp, project, language));
    }
    ui.set_tracking(tracking(tracker, timestamp));
}

/// Converts the UI-generated range enum into the domain enum.
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
    use crate::{application::Tracker, domain, language::Language};
    use slint::Model;

    #[test]
    fn projections_cover_empty_active_and_populated_states() {
        let path = std::env::temp_dir().join(format!(
            "tempo-presentation-test-{}-{}.sqlite",
            std::process::id(),
            domain::now()
        ));
        let mut tracker = Tracker::at(path.clone());
        assert!(!home(&tracker).has_last_task);
        assert_eq!(tracking(&tracker, 0).elapsed, "00:00");

        tracker.start_tracking("project-1".into(), 10).unwrap();
        assert_eq!(tracking(&tracker, 70).elapsed, "01:00");
        tracker.toggle_tracking_pause(70).unwrap();
        assert!(tracking(&tracker, 90).paused);
        tracker.end_tracking(130).unwrap();
        assert!(home(&tracker).has_last_task);
        assert_eq!(home(&tracker).last_task.duration_seconds, 60);
        let evaluation = evaluation(&tracker, 130, Language::English);
        assert_eq!(evaluation.tasks.row_count(), 1);
        assert_eq!(evaluation.tasks.row_data(0).unwrap().duration_seconds, 60);
        assert_eq!(evaluation.total_seconds, 60);
        assert_eq!(evaluation.project_totals.row_count(), 1);
        let total = evaluation.project_totals.row_data(0).unwrap();
        assert_eq!(total.project, "Project Atlas");
        assert_eq!(total.duration_seconds, 60);
        assert_eq!(total.num_tasks, 1);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn project_evaluation_contains_only_that_projects_tasks() {
        let path = std::env::temp_dir().join(format!(
            "tempo-project-evaluation-test-{}-{}.sqlite",
            std::process::id(),
            domain::now()
        ));
        let mut tracker = Tracker::at(path.clone());

        tracker.start_tracking("project-1".into(), 10).unwrap();
        tracker.end_tracking(70).unwrap();
        tracker.save_task_note("Planning".into()).unwrap();
        tracker.start_tracking("project-2".into(), 80).unwrap();
        tracker.end_tracking(110).unwrap();

        let project_id = ProjectId::from("project-1".to_owned());
        let evaluation = evaluation_tasks(&tracker, 130, &project_id, Language::English);
        assert_eq!(evaluation.project, "Project Atlas");
        assert_eq!(evaluation.tasks.row_count(), 1);
        assert_eq!(evaluation.tasks.row_data(0).unwrap().note, "Planning");
        assert_eq!(evaluation.tasks.row_data(0).unwrap().id, "task-10-1");
        assert_eq!(
            evaluation.tasks.row_data(0).unwrap().completed_label,
            "THU, 1 JAN"
        );
        assert_eq!(
            evaluation_tasks(&tracker, 130, &project_id, Language::German)
                .tasks
                .row_data(0)
                .unwrap()
                .completed_label,
            "DO., 1. JAN."
        );
        assert_eq!(evaluation.total_seconds, 60);

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn archived_projects_are_hidden_only_from_home() {
        let path = std::env::temp_dir().join(format!(
            "tempo-presentation-archive-test-{}-{}.sqlite",
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
