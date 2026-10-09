//! Request-scoped chat progress. L4 calls a sink; L1 binds it to a Tauri Channel.

use std::sync::Arc;
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::agent::diagnostics::TraceId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgressDesc {
    pub session_id: String,
    pub request_id: String,
    pub desc: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thinking: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thinking_ms: Option<u64>,
}

pub type ProgressSink = Arc<dyn Fn(ProgressDesc) + Send + Sync>;

/// One user Turn's thinking: pieces joined with a blank line; duration is the
/// sum of reasoning streams only (tool time is not added).
#[derive(Default)]
pub struct TurnThinking {
    text: String,
    done_ms: u64,
    call_started: Option<Instant>,
}

impl TurnThinking {
    pub fn tick(&mut self, call_reasoning: &str) -> (Option<String>, Option<u64>) {
        if !call_reasoning.is_empty() && self.call_started.is_none() {
            self.call_started = Some(Instant::now());
        }
        self.view(Some(call_reasoning))
    }

    pub fn finish_call(&mut self, call_reasoning: Option<&str>) {
        if let Some(started) = self.call_started.take() {
            self.done_ms = self
                .done_ms
                .saturating_add(started.elapsed().as_millis() as u64);
        }
        if let Some(piece) = call_reasoning.filter(|text| !text.is_empty()) {
            append_thinking(&mut self.text, piece);
        }
    }

    pub fn view(&self, live_call: Option<&str>) -> (Option<String>, Option<u64>) {
        let mut live = self.text.clone();
        if let Some(piece) = live_call.filter(|text| !text.is_empty()) {
            append_thinking(&mut live, piece);
        }
        let extra = self
            .call_started
            .map(|started| started.elapsed().as_millis() as u64)
            .unwrap_or(0);
        let ms = self.done_ms.saturating_add(extra);
        ((!live.is_empty()).then_some(live), (ms > 0).then_some(ms))
    }

    pub fn persist_ms(&self) -> Option<u64> {
        self.view(None).1
    }
}

fn append_thinking(dst: &mut String, piece: &str) {
    if piece.is_empty() {
        return;
    }
    if !dst.is_empty() {
        dst.push_str("\n\n");
    }
    dst.push_str(piece);
}

pub fn noop_sink() -> ProgressSink {
    Arc::new(|_| {})
}

pub fn emit_progress(
    sink: Option<&ProgressSink>,
    session_id: &str,
    request_id: &TraceId,
    desc: impl Into<String>,
) {
    emit_progress_state(sink, session_id, request_id, desc, None, None);
}

pub fn emit_progress_state(
    sink: Option<&ProgressSink>,
    session_id: &str,
    request_id: &TraceId,
    desc: impl Into<String>,
    thinking: Option<String>,
    thinking_ms: Option<u64>,
) {
    let Some(sink) = sink else {
        return;
    };
    sink(ProgressDesc {
        session_id: session_id.to_string(),
        request_id: request_id.as_str().to_string(),
        desc: desc.into(),
        thinking,
        thinking_ms,
    });
}
