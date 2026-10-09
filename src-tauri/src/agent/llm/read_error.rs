//! Maps a failed read of the response stream to an [`LlmError`].

use std::error::Error as StdError;

use super::LlmError;
use crate::agent::session;

pub(super) fn map_read_error(error: std::io::Error) -> LlmError {
    if io_error_is_timeout(&error) {
        let _ = session::log_agent_error("LLM timeout reading stream");
        return LlmError::Timeout;
    }
    let _ = session::log_agent_error(&format!("LLM stream read error: {error}"));
    LlmError::Network(error.to_string())
}

fn io_error_is_timeout(error: &std::io::Error) -> bool {
    if error.kind() == std::io::ErrorKind::TimedOut {
        return true;
    }
    let mut current: Option<&dyn StdError> = Some(error);
    while let Some(err) = current {
        let lower = err.to_string().to_ascii_lowercase();
        if lower.contains("timed out") || lower.contains("timeout") {
            return true;
        }
        current = err.source();
    }
    false
}
