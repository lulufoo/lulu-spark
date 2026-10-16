//! One run: import json, insert `ai` if missing, reconcile, then delete the file.

use std::fs;
use std::path::Path;

use lulu_spark_lib::services::notes::generate_category_id;
use lulu_spark_lib::services::spark_data::{
    get_note_category, get_note_category_by_folder, insert_note_category, list_note_categories,
    open_at, NoteCategoryRow, DB_FILE,
};
use rusqlite::Connection;
use serde::Deserialize;

const AI_FOLDER: &str = "ai";

#[derive(Debug, Deserialize)]
struct CategoriesFile {
    categories: Vec<JsonCategory>,
}

#[derive(Debug, Clone, Deserialize)]
struct JsonCategory {
    id: String,
    #[serde(default)]
    folder: String,
    title: String,
    #[serde(default)]
    description: String,
}

pub struct Outcome {
    pub row_count: usize,
    pub inserted_ai: bool,
}

fn json_path(data_dir: &Path) -> std::path::PathBuf {
    data_dir.join("notes").join("categories.json")
}

fn db_path(data_dir: &Path) -> std::path::PathBuf {
    data_dir.join(DB_FILE)
}

fn read_json(data_dir: &Path) -> Result<Vec<JsonCategory>, String> {
    let path = json_path(data_dir);
    if !path.is_file() {
        return Err(format!("categories.json not found at {}", path.display()));
    }
    let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let file: CategoriesFile = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    Ok(file.categories)
}

fn import_item(conn: &Connection, item: &JsonCategory) -> Result<(), String> {
    let incoming = NoteCategoryRow::new(&item.id, &item.folder, &item.title, &item.description);
    match get_note_category(conn, &item.id)? {
        Some(existing) if existing == incoming => Ok(()),
        Some(_) => Err(format!(
            "category {} already exists with different fields",
            item.id
        )),
        None => insert_note_category(conn, &incoming),
    }
}

fn mint_ai_id(conn: &Connection) -> Result<String, String> {
    let used: std::collections::HashSet<String> = list_note_categories(conn)?
        .into_iter()
        .map(|row| row.id)
        .collect();
    for _ in 0..8 {
        let id = generate_category_id();
        if !used.contains(&id) {
            return Ok(id);
        }
    }
    Err("Could not allocate category id".into())
}

fn insert_ai_if_missing(conn: &Connection) -> Result<bool, String> {
    if get_note_category_by_folder(conn, AI_FOLDER)?.is_some() {
        return Ok(false);
    }
    let id = mint_ai_id(conn)?;
    insert_note_category(conn, &NoteCategoryRow::new(&id, AI_FOLDER, AI_FOLDER, ""))?;
    Ok(true)
}

/// Import `data_dir/notes/categories.json` into `data_dir/spark-data.sqlite`.
/// The database write and the row-count check share one transaction: a mismatch
/// rolls it back and the json file is not deleted.
pub fn run_migration(data_dir: &Path) -> Result<Outcome, String> {
    let items = read_json(data_dir)?;
    let ai_missing = !items.iter().any(|c| c.folder == AI_FOLDER);
    let expected = items.len() + usize::from(ai_missing);

    let conn = open_at(&db_path(data_dir))?;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    for item in &items {
        import_item(&tx, item)?;
    }
    let inserted_ai = insert_ai_if_missing(&tx)?;
    let row_count = list_note_categories(&tx)?.len();
    if row_count != expected {
        return Err(format!(
            "row count {row_count} != expected {expected}; nothing was written or deleted"
        ));
    }
    for item in &items {
        let row = get_note_category(&tx, &item.id)?
            .ok_or_else(|| format!("imported id {} missing after write", item.id))?;
        if row.folder != item.folder || row.title != item.title || row.description != item.description
        {
            return Err(format!("imported id {} did not keep its fields", item.id));
        }
    }
    tx.commit().map_err(|e| e.to_string())?;
    fs::remove_file(json_path(data_dir)).map_err(|e| e.to_string())?;
    Ok(Outcome {
        row_count,
        inserted_ai,
    })
}
