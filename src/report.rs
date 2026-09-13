//! Read-only export projections over completed tasks.
//!
//! This module owns the shared export dataset and its Markdown, CSV, and JSON
//! renderers. It does not write files or mutate tracker state.

use crate::{
    domain::{self, Data, ProjectId, Range},
    language::Language,
};
use chrono::{Local, TimeZone};
use serde::Serialize;

/// The finite set of file formats offered by the export workflow.
#[derive(Clone, Copy)]
pub(crate) enum Format {
    Csv,
    Markdown,
    Json,
}

/// Renders tasks in `range`, optionally narrowed to one stable project ID.
pub(crate) fn render(
    data: &Data,
    range: Range,
    project_id: Option<&ProjectId>,
    timestamp: i64,
    language: Language,
    format: Format,
) -> Result<String, String> {
    let document = ExportDocument::new(data, range, project_id, timestamp, language);
    match format {
        Format::Csv => document.csv(),
        Format::Markdown => Ok(document.markdown()),
        Format::Json => serde_json::to_string_pretty(&document).map_err(|error| error.to_string()),
    }
}

#[derive(Serialize)]
struct ExportDocument {
    generated_at: String,
    range: &'static str,
    scope: ExportScope,
    total_seconds: i64,
    tasks: Vec<ExportTask>,
    #[serde(skip)]
    copy: ReportCopy,
}

impl ExportDocument {
    fn new(
        data: &Data,
        range: Range,
        project_id: Option<&ProjectId>,
        timestamp: i64,
        language: Language,
    ) -> Self {
        let copy = ReportCopy::for_language(language);
        let tasks = data
            .tasks()
            .iter()
            .filter(|task| domain::in_range(task, range, timestamp))
            .filter(|task| project_id.is_none_or(|id| task.project_id() == id))
            .map(|task| {
                let duration_seconds = task.ended() - task.started();
                ExportTask {
                    task_id: task.id().as_str().to_owned(),
                    project_id: task.project_id().as_str().to_owned(),
                    project: data.project_name(task.project_id()),
                    started_at: local_timestamp(task.started()),
                    ended_at: local_timestamp(task.ended()),
                    duration_seconds,
                    duration: copy.duration(duration_seconds),
                    note: task.note().to_owned(),
                }
            })
            .collect::<Vec<_>>();
        let total_seconds = tasks.iter().map(|task| task.duration_seconds).sum();
        let scope = match project_id {
            Some(id) => ExportScope::Project {
                project_id: id.as_str().to_owned(),
                project: data.project_name(id),
            },
            None => ExportScope::AllProjects,
        };

        Self {
            generated_at: local_timestamp(timestamp),
            range: range_key(range),
            scope,
            total_seconds,
            tasks,
            copy,
        }
    }

    fn csv(&self) -> Result<String, String> {
        let mut writer = csv::Writer::from_writer(Vec::new());
        for task in &self.tasks {
            writer.serialize(task).map_err(|error| error.to_string())?;
        }
        let bytes = writer
            .into_inner()
            .map_err(|error| error.into_error().to_string())?;
        String::from_utf8(bytes).map_err(|error| error.to_string())
    }

    fn markdown(&self) -> String {
        let title = match &self.scope {
            ExportScope::AllProjects => self.copy.range_name(self.range).to_owned(),
            ExportScope::Project { project, .. } => {
                format!("{project} — {}", self.copy.range_name(self.range))
            }
        };
        let mut report = format!(
            "# Tempo — {title}\n\n**{}:** {} ({} s)\n",
            self.copy.total_label,
            self.copy.duration(self.total_seconds),
            self.total_seconds
        );
        if self.tasks.is_empty() {
            report.push_str(&format!("\n{}\n", self.copy.no_completed_tasks));
        }
        for task in &self.tasks {
            report.push_str(&format!(
                "\n- **{}** — {} ({} s) ({})",
                task.project, task.duration, task.duration_seconds, task.ended_at
            ));
            if !task.note.trim().is_empty() {
                report.push_str(&format!(": {}", task.note.trim()));
            }
            report.push('\n');
        }
        report
    }
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ExportScope {
    AllProjects,
    Project { project_id: String, project: String },
}

#[derive(Serialize)]
struct ExportTask {
    task_id: String,
    project_id: String,
    project: String,
    started_at: String,
    ended_at: String,
    duration_seconds: i64,
    duration: String,
    note: String,
}

fn local_timestamp(timestamp: i64) -> String {
    Local
        .timestamp_opt(timestamp, 0)
        .single()
        .unwrap_or_else(Local::now)
        .to_rfc3339()
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

    fn range_name(&self, range: &str) -> &'static str {
        self.ranges[match range {
            "day" => 0,
            "week" => 1,
            "month" => 2,
            "year" => 3,
            _ => unreachable!("range keys are defined by Range::key"),
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

fn range_key(range: Range) -> &'static str {
    match range {
        Range::Day => "day",
        Range::Week => "week",
        Range::Month => "month",
        Range::Year => "year",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::Tracker;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEST_ID: AtomicU64 = AtomicU64::new(0);

    fn tracker_with_tasks() -> (Tracker, std::path::PathBuf) {
        let path = std::env::temp_dir().join(format!(
            "tempo-report-test-{}-{}.sqlite",
            std::process::id(),
            NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let mut tracker = Tracker::at(path.clone());
        tracker.start_tracking("project-1".into(), 0).unwrap();
        tracker.end_tracking(2_520).unwrap();
        tracker
            .save_task_note("Brief, \"review\"\nnext".into())
            .unwrap();
        tracker.start_tracking("project-2".into(), 3_000).unwrap();
        tracker.end_tracking(3_060).unwrap();
        (tracker, path)
    }

    #[test]
    fn exports_are_scoped_and_preserve_machine_readable_fields() {
        let (tracker, path) = tracker_with_tasks();
        let project = ProjectId::from("project-1".to_owned());
        let json = render(
            tracker.data(),
            Range::Year,
            Some(&project),
            3_060,
            Language::English,
            Format::Json,
        )
        .unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["scope"]["kind"], "project");
        assert_eq!(value["tasks"].as_array().unwrap().len(), 1);
        assert_eq!(value["tasks"][0]["duration_seconds"], 2_520);
        assert!(value["tasks"][0]["ended_at"]
            .as_str()
            .unwrap()
            .contains('T'));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn csv_quotes_notes_and_markdown_is_localized() {
        let (tracker, path) = tracker_with_tasks();
        let csv = render(
            tracker.data(),
            Range::Year,
            None,
            3_060,
            Language::English,
            Format::Csv,
        )
        .unwrap();
        let mut reader = csv::Reader::from_reader(csv.as_bytes());
        let mut records = reader.records();
        let first = records.next().unwrap().unwrap();
        assert_eq!(first.get(7), Some("Brief, \"review\"\nnext"));

        let markdown = render(
            tracker.data(),
            Range::Year,
            None,
            3_060,
            Language::German,
            Format::Markdown,
        )
        .unwrap();
        assert!(markdown.contains("**Gesamt:**"));
        assert!(markdown.contains("DIESES JAHR"));
        std::fs::remove_file(path).unwrap();
    }
}
