//! Core tracker concepts and their in-memory lifecycle rules.
//!
//! This module keeps project identity, recorded tasks, active tracking, time
//! ranges, and aggregate mutations independent of the UI and SQLite adapter.
//! The application layer coordinates persistence around these operations.

mod data;
mod ids;
mod project;
mod range;
mod task;
mod time;

pub(crate) use data::Data;
pub(crate) use ids::{ProjectId, TaskId};
pub(crate) use project::{validate_project_name, Project};
pub(crate) use range::{in_range, Range};
pub(crate) use task::{ActiveTask, Task};
pub(crate) use time::now;
