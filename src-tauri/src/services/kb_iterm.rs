//! Open knowledge-base repo directory in iTerm (`server.py::_handle_open_iterm`).

use std::path::PathBuf;
use std::process::Command;

use serde_json::{json, Value};

use crate::config::paths;

fn applescript_cd_path(local_dir: &PathBuf) -> String {
    local_dir.display().to_string().replace('\\', "\\\\").replace('"', "\\\"")
}

pub fn open_kb_in_iterm(payload: &Value) -> Value {
    let repo = payload.get("repo").and_then(|v| v.as_str()).unwrap_or("").trim();
    if repo.is_empty() {
        return json!({ "error": "repo required", "_status": 400 });
    }
    let repo_name = repo.split('/').next_back().unwrap_or(repo);
    let kb_root = match paths::knowledge_corpus_root() {
        Ok(p) => p,
        Err(e) => return json!({ "error": format!("{e:?}"), "_status": 500 }),
    };
    let local_dir = kb_root.join(repo_name);
    if !local_dir.is_dir() {
        return json!({
            "error": format!("repo not cloned locally: {repo_name}"),
            "_status": 404
        });
    }
    let cd = applescript_cd_path(&local_dir);
    let script = format!(
        r#"tell application "iTerm"
    activate
    if (count of windows) = 0 then
        create window with default profile
    end if
    tell current window
        create tab with default profile
        tell current session of current tab
            write text "cd \"{cd}\""
        end tell
    end tell
end tell"#
    );
    let output = Command::new("osascript").arg("-e").arg(&script).output();
    match output {
        Ok(o) if o.status.success() => json!({ "ok": true }),
        Ok(o) => {
            let err = String::from_utf8_lossy(&o.stderr).trim().to_string();
            json!({
                "error": if err.is_empty() { "osascript failed".into() } else { err },
                "_status": 500
            })
        }
        Err(e) => json!({ "error": e.to_string(), "_status": 500 }),
    }
}

#[cfg(test)]
#[path = "../unit-tests/services/kb_iterm.rs"]
mod tests;
