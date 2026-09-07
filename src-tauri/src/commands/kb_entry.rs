use serde_json::Value;
use tauri::AppHandle;

use crate::config::paths;
use crate::services::knowledge;

fn repo_root() -> Result<std::path::PathBuf, String> {
    paths::repo_root().map_err(|e| format!("{e:?}"))
}

#[tauri::command]
pub fn kb_create(
    _app: AppHandle,
    repo: String,
    parent: String,
    name: String,
    kind: String,
) -> Result<Value, String> {
    Ok(knowledge::kb_create(&repo_root()?, repo, parent, name, kind))
}

#[tauri::command]
pub fn kb_delete(_app: AppHandle, repo: String, path: String) -> Result<Value, String> {
    Ok(knowledge::kb_delete(&repo_root()?, repo, path))
}
