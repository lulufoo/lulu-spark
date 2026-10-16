//! Knowledge topic directories (`knowledge_directory` in spark-data).

use std::collections::HashSet;
use std::fs;
use std::sync::Mutex;

use chrono::{FixedOffset, Utc};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::config::paths;
use crate::services::id::random_alnum6;
use crate::services::spark_data::{
    self, clear_knowledge_category, delete_knowledge_directory, get_knowledge_directory_by_folder,
    insert_knowledge_directory, list_knowledge_category_names, list_knowledge_directories,
    rename_knowledge_category, update_knowledge_directory, KnowledgeDirectoryRow, UNCATEGORIZED,
};

pub const UNCATEGORIZED_ID: &str = UNCATEGORIZED;
pub const UNCATEGORIZED_NAME: &str = UNCATEGORIZED;

static WRITE_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopicRow {
    pub repo: String,
    pub description: String,
    pub category_id: String,
    pub category_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Category {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CategoriesFile {
    pub version: u32,
    pub categories: Vec<Category>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoEntry {
    pub full_name: String,
    #[serde(default)]
    pub description: String,
    pub category_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReposFile {
    pub version: u32,
    pub repos: Vec<RepoEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SedimentKbError {
    InvalidFormat,
    NotAccessible(String),
    Duplicate,
    InvalidName,
    ProtectedCategory,
    CategoryNotFound,
    RepoNotFound,
    Io(String),
}

impl SedimentKbError {
    pub fn invalid_format() -> Self {
        Self::InvalidFormat
    }
    pub fn not_accessible(message: impl Into<String>) -> Self {
        Self::NotAccessible(message.into())
    }
    pub fn is_duplicate(&self) -> bool {
        matches!(self, Self::Duplicate)
    }
    pub fn is_invalid_format(&self) -> bool {
        matches!(self, Self::InvalidFormat)
    }
    pub fn is_not_accessible(&self) -> bool {
        matches!(self, Self::NotAccessible(_))
    }
    pub fn is_invalid_name(&self) -> bool {
        matches!(self, Self::InvalidName)
    }
    pub fn is_protected_category(&self) -> bool {
        matches!(self, Self::ProtectedCategory)
    }
}

impl std::fmt::Display for SedimentKbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidFormat => write!(f, "invalid repo format"),
            Self::NotAccessible(msg) | Self::Io(msg) => write!(f, "{msg}"),
            Self::Duplicate => write!(f, "duplicate repo"),
            Self::InvalidName => write!(f, "invalid category name"),
            Self::ProtectedCategory => write!(f, "protected category"),
            Self::CategoryNotFound => write!(f, "category not found"),
            Self::RepoNotFound => write!(f, "repo not found"),
        }
    }
}

fn with_write_lock<F, T>(f: F) -> T
where
    F: FnOnce() -> T,
{
    let _guard = WRITE_LOCK.lock().expect("sediment_kb write lock");
    f()
}

fn db_err(e: String) -> SedimentKbError {
    SedimentKbError::Io(e)
}

fn open_db() -> Result<Connection, SedimentKbError> {
    spark_data::open().map_err(db_err)
}

fn ensure_knowledge_dir() -> Result<(), SedimentKbError> {
    let data = paths::runtime_data_dir().map_err(|e| SedimentKbError::Io(format!("{e:?}")))?;
    crate::services::knowledge_layout::ensure_knowledge_registry_layout(&data)
        .map_err(SedimentKbError::Io)?;
    let dir = paths::knowledge_root().map_err(|e| SedimentKbError::Io(format!("{e:?}")))?;
    fs::create_dir_all(&dir).map_err(|e| SedimentKbError::Io(e.to_string()))
}

fn entry_from_row(row: KnowledgeDirectoryRow) -> RepoEntry {
    RepoEntry { full_name: row.folder, description: row.description, category_id: row.category }
}

fn category_list(names: Vec<String>) -> CategoriesFile {
    CategoriesFile {
        version: 1,
        categories: names.into_iter().map(|name| Category { id: name.clone(), name }).collect(),
    }
}

pub fn generate_directory_id() -> String {
    let tz = FixedOffset::east_opt(8 * 3600).expect("UTC+8");
    let now = Utc::now().with_timezone(&tz);
    format!("{}{:03}{}", now.format("%Y%m%d%H%M%S"), now.timestamp_subsec_millis(), random_alnum6())
}

fn mint_directory_id(conn: &Connection) -> Result<String, SedimentKbError> {
    let used: HashSet<String> = list_knowledge_directories(conn)
        .map_err(db_err)?
        .into_iter()
        .map(|row| row.id)
        .collect();
    for _ in 0..8 {
        let id = generate_directory_id();
        if !used.contains(&id) {
            return Ok(id);
        }
    }
    Err(SedimentKbError::Io("Could not allocate directory id".into()))
}

fn normalize_dir_name(input: &str) -> Result<String, SedimentKbError> {
    let name = input.trim();
    if name.is_empty() || name == "." || name == ".." || name.contains('/') || name.contains('\\') || name.contains('\0')
    {
        return Err(SedimentKbError::invalid_format());
    }
    Ok(name.to_string())
}

fn normalize_category_name(input: &str) -> Result<String, SedimentKbError> {
    let name = input.trim();
    if name.is_empty() { Err(SedimentKbError::InvalidName) } else { Ok(name.to_string()) }
}

pub fn ensure_uncategorized() -> Result<(), SedimentKbError> {
    with_write_lock(|| {
        ensure_knowledge_dir()?;
        open_db().map(|_| ())
    })
}

pub fn load_categories() -> Result<CategoriesFile, SedimentKbError> {
    ensure_uncategorized()?;
    let conn = open_db()?;
    Ok(category_list(
        list_knowledge_category_names(&conn).map_err(db_err)?,
    ))
}

pub fn load_repos() -> Result<ReposFile, SedimentKbError> {
    ensure_uncategorized()?;
    let conn = open_db()?;
    let repos = list_knowledge_directories(&conn)
        .map_err(db_err)?
        .into_iter()
        .map(entry_from_row)
        .collect();
    Ok(ReposFile { version: 1, repos })
}

pub fn list_repos_for_topics() -> Result<Vec<TopicRow>, SedimentKbError> {
    Ok(load_repos()?
        .repos
        .into_iter()
        .map(|r| TopicRow {
            repo: r.full_name,
            description: r.description,
            category_id: r.category_id.clone(),
            category_name: r.category_id,
        })
        .collect())
}

pub fn add_directory(name: &str) -> Result<(), SedimentKbError> {
    with_write_lock(|| {
        ensure_knowledge_dir()?;
        let dir_name = normalize_dir_name(name)?;
        let conn = open_db()?;
        if get_knowledge_directory_by_folder(&conn, &dir_name)
            .map_err(db_err)?
            .is_some()
        {
            return Err(SedimentKbError::Duplicate);
        }
        let root = paths::knowledge_root().map_err(|e| SedimentKbError::Io(format!("{e:?}")))?;
        fs::create_dir_all(root.join(&dir_name)).map_err(|e| SedimentKbError::Io(e.to_string()))?;
        let id = mint_directory_id(&conn)?;
        insert_knowledge_directory(&conn, &KnowledgeDirectoryRow::new(&id, &dir_name, UNCATEGORIZED, ""))
            .map_err(db_err)
    })
}

pub fn remove_repo(full_name: &str) -> Result<(), SedimentKbError> {
    with_write_lock(|| {
        let folder = normalize_dir_name(full_name)?;
        let conn = open_db()?;
        let Some(row) = get_knowledge_directory_by_folder(&conn, &folder).map_err(db_err)? else {
            return Err(SedimentKbError::RepoNotFound);
        };
        if delete_knowledge_directory(&conn, &row.id).map_err(db_err)? { Ok(()) } else { Err(SedimentKbError::RepoNotFound) }
    })
}

pub fn update_repo_category(full_name: &str, category_id: &str) -> Result<(), SedimentKbError> {
    with_write_lock(|| {
        let folder = normalize_dir_name(full_name)?;
        let category = normalize_category_name(category_id)?;
        let conn = open_db()?;
        let Some(mut row) = get_knowledge_directory_by_folder(&conn, &folder).map_err(db_err)? else {
            return Err(SedimentKbError::RepoNotFound);
        };
        row.category = category;
        if !update_knowledge_directory(&conn, &row).map_err(db_err)? {
            return Err(SedimentKbError::RepoNotFound);
        }
        Ok(())
    })
}

pub fn add_category(name: &str) -> Result<String, SedimentKbError> {
    normalize_category_name(name)
}

pub fn rename_category(id: &str, name: &str) -> Result<(), SedimentKbError> {
    with_write_lock(|| {
        let from = normalize_category_name(id)?;
        let to = normalize_category_name(name)?;
        if from == UNCATEGORIZED {
            return Err(SedimentKbError::ProtectedCategory);
        }
        let conn = open_db()?;
        if rename_knowledge_category(&conn, &from, &to).map_err(db_err)? == 0 {
            return Err(SedimentKbError::CategoryNotFound);
        }
        Ok(())
    })
}

pub fn remove_category(id: &str) -> Result<(), SedimentKbError> {
    with_write_lock(|| {
        let name = normalize_category_name(id)?;
        if name == UNCATEGORIZED {
            return Err(SedimentKbError::ProtectedCategory);
        }
        let conn = open_db()?;
        if clear_knowledge_category(&conn, &name).map_err(db_err)? == 0 {
            return Err(SedimentKbError::CategoryNotFound);
        }
        Ok(())
    })
}

#[cfg(test)]
#[path = "../unit-tests/services/sediment_kb.rs"]
mod tests;
