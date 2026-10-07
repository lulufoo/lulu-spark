//! Write IDE ticket handles into the current login launchd environment.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::config::settings;
use crate::services::mcp_oauth::{ledger_record, Slot, TicketHandle, TicketState};

const AGENT_PREFIX: &str = "com.lulu-spark.env.";

pub fn apply_ide_ticket_env(
    slot: Slot,
    handle: &TicketHandle,
    previous_env: Option<&str>,
) -> Result<(), String> {
    let Some(base) = slot.env_var_base() else {
        return Ok(());
    };
    if cfg!(test) || settings::is_test_sandbox() {
        return Ok(());
    }
    let new_var = ledger_record(slot)
        .ok()
        .flatten()
        .and_then(|record| record.env_var())
        .unwrap_or_else(|| base.to_string());
    remove_leftover_launch_agents(slot);
    if let Some(previous) = previous_env {
        if previous != new_var {
            launchctl_unsetenv(previous)?;
        }
    }
    if base != new_var {
        launchctl_unsetenv(base)?;
    }
    launchctl_setenv(&new_var, handle.as_str())
}

pub(crate) fn live_ide_slots() -> Vec<Slot> {
    [Slot::CursorIde, Slot::Codex, Slot::Claude]
        .into_iter()
        .filter(|slot| {
            matches!(
                ledger_record(*slot),
                Ok(Some(record)) if record.state == TicketState::Live
            )
        })
        .collect()
}

pub fn apply_live_ide_ticket_envs() {
    for slot in live_ide_slots() {
        let Ok(Some(record)) = ledger_record(slot) else {
            continue;
        };
        if let Err(err) = apply_ide_ticket_env(slot, &record.handle, None) {
            eprintln!("[mcp-ide-env] {} setenv failed: {err}", slot.as_str());
        }
    }
}

fn leftover_names(slot: Slot) -> &'static [&'static str] {
    match slot {
        Slot::CursorIde => &["cursor", "cursor_ide"],
        Slot::Codex => &["codex"],
        Slot::Claude => &["claude"],
        Slot::Spark => &[],
    }
}

fn launch_agents_dir() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join("Library/LaunchAgents"))
}

fn remove_leftover_launch_agents(slot: Slot) {
    let Some(dir) = launch_agents_dir() else {
        return;
    };
    for name in leftover_names(slot) {
        let path = dir.join(format!("{AGENT_PREFIX}{name}.plist"));
        let _ = fs::remove_file(path);
    }
}

fn launchctl_setenv(var: &str, value: &str) -> Result<(), String> {
    launchctl(&["setenv", var, value])
}

fn launchctl_unsetenv(var: &str) -> Result<(), String> {
    launchctl(&["unsetenv", var])
}

fn launchctl(args: &[&str]) -> Result<(), String> {
    let status = Command::new("/bin/launchctl")
        .args(args)
        .status()
        .map_err(|err| format!("launchctl {}: {err}", args.first().copied().unwrap_or("")))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "launchctl {} exited {status}",
            args.first().copied().unwrap_or("")
        ))
    }
}

#[cfg(test)]
#[path = "../unit-tests/commands/mcp_ide_env.rs"]
mod tests;
