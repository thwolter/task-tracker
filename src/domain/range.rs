//! Calendar ranges used to filter completed tasks for presentation and reports.
//!
//! Membership is determined from each task's end timestamp in the local time
//! zone, relative to a supplied reference timestamp.

use super::Task;
use chrono::{Datelike, Local, TimeZone};

/// A calendar period relative to a reference timestamp in the local time zone.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Range {
    Day,
    Week,
    Month,
    Year,
}

/// Returns whether the task ended within `range` containing `timestamp`.
///
/// Invalid timestamps fall back to the current local time, matching report
/// date formatting.
pub(crate) fn in_range(task: &Task, range: Range, timestamp: i64) -> bool {
    let current = Local
        .timestamp_opt(timestamp, 0)
        .single()
        .unwrap_or_else(Local::now);
    let item = Local
        .timestamp_opt(task.ended(), 0)
        .single()
        .unwrap_or_else(Local::now);
    match range {
        Range::Day => current.date_naive() == item.date_naive(),
        Range::Week => current.iso_week() == item.iso_week(),
        Range::Month => current.year() == item.year() && current.month() == item.month(),
        Range::Year => current.year() == item.year(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ProjectId, TaskId};

    #[test]
    fn range_checks_are_readable() {
        let task = Task::from_storage(
            TaskId::new("task-1"),
            ProjectId::new("project-1"),
            None,
            0,
            2_520,
            "Brief".into(),
        );

        assert!(in_range(&task, Range::Year, 2_520));
    }
}
