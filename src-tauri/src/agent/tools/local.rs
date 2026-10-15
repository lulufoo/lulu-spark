//! Runs local (non-MCP) tools under the time limit.
//!
//! Tools that read or write the disk run in the timeout shell. Tools that only look at
//! the Session in memory (`list_staged`, `get_staged`) run inline. `stage` is split: the
//! disk check runs in the shell, the registration (memory only) runs inline.

use std::time::Duration;

use serde_json::Value;

use crate::agent::fence::PathFence;
use crate::agent::prompt::{read_tool_timeout_text, write_tool_timeout_text};
use crate::agent::session::Session;

use super::catalog::ToolResult;
use super::timeout::{run_with_timeout, Waited};
use super::{host, stage};

pub enum Local {
    Done(ToolResult),
    /// The limit passed. The result says whether it is safe to retry.
    TimedOut(ToolResult),
    /// The turn was cancelled while the tool was running.
    Cancelled,
}

/// Run one local tool. `read_only` picks the timeout text: a read-only tool that timed out
/// changed nothing; any other tool may have.
pub fn call(
    name: &str,
    arguments: &Value,
    session: &mut Session,
    fence: Option<&PathFence>,
    read_only: bool,
    limit: Duration,
    is_cancelled: &dyn Fn() -> bool,
) -> Local {
    match name {
        "list_staged" | "get_staged" => Local::Done(stage::call(name, arguments, session, fence)),
        "stage" => {
            let (arguments, fence) = (arguments.clone(), fence.cloned());
            let waited = run_with_timeout(limit, is_cancelled, move |_| {
                stage::check_stage(&arguments, fence.as_ref())
            });
            match settle(waited, read_only) {
                Ok(Ok(checked)) => Local::Done(stage::register_stage(session, checked)),
                Ok(Err(content)) => Local::Done(error(content)),
                Err(local) => local,
            }
        }
        _ => {
            let Some(fence) = fence else {
                return Local::Done(error(format!(
                    "Host tool '{name}' has no path fence for this binding."
                )));
            };
            let (name, arguments, fence) = (name.to_string(), arguments.clone(), fence.clone());
            let waited = run_with_timeout(limit, is_cancelled, move |deadline| {
                host::call_until(&name, &arguments, &fence, Some(deadline))
            });
            match settle(waited, read_only) {
                Ok(result) => Local::Done(result),
                Err(local) => local,
            }
        }
    }
}

fn settle<T>(waited: Waited<T>, read_only: bool) -> Result<T, Local> {
    match waited {
        Waited::Done(value) => Ok(value),
        Waited::TimedOut => Err(Local::TimedOut(timeout_result(read_only))),
        Waited::Cancelled => Err(Local::Cancelled),
        Waited::Crashed => Err(Local::Done(error(
            "The tool stopped unexpectedly before it returned a result.".into(),
        ))),
    }
}

fn timeout_result(read_only: bool) -> ToolResult {
    let content = if read_only {
        read_tool_timeout_text()
    } else {
        write_tool_timeout_text()
    };
    error(content.into())
}

fn error(content: String) -> ToolResult {
    ToolResult {
        content,
        is_error: true,
    }
}

#[cfg(test)]
#[path = "../../unit-tests/agent/tools_local_tests.rs"]
mod tests;
