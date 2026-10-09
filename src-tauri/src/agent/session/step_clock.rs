//! Wall-clock marks of one `Step`, stored on its `model_steps` row.
//!
//! Unit is unix **milliseconds**. `messages.created_at` is seconds; do not mix them.

use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

pub fn read_unix_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// `None` means "not instrumented" (old rows, bare `append_step` callers), never 0.
///
/// Fixed when the `Step` is pushed and never rewritten: `turn_store::sync` compares whole
/// Steps, so a changed row is deleted with its suffix and re-inserted, which also rewrites
/// `messages.created_at`.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct StepClock {
    /// The `Step` was pushed into memory.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub create_ts: Option<i64>,
    /// This row's own work began (model request sent, tool `invoke` started, user sent).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_ts: Option<i64>,
    /// This row's own work ended.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_ts: Option<i64>,
}

impl StepClock {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// A row with no model or tool work of its own: the user's send, or a Host-written reply.
    pub fn stamp_now() -> Self {
        let now = read_unix_millis();
        Self {
            create_ts: Some(now),
            start_ts: Some(now),
            end_ts: Some(now),
        }
    }

    /// A row whose own work ran from `start_ts` to `end_ts`; pushed now.
    pub fn stamp_span(start_ts: i64, end_ts: i64) -> Self {
        Self {
            create_ts: Some(read_unix_millis()),
            start_ts: Some(start_ts),
            end_ts: Some(end_ts),
        }
    }
}
