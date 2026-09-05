use crate::{
    EvaluationState, HomeState, Range, SessionItem, TaskItem, TrackingState,
    application::Tracker,
    domain::{self, Range as DomainRange},
};
use chrono::{Local, TimeZone};
use slint::{ModelRc, SharedString, VecModel};

pub(crate) fn home(tracker: &Tracker) -> HomeState {
    let tasks = ModelRc::new(VecModel::from(
        tracker
            .data()
            .tasks()
            .iter()
            .map(|task| TaskItem {
                id: task.id().into(),
                title: task.title().into(),
                completed: false,
            })
            .collect::<Vec<_>>(),
    ));
    let last_session = tracker
        .data()
        .sessions()
        .last()
        .map(|session| {
            format!(
                "{}  ·  {}",
                tracker.data().task_name(session.task_id()),
                domain::duration(session.ended() - session.started())
            )
        })
        .unwrap_or_else(|| "No completed sessions yet".into())
        .into();
    HomeState {
        tasks,
        last_session,
    }
}

pub(crate) fn evaluation(tracker: &Tracker, timestamp: i64) -> EvaluationState {
    let matching: Vec<_> = tracker
        .data()
        .sessions()
        .iter()
        .filter(|session| domain::in_range(session, tracker.range(), timestamp))
        .collect();
    let total: i64 = matching
        .iter()
        .map(|session| session.ended() - session.started())
        .sum();
    let sessions = ModelRc::new(VecModel::from(
        matching
            .iter()
            .rev()
            .take(3)
            .map(|session| SessionItem {
                task: tracker.data().task_name(session.task_id()).into(),
                note: session.note().into(),
                duration: domain::duration(session.ended() - session.started()).into(),
                timestamp: Local
                    .timestamp_opt(session.ended(), 0)
                    .single()
                    .unwrap_or_else(Local::now)
                    .format("%b %-d, %H:%M")
                    .to_string()
                    .into(),
            })
            .collect::<Vec<_>>(),
    ));
    EvaluationState {
        sessions,
        total: format!("{}  {}", tracker.range().name(), domain::duration(total)).into(),
        range: slint_range(tracker.range()),
    }
}

pub(crate) fn tracking(tracker: &Tracker, timestamp: i64) -> TrackingState {
    let (active_task, elapsed) = tracker.data().active().map_or_else(
        || (SharedString::new(), SharedString::from("00:00")),
        |active| {
            (
                tracker
                    .data()
                    .task_name(active.task_id())
                    .to_uppercase()
                    .into(),
                format_elapsed(timestamp - active.started()).into(),
            )
        },
    );
    TrackingState {
        active_task,
        elapsed,
    }
}

pub(crate) fn refresh(ui: &crate::AppWindow, tracker: &Tracker, timestamp: i64) {
    ui.set_home(home(tracker));
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
        assert_eq!(home(&tracker).last_session, "No completed sessions yet");
        assert_eq!(tracking(&tracker, 0).elapsed, "00:00");

        tracker.start_task("task-1".into(), 10).unwrap();
        assert_eq!(tracking(&tracker, 70).elapsed, "01:00");
        tracker.end_task(130).unwrap();
        assert_eq!(evaluation(&tracker, 130).sessions.row_count(), 1);
        std::fs::remove_file(path).unwrap();
    }
}
