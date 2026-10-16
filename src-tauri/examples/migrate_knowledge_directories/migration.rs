//! One run: import json, skip missing/deprecated folders, reconcile, then delete json.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use lulu_spark_lib::services::sediment_kb::generate_directory_id;
use lulu_spark_lib::services::spark_data::{
    get_knowledge_directory, get_knowledge_directory_by_folder, insert_knowledge_directory,
    list_knowledge_directories, open_at, update_knowledge_directory, KnowledgeDirectoryRow,
    DB_FILE, UNCATEGORIZED,
};
use rusqlite::Connection;
use serde::Deserialize;

const SKIP_FOLDERS: &[&str] = &["ai-dev-docs", "lulu-workflow-framework"];

#[derive(Debug, Deserialize)]
struct CategoriesFile {
    #[serde(default)]
    categories: Vec<JsonCategory>,
}

#[derive(Debug, Deserialize)]
struct JsonCategory {
    id: String,
    name: String,
}

#[derive(Debug, Deserialize)]
struct ReposFile {
    repos: Vec<JsonRepo>,
}

#[derive(Debug, Clone, Deserialize)]
struct JsonRepo {
    full_name: String,
    #[serde(default)]
    description: String,
    category_id: String,
}

pub struct Outcome {
    pub row_count: usize,
    pub skipped: usize,
}

fn knowledge_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("knowledge")
}

fn repos_json(data_dir: &Path) -> PathBuf {
    knowledge_dir(data_dir).join("repos.json")
}

fn categories_json(data_dir: &Path) -> PathBuf {
    knowledge_dir(data_dir).join("categories.json")
}

fn db_path(data_dir: &Path) -> PathBuf {
    data_dir.join(DB_FILE)
}

fn folder_of(full_name: &str) -> &str {
    full_name.rsplit('/').next().unwrap_or(full_name)
}

fn should_skip(folder: &str) -> bool {
    SKIP_FOLDERS.contains(&folder)
}

fn category_name(names: &HashMap<String, String>, category_id: &str) -> String {
    let name = names
        .get(category_id)
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .unwrap_or(UNCATEGORIZED);
    if name == "未分类" || name.eq_ignore_ascii_case("uncategorized") {
        UNCATEGORIZED.to_string()
    } else {
        name.to_string()
    }
}

fn read_repos(data_dir: &Path) -> Result<Vec<JsonRepo>, String> {
    let path = repos_json(data_dir);
    if !path.is_file() {
        return Err(format!("repos.json not found at {}", path.display()));
    }
    let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let file: ReposFile = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    Ok(file.repos)
}

fn read_category_names(data_dir: &Path) -> Result<HashMap<String, String>, String> {
    let path = categories_json(data_dir);
    if !path.is_file() {
        return Ok(HashMap::new());
    }
    let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let file: CategoriesFile = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    Ok(file.categories.into_iter().map(|c| (c.id, c.name)).collect())
}

fn mint_id(conn: &Connection) -> Result<String, String> {
    let used: std::collections::HashSet<String> = list_knowledge_directories(conn)?
        .into_iter()
        .map(|row| row.id)
        .collect();
    for _ in 0..8 {
        let id = generate_directory_id();
        if !used.contains(&id) {
            return Ok(id);
        }
    }
    Err("Could not allocate directory id".into())
}

fn import_item(
    conn: &Connection,
    folder: &str,
    category: &str,
    description: &str,
) -> Result<(), String> {
    let incoming_fields = (folder, category, description);
    match get_knowledge_directory_by_folder(conn, folder)? {
        Some(existing)
            if existing.folder == incoming_fields.0
                && existing.category == incoming_fields.1
                && existing.description == incoming_fields.2 =>
        {
            Ok(())
        }
        Some(existing) => {
            let mut row = existing;
            row.category = category.to_string();
            row.description = description.to_string();
            update_knowledge_directory(conn, &row).map(|_| ())
        }
        None => {
            let id = mint_id(conn)?;
            insert_knowledge_directory(
                conn,
                &KnowledgeDirectoryRow::new(&id, folder, category, description),
            )
        }
    }
}

/// Import `data_dir/knowledge/repos.json` into `data_dir/spark-data.sqlite`.
pub fn run_migration(data_dir: &Path) -> Result<Outcome, String> {
    let repos = read_repos(data_dir)?;
    let names = read_category_names(data_dir)?;
    let kb = knowledge_dir(data_dir);
    let mut imported: Vec<(String, String, String)> = Vec::new();
    let mut skipped = 0;
    for repo in &repos {
        let folder = folder_of(&repo.full_name);
        if should_skip(folder) || !kb.join(folder).is_dir() {
            skipped += 1;
            continue;
        }
        imported.push((
            folder.to_string(),
            category_name(&names, &repo.category_id),
            repo.description.clone(),
        ));
    }

    let conn = open_at(&db_path(data_dir))?;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    for (folder, category, description) in &imported {
        import_item(&tx, folder, category, description)?;
    }
    let row_count = imported.len();
    for (folder, category, description) in &imported {
        let row = get_knowledge_directory_by_folder(&tx, folder)?
            .ok_or_else(|| format!("imported folder {folder} missing after write"))?;
        if row.category != *category || row.description != *description {
            return Err(format!("imported folder {folder} did not keep its fields"));
        }
        if get_knowledge_directory(&tx, &row.id)?.is_none() {
            return Err(format!("imported id {} missing after write", row.id));
        }
    }
    tx.commit().map_err(|e| e.to_string())?;
    fs::remove_file(repos_json(data_dir)).map_err(|e| e.to_string())?;
    let cats = categories_json(data_dir);
    if cats.is_file() {
        fs::remove_file(cats).map_err(|e| e.to_string())?;
    }
    Ok(Outcome { row_count, skipped })
}
