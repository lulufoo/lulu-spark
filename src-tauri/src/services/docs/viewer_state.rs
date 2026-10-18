//! What each workspace's page had selected, per Project:
//! `{cache_dir}/workspace-viewer-state/{workspace_id}.json`.
//!
//! ```json
//! { "version": 2, "project": "<last project id, empty = All>",
//!   "projects": { "<project id>": { "node": "<dir node key>", "doc": "<doc id>", "date": "<day>" } } }
//! ```
//!
//! The file format keeps the `project` key; only the API names it `project_id`.
//!
//! The directory view records `node` + `doc`, the timeline only `date`. A project without an
//! entry has no record. File missing, corrupt or of the old `{ id }` shape → empty, no overwrite;
//! a missing file is not seeded.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::config::paths;
use crate::repositories::atomic_json;
use crate::services::spark_data::validate_workspace_id;

use super::resolve::{bad_request, internal, open_data, resolve_workspace, text_field};

static WRITE_LOCK: Mutex<()> = Mutex::new(());

const VERSION: u32 = 2;

/// What one project had selected; an empty field means nothing was recorded for it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
struct ProjectState {
    #[serde(default)]
    node: String,
    #[serde(default)]
    doc: String,
    #[serde(default)]
    date: String,
}

impl ProjectState {
    fn is_empty(&self) -> bool {
        self.node.is_empty() && self.doc.is_empty() && self.date.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ViewerStateFile {
    #[serde(default)]
    version: u32,
    #[serde(default)]
    project: String,
    #[serde(default)]
    projects: BTreeMap<String, ProjectState>,
}

impl Default for ViewerStateFile {
    fn default() -> Self {
        Self { version: VERSION, project: String::new(), projects: BTreeMap::new() }
    }
}

fn file_path(workspace_id: &str) -> Result<PathBuf, String> {
    validate_workspace_id(workspace_id)?;
    paths::workspace_viewer_state_path(workspace_id).map_err(|e| format!("{e:?}"))
}

fn load_unlocked(workspace_id: &str) -> ViewerStateFile {
    let Ok(path) = file_path(workspace_id) else {
        return ViewerStateFile::default();
    };
    let Ok(text) = fs::read_to_string(path) else {
        return ViewerStateFile::default();
    };
    match serde_json::from_str::<ViewerStateFile>(&text) {
        Ok(data) if data.version == VERSION => ViewerStateFile {
            version: VERSION,
            project: data.project.trim().to_string(),
            projects: data
                .projects
                .into_iter()
                .filter(|(folder, _)| !folder.trim().is_empty())
                .map(|(folder, st)| {
                    let st = ProjectState {
                        node: st.node.trim().to_string(),
                        doc: st.doc.trim().to_string(),
                        date: st.date.trim().to_string(),
                    };
                    (folder, st)
                })
                .filter(|(_, st)| !st.is_empty())
                .collect(),
        },
        _ => ViewerStateFile::default(),
    }
}

fn workspace_id_of(payload: &Value) -> Result<String, Value> {
    let conn = open_data()?;
    resolve_workspace(&conn, text_field(payload, "workspace_id")).map(|ws| ws.id)
}

/// A field of the patch: `None` = not given (leave alone), `Some("")` = clear.
fn patch_field(payload: &Value, key: &str) -> Result<Option<String>, Value> {
    match payload.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.trim().to_string())),
        Some(_) => Err(bad_request(format!("{key} must be a string"))),
    }
}

/// The file shape.
fn to_file_value(data: &ViewerStateFile) -> Value {
    json!({ "version": data.version, "project": data.project, "projects": data.projects })
}

/// The API shape: the file's `project` is named `project_id`.
fn to_value(data: &ViewerStateFile) -> Value {
    json!({ "version": data.version, "project_id": data.project, "projects": data.projects })
}

/// `{ workspace_id? }` → `{ version, project_id, projects }`; empty when nothing is remembered.
pub fn get_viewer_state(payload: &Value) -> Value {
    let workspace = match workspace_id_of(payload) {
        Ok(id) => id,
        Err(v) => return v,
    };
    let _guard = WRITE_LOCK.lock().ok();
    to_value(&load_unlocked(&workspace))
}

/// `{ workspace_id?, project_id, node?, doc?, date? }` → the whole state. `project_id` (empty = All) is
/// the last selected project; the other fields update that project's entry: left out = kept,
/// empty = cleared.
pub fn set_viewer_state(payload: &Value) -> Value {
    let workspace = match workspace_id_of(payload) {
        Ok(id) => id,
        Err(v) => return v,
    };
    let project = match patch_field(payload, "project_id") {
        Ok(Some(project)) => project,
        Ok(None) => return bad_request("Missing project_id"),
        Err(v) => return v,
    };
    let mut patch = Vec::new();
    for key in ["node", "doc", "date"] {
        match patch_field(payload, key) {
            Ok(value) => patch.push(value),
            Err(v) => return v,
        }
    }
    let _guard = WRITE_LOCK.lock().ok();
    let path = match file_path(&workspace) {
        Ok(path) => path,
        Err(e) => return internal(e),
    };
    let mut data = load_unlocked(&workspace);
    if !project.is_empty() {
        let mut entry = data.projects.remove(&project).unwrap_or_default();
        let mut patch = patch.into_iter();
        for field in [&mut entry.node, &mut entry.doc, &mut entry.date] {
            if let Some(Some(value)) = patch.next() {
                *field = value;
            }
        }
        if !entry.is_empty() {
            data.projects.insert(project.clone(), entry);
        }
    }
    data.project = project;
    match atomic_json::write_json(&path, &to_file_value(&data)) {
        Ok(()) => to_value(&data),
        Err(e) => internal(e),
    }
}

#[cfg(test)]
#[path = "../../unit-tests/services/docs_viewer_state.rs"]
mod tests;
