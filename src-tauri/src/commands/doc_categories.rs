//! Settings commands for the document classification list.

use serde_json::{json, Value};

use crate::services::docs::{self, DocCategory};

fn payload(items: Vec<DocCategory>) -> Value {
    json!({ "ok": true, "categories": items })
}

#[tauri::command]
pub fn get_doc_categories() -> Result<Value, String> {
    Ok(payload(docs::load_doc_categories()?))
}

#[tauri::command]
pub fn set_doc_categories(categories: Vec<DocCategory>) -> Result<Value, String> {
    docs::save_doc_categories(&categories)?;
    Ok(payload(categories))
}

#[cfg(test)]
#[path = "../unit-tests/commands/doc_categories.rs"]
mod tests;
