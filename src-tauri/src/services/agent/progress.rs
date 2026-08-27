//! Request-scoped chat progress. L4 calls a sink; L1 binds it to a Tauri Channel.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::services::agent::diagnostics::TraceId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgressDesc {
    pub session_id: String,
    pub request_id: String,
    pub desc: String,
}

pub type ProgressSink = Arc<dyn Fn(ProgressDesc) + Send + Sync>;

pub fn noop_sink() -> ProgressSink {
    Arc::new(|_| {})
}

pub fn emit_progress(
    sink: Option<&ProgressSink>,
    session_id: &str,
    request_id: &TraceId,
    desc: impl Into<String>,
) {
    let Some(sink) = sink else {
        return;
    };
    sink(ProgressDesc {
        session_id: session_id.to_string(),
        request_id: request_id.as_str().to_string(),
        desc: desc.into(),
    });
}
