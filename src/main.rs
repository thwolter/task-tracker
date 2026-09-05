use chrono::{Datelike, Local, TimeZone};
use rfd::FileDialog;
use serde::{Deserialize, Serialize};
use slint::{ModelRc, SharedString, Timer, TimerMode, VecModel};
use std::{
    cell::RefCell,
    fs,
    path::PathBuf,
    rc::Rc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

slint::include_modules!();

#[derive(Clone, Serialize, Deserialize)]
struct Task {
    id: String,
    title: String,
}
#[derive(Clone, Serialize, Deserialize)]
struct Session {
    task_id: String,
    started: i64,
    ended: i64,
    note: String,
}
#[derive(Clone, Serialize, Deserialize)]
struct ActiveSession {
    task_id: String,
    started: i64,
    checkpoint: i64,
}
#[derive(Serialize, Deserialize)]
struct Data {
    tasks: Vec<Task>,
    sessions: Vec<Session>,
    active: Option<ActiveSession>,
}
struct State {
    data: Data,
    path: PathBuf,
    range: Range,
    editing_id: Option<String>,
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
fn default_data() -> Data {
    Data {
        tasks: ["Project Atlas", "Admin", "Writing", "Personal"]
            .into_iter()
            .enumerate()
            .map(|(i, title)| Task {
                id: format!("task-{}", i + 1),
                title: title.into(),
            })
            .collect(),
        sessions: vec![],
        active: None,
    }
}
fn default_path() -> PathBuf {
    let base = if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME").map(|p| PathBuf::from(p).join("Library/Application Support"))
    } else {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".local/share")))
    }
    .unwrap_or_else(std::env::temp_dir);
    base.join("Tempo").join("tempo.json")
}
fn load(path: &PathBuf) -> Data {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_else(default_data)
}

fn recover_active(data: &mut Data) {
    if let Some(active) = data.active.take() {
        data.sessions.push(Session {
            task_id: active.task_id,
            started: active.started,
            ended: active.checkpoint,
            note: String::new(),
        });
    }
}
fn save(state: &State) -> Result<(), String> {
    if let Some(parent) = state.path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(
        &state.path,
        serde_json::to_string_pretty(&state.data).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}
fn duration(seconds: i64) -> String {
    let minutes = (seconds.max(0) + 30) / 60;
    if minutes >= 60 {
        format!("{} h {} min", minutes / 60, minutes % 60)
    } else {
        format!("{} min", minutes)
    }
}
fn elapsed(seconds: i64) -> SharedString {
    format!("{:02}:{:02}", seconds.max(0) / 60, seconds.max(0) % 60).into()
}
fn range_name(range: Range) -> &'static str {
    match range {
        Range::Day => "TODAY",
        Range::Week => "THIS WEEK",
        Range::Month => "THIS MONTH",
        Range::Year => "THIS YEAR",
    }
}
fn in_range(session: &Session, range: Range, timestamp: i64) -> bool {
    let current = Local
        .timestamp_opt(timestamp, 0)
        .single()
        .unwrap_or_else(Local::now);
    let item = Local
        .timestamp_opt(session.ended, 0)
        .single()
        .unwrap_or_else(Local::now);
    match range {
        Range::Day => current.date_naive() == item.date_naive(),
        Range::Week => current.iso_week() == item.iso_week(),
        Range::Month => current.year() == item.year() && current.month() == item.month(),
        Range::Year => current.year() == item.year(),
    }
}
fn task_name(data: &Data, id: &str) -> String {
    data.tasks
        .iter()
        .find(|t| t.id == id)
        .map(|t| t.title.clone())
        .unwrap_or_else(|| "Deleted task".into())
}
fn markdown(data: &Data, range: Range, timestamp: i64) -> String {
    let sessions: Vec<_> = data
        .sessions
        .iter()
        .filter(|s| in_range(s, range, timestamp))
        .collect();
    let total: i64 = sessions.iter().map(|s| s.ended - s.started).sum();
    let mut report = format!(
        "# Tempo — {}\n\n**Total:** {}\n",
        range_name(range),
        duration(total)
    );
    if sessions.is_empty() {
        report.push_str("\nNo completed sessions.\n");
    }
    for session in sessions {
        let date = Local
            .timestamp_opt(session.ended, 0)
            .single()
            .unwrap_or_else(Local::now)
            .format("%Y-%m-%d %H:%M");
        report.push_str(&format!(
            "\n- **{}** — {} ({})",
            task_name(data, &session.task_id),
            duration(session.ended - session.started),
            date
        ));
        if !session.note.trim().is_empty() {
            report.push_str(&format!(": {}", session.note.trim()));
        }
        report.push('\n');
    }
    report
}
fn set_status(ui: &AppWindow, message: impl Into<SharedString>) {
    ui.set_status(message.into());
}
fn set_task_dialog(
    ui: &AppWindow,
    open: bool,
    rename_mode: bool,
    initial_draft: impl Into<SharedString>,
) {
    ui.set_task_dialog(TaskDialogState {
        open,
        rename_mode,
        initial_draft: initial_draft.into(),
    });
}
fn persist(ui: &AppWindow, state: &State) {
    if let Err(error) = save(state) {
        set_status(ui, format!("Could not save data: {error}"));
    }
}
fn refresh(ui: &AppWindow, state: &State) {
    let tasks = ModelRc::new(VecModel::from(
        state
            .data
            .tasks
            .iter()
            .map(|task| TaskItem {
                id: task.id.clone().into(),
                title: task.title.clone().into(),
                completed: false,
            })
            .collect::<Vec<_>>(),
    ));
    let timestamp = now();
    let matching: Vec<_> = state
        .data
        .sessions
        .iter()
        .filter(|s| in_range(s, state.range, timestamp))
        .collect();
    let total: i64 = matching.iter().map(|s| s.ended - s.started).sum();
    let sessions = ModelRc::new(VecModel::from(
        matching
            .iter()
            .rev()
            .take(3)
            .map(|s| SessionItem {
                task: task_name(&state.data, &s.task_id).into(),
                note: s.note.clone().into(),
                duration: duration(s.ended - s.started).into(),
                timestamp: Local
                    .timestamp_opt(s.ended, 0)
                    .single()
                    .unwrap_or_else(Local::now)
                    .format("%b %-d, %H:%M")
                    .to_string()
                    .into(),
            })
            .collect::<Vec<_>>(),
    ));
    let last_session = state
            .data
            .sessions
            .last()
            .map(|s| {
                format!(
                    "{}  ·  {}",
                    task_name(&state.data, &s.task_id),
                    duration(s.ended - s.started)
                )
            })
            .unwrap_or_else(|| "No completed sessions yet".into())
            .into();

    ui.set_home(HomeState { tasks, last_session });
    ui.set_evaluation(EvaluationState {
        sessions,
        total: format!("{}  {}", range_name(state.range), duration(total)).into(),
        range: state.range,
    });
    let (active_task, elapsed) = state.data.active.as_ref().map_or_else(
        || ("".into(), "00:00".into()),
        |active| (
            task_name(&state.data, &active.task_id).to_uppercase().into(),
            elapsed(timestamp - active.started),
        ),
    );
    ui.set_tracking(TrackingState { active_task, elapsed });
}

fn main() -> Result<(), slint::PlatformError> {
    let path = default_path();
    let mut data = load(&path);
    recover_active(&mut data);
    let state = Rc::new(RefCell::new(State {
        data,
        path,
        range: Range::Day,
        editing_id: None,
    }));
    let ui = AppWindow::new()?;
    refresh(&ui, &state.borrow());
    persist(&ui, &state.borrow());
    let weak = ui.as_weak();
    let state_for_timer = state.clone();
    let timer = Timer::default();
    timer.start(TimerMode::Repeated, Duration::from_secs(1), move || {
        if let Some(ui) = weak.upgrade() {
            let mut state = state_for_timer.borrow_mut();
            if let Some(active) = &mut state.data.active {
                active.checkpoint = now();
                persist(&ui, &state);
            }
            refresh(&ui, &state);
        }
    });
    {
        let weak = ui.as_weak();
        let state = state.clone();
        ui.on_start_task(move |id| {
            if let Some(ui) = weak.upgrade() {
                let mut state = state.borrow_mut();
                let timestamp = now();
                state.data.active = Some(ActiveSession {
                    task_id: id.to_string(),
                    started: timestamp,
                    checkpoint: timestamp,
                });
                persist(&ui, &state);
                ui.set_page(Page::Tracking);
                refresh(&ui, &state);
            }
        });
    }
    {
        let weak = ui.as_weak();
        ui.on_open_settings(move || {
            if let Some(ui) = weak.upgrade() {
                ui.set_page(Page::Settings);
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = state.clone();
        ui.on_open_last_session(move || {
            if let Some(ui) = weak.upgrade() {
                if !state.borrow().data.sessions.is_empty() {
                    ui.set_page(Page::Note);
                }
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = state.clone();
        ui.on_end_task(move || {
            if let Some(ui) = weak.upgrade() {
                let mut state = state.borrow_mut();
                if let Some(active) = state.data.active.take() {
                    state.data.sessions.push(Session {
                        task_id: active.task_id,
                        started: active.started,
                        ended: now(),
                        note: String::new(),
                    });
                    persist(&ui, &state);
                    ui.set_page(Page::Note);
                    refresh(&ui, &state);
                }
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = state.clone();
        ui.on_save_note(move |note| {
            if let Some(ui) = weak.upgrade() {
                let mut state = state.borrow_mut();
                if let Some(session) = state.data.sessions.last_mut() {
                    session.note = note.to_string();
                }
                persist(&ui, &state);
                ui.set_page(Page::Home);
                refresh(&ui, &state);
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = state.clone();
        ui.on_skip_note(move || {
            if let Some(ui) = weak.upgrade() {
                persist(&ui, &state.borrow());
                ui.set_page(Page::Home);
                refresh(&ui, &state.borrow());
            }
        });
    }
    {
        let weak = ui.as_weak();
        ui.on_open_evaluation(move || {
            if let Some(ui) = weak.upgrade() {
                ui.set_page(Page::Evaluation);
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = state.clone();
        ui.on_choose_range(move |range| {
            if let Some(ui) = weak.upgrade() {
                state.borrow_mut().range = range;
                refresh(&ui, &state.borrow());
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = state.clone();
        ui.on_open_add_task(move || {
            if let Some(ui) = weak.upgrade() {
                let mut state = state.borrow_mut();
                state.editing_id = None;
                set_task_dialog(&ui, true, false, "");
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = state.clone();
        ui.on_open_rename_task(move |id| {
            if let Some(ui) = weak.upgrade() {
                let mut state = state.borrow_mut();
                let task = state
                    .data
                    .tasks
                    .iter()
                    .find(|t| t.id == id.as_str())
                    .map(|t| t.title.clone())
                    .unwrap_or_default();
                state.editing_id = Some(id.to_string());
                set_task_dialog(&ui, true, true, task);
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = state.clone();
        ui.on_save_task(move |name| {
            if let Some(ui) = weak.upgrade() {
                let mut state = state.borrow_mut();
                let name = name.trim();
                if name.is_empty() {
                    set_status(&ui, "Task name cannot be empty");
                    return;
                }
                if let Some(id) = state.editing_id.take() {
                    if let Some(task) = state.data.tasks.iter_mut().find(|t| t.id == id) {
                        task.title = name.into();
                    }
                } else {
                    state.data.tasks.push(Task {
                        id: format!("task-{}", now()),
                        title: name.into(),
                    });
                }
                persist(&ui, &state);
                set_task_dialog(&ui, false, false, "");
                refresh(&ui, &state);
            }
        });
    }
    {
        let weak = ui.as_weak();
        ui.on_close_task_dialog(move || {
            if let Some(ui) = weak.upgrade() {
                set_task_dialog(&ui, false, false, "");
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = state.clone();
        ui.on_choose_data_file(move || {
            if let Some(ui) = weak.upgrade() {
                if let Some(path) = FileDialog::new()
                    .add_filter("Tempo data", &["json"])
                    .pick_file()
                {
                    let mut state = state.borrow_mut();
                    state.path = path;
                    state.data = load(&state.path);
                    recover_active(&mut state.data);
                    persist(&ui, &state);
                    refresh(&ui, &state);
                    set_status(&ui, "Data file changed");
                }
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = state.clone();
        ui.on_new_data_file(move || {
            if let Some(ui) = weak.upgrade() {
                if let Some(path) = FileDialog::new()
                    .add_filter("Tempo data", &["json"])
                    .set_file_name("tempo.json")
                    .save_file()
                {
                    let mut state = state.borrow_mut();
                    state.path = path;
                    state.data = default_data();
                    persist(&ui, &state);
                    refresh(&ui, &state);
                    set_status(&ui, "New data file created");
                }
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = state.clone();
        ui.on_delete_data_file(move || {
            if let Some(ui) = weak.upgrade() {
                let mut state = state.borrow_mut();
                if fs::remove_file(&state.path).is_ok() {
                    state.data = default_data();
                    persist(&ui, &state);
                    refresh(&ui, &state);
                    set_status(&ui, "Data file reset");
                }
            }
        });
    }
    {
        let weak = ui.as_weak();
        let state = state.clone();
        ui.on_export_markdown(move || {
            if let Some(ui) = weak.upgrade() {
                let state = state.borrow();
                let report = markdown(&state.data, state.range, now());
                if let Some(path) = FileDialog::new()
                    .add_filter("Markdown", &["md"])
                    .set_file_name("tempo-report.md")
                    .save_file()
                {
                    match fs::write(path, report) {
                        Ok(()) => set_status(&ui, "Markdown exported"),
                        Err(error) => set_status(&ui, format!("Export failed: {error}")),
                    }
                }
            }
        });
    }
    let _timer = timer;
    ui.run()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn duration_range_and_markdown_are_readable() {
        let mut data = default_data();
        data.sessions.push(Session {
            task_id: "task-1".into(),
            started: 0,
            ended: 2520,
            note: "Brief".into(),
        });
        assert_eq!(duration(2520), "42 min");
        assert!(in_range(&data.sessions[0], Range::Year, 2520));
        assert!(markdown(&data, Range::Year, 2520).contains("Project Atlas"));
    }

    #[test]
    fn data_round_trips_and_recovers_interrupted_timer() {
        let mut data = default_data();
        data.active = Some(ActiveSession {
            task_id: "task-2".into(),
            started: 10,
            checkpoint: 40,
        });
        let text = serde_json::to_string(&data).unwrap();
        let mut restored: Data = serde_json::from_str(&text).unwrap();
        recover_active(&mut restored);
        assert!(restored.active.is_none());
        assert_eq!(
            restored.sessions[0].ended - restored.sessions[0].started,
            30
        );
    }
}
