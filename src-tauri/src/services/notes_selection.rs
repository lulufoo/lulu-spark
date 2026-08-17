//! Host-authoritative Notes selection snapshot. Sidecar HTTP is the only write path.

use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotesSelectionDocument {
    pub id: String,
    pub selected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotesSelectionSnapshot {
    pub date: Option<String>,
    pub documents: Vec<NotesSelectionDocument>,
}

impl Default for NotesSelectionSnapshot {
    fn default() -> Self {
        Self {
            date: None,
            documents: Vec::new(),
        }
    }
}

static STORE: Mutex<NotesSelectionSnapshot> = Mutex::new(NotesSelectionSnapshot {
    date: None,
    documents: Vec::new(),
});

fn snapshot_to_value(snapshot: &NotesSelectionSnapshot) -> Value {
    serde_json::to_value(snapshot).unwrap_or_else(|_| json!({ "date": null, "documents": [] }))
}

pub fn reset_snapshot() {
    let mut guard = STORE.lock().expect("notes selection store");
    *guard = NotesSelectionSnapshot::default();
}

pub fn notes_selection_snapshot_get() -> Value {
    let guard = STORE.lock().expect("notes selection store");
    snapshot_to_value(&guard)
}

pub fn notes_selection_snapshot_put(body: &Value) -> Value {
    let date = match body.get("date") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) => Some(s.clone()),
        Some(_) => return json!({ "error": "Invalid date", "_status": 400 }),
    };
    let Some(docs_val) = body.get("documents").and_then(|v| v.as_array()) else {
        return json!({ "error": "Missing documents", "_status": 400 });
    };
    let mut documents = Vec::with_capacity(docs_val.len());
    for doc in docs_val {
        let Some(id) = doc.get("id").and_then(|v| v.as_str()) else {
            return json!({ "error": "Invalid document", "_status": 400 });
        };
        let Some(selected) = doc.get("selected").and_then(|v| v.as_bool()) else {
            return json!({ "error": "Invalid document", "_status": 400 });
        };
        documents.push(NotesSelectionDocument {
            id: id.to_string(),
            selected,
        });
    }
    let snapshot = NotesSelectionSnapshot { date, documents };
    let value = snapshot_to_value(&snapshot);
    let mut guard = STORE.lock().expect("notes selection store");
    *guard = snapshot;
    value
}
