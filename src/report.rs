//! Localized Markdown reporting over completed tasks.
//!
//! Reports are read-only projections of the selected calendar range. This
//! module owns report copy and duration formatting, not file export.

use crate::{
    domain::{self, Data, Range},
    language::Language,
};
use chrono::{Local, TimeZone};

/// Renders completed tasks in `range` as a localized Markdown report.
///
/// Durations are rounded to the nearest minute and task timestamps are shown
/// in the local time zone.
pub(crate) fn markdown(data: &Data, range: Range, timestamp: i64, language: Language) -> String {
    let tasks: Vec<_> = data
        .tasks()
        .iter()
        .filter(|task| domain::in_range(task, range, timestamp))
        .collect();
    let total: i64 = tasks.iter().map(|task| task.ended() - task.started()).sum();
    let copy = ReportCopy::for_language(language);
    let mut report = format!(
        "# Tempo — {}\n\n**{}:** {}\n",
        copy.range_name(range),
        copy.total_label,
        copy.duration(total)
    );
    if tasks.is_empty() {
        report.push_str(&format!("\n{}\n", copy.no_completed_tasks));
    }
    for task in tasks {
        let date = Local
            .timestamp_opt(task.ended(), 0)
            .single()
            .unwrap_or_else(Local::now)
            .format("%Y-%m-%d %H:%M");
        report.push_str(&format!(
            "\n- **{}** — {} ({})",
            data.project_name(task.project_id()),
            copy.duration(task.ended() - task.started()),
            date
        ));
        if !task.note().trim().is_empty() {
            report.push_str(&format!(": {}", task.note().trim()));
        }
        report.push('\n');
    }
    report
}

struct ReportCopy {
    total_label: &'static str,
    no_completed_tasks: &'static str,
    minute: &'static str,
    hour: &'static str,
    ranges: [&'static str; 4],
}

impl ReportCopy {
    fn for_language(language: Language) -> Self {
        match language {
            Language::English => Self {
                total_label: "Total",
                no_completed_tasks: "No completed tasks.",
                minute: "min",
                hour: "h",
                ranges: ["TODAY", "THIS WEEK", "THIS MONTH", "THIS YEAR"],
            },
            Language::German => Self {
                total_label: "Gesamt",
                no_completed_tasks: "Keine abgeschlossenen Aufgaben.",
                minute: "Min.",
                hour: "Std.",
                ranges: ["HEUTE", "DIESE WOCHE", "DIESER MONAT", "DIESES JAHR"],
            },
        }
    }

    fn range_name(&self, range: Range) -> &'static str {
        self.ranges[match range {
            Range::Day => 0,
            Range::Week => 1,
            Range::Month => 2,
            Range::Year => 3,
        }]
    }

    fn duration(&self, seconds: i64) -> String {
        let minutes = (seconds.max(0) + 30) / 60;
        if minutes >= 60 {
            format!(
                "{} {} {} {}",
                minutes / 60,
                self.hour,
                minutes % 60,
                self.minute
            )
        } else {
            format!("{minutes} {}", self.minute)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::Tracker;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEST_ID: AtomicU64 = AtomicU64::new(0);

    fn tracker_with_task() -> (Tracker, std::path::PathBuf) {
        let path = std::env::temp_dir().join(format!(
            "tempo-report-test-{}-{}.sqlite",
            std::process::id(),
            NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let mut tracker = Tracker::at(path.clone());
        tracker.start_tracking("project-1".into(), 0).unwrap();
        tracker.end_tracking(2_520).unwrap();
        tracker.save_task_note("Brief".into()).unwrap();
        (tracker, path)
    }

    #[test]
    fn report_is_localized_and_preserves_duration_rounding() {
        let (tracker, path) = tracker_with_task();
        let english = markdown(tracker.data(), Range::Year, 2520, Language::English);
        assert!(english.contains("**Total:** 42 min"));
        assert!(english.contains("Project Atlas"));

        let german = markdown(tracker.data(), Range::Year, 2520, Language::German);
        assert!(german.contains("**Gesamt:** 42 Min."));
        assert!(german.contains("DIESES JAHR"));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn report_handles_empty_ranges_and_hours() {
        let (tracker, path) = tracker_with_task();
        let empty = markdown(tracker.data(), Range::Day, 86_400, Language::German);
        assert!(empty.contains("Keine abgeschlossenen Aufgaben."));

        let copy = ReportCopy::for_language(Language::English);
        assert_eq!(copy.duration(3_570), "1 h 0 min");
        std::fs::remove_file(path).unwrap();
    }
}
