//! Runs local (non-MCP) tools under the time limit.
//!
//! Tools that read or write the disk run in the timeout shell. Tools that only look at
//! the Session in memory (`list_staged`, `get_staged`) run inline. `stage` is split: the
//! disk check runs in the shell, the registration (memory only) runs inline.

use std::time::Duration;

use serde_json::Value;

use crate::agent::fence::PathFence;
use crate::agent::prompt::{read_tool_timeout_text, write_tool_crash_text, write_tool_timeout_text};
use crate::agent::session::Session;

use super::catalog::ToolResult;
use super::gate::{self, GateError};
use super::timeout::{run_with_timeout, Waited};
use super::{fs, host, stage};

/// Why a local tool ended without a result of its own. Either way the tool may have changed
/// something before it stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stopped {
    TimedOut,
    Crashed,
}

pub enum Local {
    Done(ToolResult),
    /// The tool did not finish. The result says whether it is safe to retry.
    Stopped(Stopped, ToolResult),
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
            let arguments = arguments.clone();
            let fence = fence.cloned();
            let fence_for_check = fence.clone();
            let waited = run_with_timeout(limit, is_cancelled, move |_| {
                stage::check_stage(&arguments, fence_for_check.as_ref())
            });
            match settle(waited, read_only) {
                Ok(Ok(checked)) => match gate::prepare_outside_write(
                    checked.path_for_gate(),
                    session,
                    fence.as_ref().unwrap_or(&PathFence::default()),
                    is_cancelled,
                ) {
                    Ok(_) => Local::Done(stage::register_stage(session, checked)),
                    Err(GateError::Cancelled) => Local::Cancelled,
                    Err(GateError::Denied(content)) => Local::Done(error(content)),
                },
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
            let fence = match grant_outside_dest(name, arguments, session, fence, is_cancelled) {
                Ok(next) => next,
                Err(GateError::Cancelled) => return Local::Cancelled,
                Err(GateError::Denied(content)) => return Local::Done(error(content)),
            };
            let (name, arguments) = (name.to_string(), arguments.clone());
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

fn grant_outside_dest(
    name: &str,
    arguments: &Value,
    session: &mut Session,
    fence: &PathFence,
    is_cancelled: &dyn Fn() -> bool,
) -> Result<PathFence, GateError> {
    if !matches!(name, "write" | "str_replace" | "copy") {
        return Ok(fence.clone());
    }
    let dest = if name == "copy" {
        fs::arg_str(arguments, "dest_path")
    } else {
        fs::arg_str(arguments, "path")
    }
    .map_err(GateError::Denied)?;
    let abs = crate::agent::fence::require_absolute(&dest).map_err(GateError::Denied)?;
    if fence.allows_write(&abs) {
        return Ok(fence.clone());
    }
    let canon = gate::prepare_outside_write(&dest, session, fence, is_cancelled)?;
    let mut next = fence.clone();
    next.grant_staged_file(canon);
    Ok(next)
}

fn settle<T>(waited: Waited<T>, read_only: bool) -> Result<T, Local> {
    match waited {
        Waited::Done(value) => Ok(value),
        Waited::TimedOut => Err(Local::Stopped(
            Stopped::TimedOut,
            timeout_result(read_only),
        )),
        Waited::Cancelled => Err(Local::Cancelled),
        Waited::Crashed => Err(Local::Stopped(Stopped::Crashed, crash_result(read_only))),
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

fn crash_result(read_only: bool) -> ToolResult {
    if read_only {
        error("The tool stopped unexpectedly before it returned a result.".into())
    } else {
        error(write_tool_crash_text().into())
    }
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
