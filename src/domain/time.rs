//! Clock access for tracker timestamps.

use std::time::{SystemTime, UNIX_EPOCH};

/// Returns the current Unix timestamp in whole seconds.
///
/// A system clock before the Unix epoch yields zero.
pub(crate) fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
