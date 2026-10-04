//! Sediment knowledge-base SSOT (`paths::sediment_kb_*` under `spark_root`).

use std::fs;
use std::sync::Mutex;
#[cfg(test)]
use std::sync::OnceLock;

use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::config::paths;
use crate::integrations::github;
use crate::repositories::atomic_json;

use super::id::random_hex12;

pub const UNCATEGORIZED_ID: &str = "uncategorized";

static WRITE_LOCK: Mutex<()> = Mutex::new(());

#[cfg(test)]
type RepoValidatorFn = fn(&str) -> Result<String, SedimentKbError>;

#[cfg(test)]
static TEST_REPO_VALIDATOR: OnceLock<Mutex<Option<RepoValidatorFn>>> = OnceLock::new();

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
            Self::NotAccessible(msg) => write!(f, "{msg}"),
            Self::Duplicate => write!(f, "duplicate repo"),
            Self::InvalidName => write!(f, "invalid category name"),
            Self::ProtectedCategory => write!(f, "protected category"),
            Self::CategoryNotFound => write!(f, "category not found"),
            Self::RepoNotFound => write!(f, "repo not found"),
            Self::Io(msg) => write!(f, "{msg}"),
        }
    }
}

#[cfg(test)]
pub fn set_test_repo_validator(validator: Option<RepoValidatorFn>) {
    let lock = TEST_REPO_VALIDATOR.get_or_init(|| Mutex::new(None));
    *lock.lock().expect("test validator lock") = validator;
}

fn with_write_lock<F, T>(f: F) -> T
where
    F: FnOnce() -> T,
{
    let _guard = WRITE_LOCK.lock().expect("sediment_kb write lock");
    f()
}

fn default_categories_file() -> CategoriesFile {
    CategoriesFile {
        version: 1,
        categories: vec![Category {
            id: UNCATEGORIZED_ID.to_string(),
            name: "未分类".to_string(),
        }],
    }
}

fn default_repos_file() -> ReposFile {
    ReposFile {
        version: 1,
        repos: vec![],
    }
}

fn ensure_storage_dir() -> Result<(), SedimentKbError> {
    if let Ok(wb) = paths::spark_root() {
        crate::services::knowledge_layout::ensure_knowledge_registry_layout(&wb)
            .map_err(SedimentKbError::Io)?;
    }
    let dir = paths::sediment_kb_dir().map_err(|e| SedimentKbError::Io(format!("{e:?}")))?;
    fs::create_dir_all(&dir).map_err(|e| SedimentKbError::Io(e.to_string()))
}

fn read_json_file<T: for<'de> Deserialize<'de>>(path: &std::path::Path) -> Result<T, SedimentKbError> {
    let text = fs::read_to_string(path).map_err(|e| SedimentKbError::Io(e.to_string()))?;
    serde_json::from_str(&text).map_err(|e| SedimentKbError::Io(e.to_string()))
}

fn write_json_file<T: Serialize>(path: &std::path::Path, data: &T) -> Result<(), SedimentKbError> {
    let value = serde_json::to_value(data).map_err(|e| SedimentKbError::Io(e.to_string()))?;
    atomic_json::write_json(path, &value).map_err(SedimentKbError::Io)
}

pub fn load_categories() -> Result<CategoriesFile, SedimentKbError> {
    ensure_storage_dir()?;
    let path = paths::sediment_kb_categories_path().map_err(|e| SedimentKbError::Io(format!("{e:?}")))?;
    if !path.is_file() {
        return Ok(default_categories_file());
    }
    read_json_file(&path)
}

pub fn save_categories(data: &CategoriesFile) -> Result<(), SedimentKbError> {
    with_write_lock(|| save_categories_unlocked(data))
}

fn save_categories_unlocked(data: &CategoriesFile) -> Result<(), SedimentKbError> {
    ensure_storage_dir()?;
    let path =
        paths::sediment_kb_categories_path().map_err(|e| SedimentKbError::Io(format!("{e:?}")))?;
    write_json_file(&path, data)
}

pub fn load_repos() -> Result<ReposFile, SedimentKbError> {
    ensure_storage_dir()?;
    let path = paths::sediment_kb_repos_path().map_err(|e| SedimentKbError::Io(format!("{e:?}")))?;
    if !path.is_file() {
        return Ok(default_repos_file());
    }
    read_json_file(&path)
}

pub fn save_repos(data: &ReposFile) -> Result<(), SedimentKbError> {
    with_write_lock(|| save_repos_unlocked(data))
}

fn save_repos_unlocked(data: &ReposFile) -> Result<(), SedimentKbError> {
    ensure_storage_dir()?;
    let path = paths::sediment_kb_repos_path().map_err(|e| SedimentKbError::Io(format!("{e:?}")))?;
    write_json_file(&path, data)
}

pub fn ensure_uncategorized() -> Result<(), SedimentKbError> {
    with_write_lock(ensure_uncategorized_unlocked)
}

pub fn list_repos_for_topics() -> Result<Vec<TopicRow>, SedimentKbError> {
    ensure_uncategorized()?;
    let categories = load_categories()?;
    let repos = load_repos()?;
    let name_by_id: std::collections::HashMap<&str, &str> = categories
        .categories
        .iter()
        .map(|c| (c.id.as_str(), c.name.as_str()))
        .collect();
    Ok(repos
        .repos
        .iter()
        .map(|r| TopicRow {
            repo: r.full_name.clone(),
            description: r.description.clone(),
            category_id: r.category_id.clone(),
            category_name: name_by_id
                .get(r.category_id.as_str())
                .copied()
                .unwrap_or("未分类")
                .to_string(),
        })
        .collect())
}

fn ensure_uncategorized_unlocked() -> Result<(), SedimentKbError> {
    ensure_storage_dir()?;
    let cat_path =
        paths::sediment_kb_categories_path().map_err(|e| SedimentKbError::Io(format!("{e:?}")))?;
    let repo_path =
        paths::sediment_kb_repos_path().map_err(|e| SedimentKbError::Io(format!("{e:?}")))?;

    if !cat_path.is_file() {
        save_categories_unlocked(&default_categories_file())?;
    } else {
        let mut cats = load_categories()?;
        if !cats
            .categories
            .iter()
            .any(|c| c.id == UNCATEGORIZED_ID)
        {
            cats.categories.insert(
                0,
                Category {
                    id: UNCATEGORIZED_ID.to_string(),
                    name: "未分类".to_string(),
                },
            );
            save_categories_unlocked(&cats)?;
        }
    }

    if !repo_path.is_file() {
        save_repos_unlocked(&default_repos_file())?;
    }
    Ok(())
}

fn normalize_full_name(input: &str) -> Result<String, SedimentKbError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(SedimentKbError::invalid_format());
    }
    if let Some(rest) = trimmed.strip_prefix("https://github.com/") {
        let parts: Vec<&str> = rest.trim_end_matches('/').split('/').collect();
        if parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty() {
            return Ok(format!("{}/{}", parts[0], parts[1]));
        }
        return Err(SedimentKbError::invalid_format());
    }
    let parts: Vec<&str> = trimmed.split('/').collect();
    if parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty() {
        return Ok(format!("{}/{}", parts[0], parts[1]));
    }
    Err(SedimentKbError::invalid_format())
}

fn validate_repo_access(full_name: &str) -> Result<String, SedimentKbError> {
    #[cfg(test)]
    if let Some(lock) = TEST_REPO_VALIDATOR.get() {
        if let Some(validator) = *lock.lock().expect("test validator lock") {
            return validator(full_name);
        }
    }

    let parts: Vec<&str> = full_name.split('/').collect();
    if parts.len() != 2 {
        return Err(SedimentKbError::invalid_format());
    }
    let (owner, repo) = (parts[0], parts[1]);
    match github::request(Method::GET, &format!("repos/{owner}/{repo}"), None) {
        Ok(_) => Ok(full_name.to_string()),
        Err(e) => Err(SedimentKbError::not_accessible(e.message)),
    }
}

fn category_exists(categories: &CategoriesFile, category_id: &str) -> bool {
    categories.categories.iter().any(|c| c.id == category_id)
}

pub fn add_repo(
    full_name: &str,
    category_id: Option<&str>,
    description: &str,
) -> Result<(), SedimentKbError> {
    with_write_lock(|| {
        ensure_uncategorized_unlocked()?;
        let normalized = normalize_full_name(full_name)?;
        let validated = validate_repo_access(&normalized)?;

        let categories = load_categories()?;
        let cat_id = category_id.unwrap_or(UNCATEGORIZED_ID);
        if !category_exists(&categories, cat_id) {
            return Err(SedimentKbError::CategoryNotFound);
        }

        let mut repos = load_repos()?;
        if repos.repos.iter().any(|r| r.full_name == validated) {
            return Err(SedimentKbError::Duplicate);
        }

        repos.repos.push(RepoEntry {
            full_name: validated,
            description: description.trim().to_string(),
            category_id: cat_id.to_string(),
        });
        save_repos_unlocked(&repos)?;
        Ok(())
    })
}

pub fn remove_repo(full_name: &str) -> Result<(), SedimentKbError> {
    with_write_lock(|| {
        ensure_uncategorized_unlocked()?;
        let normalized = normalize_full_name(full_name)?;
        let mut repos = load_repos()?;
        let before = repos.repos.len();
        repos.repos.retain(|r| r.full_name != normalized);
        if repos.repos.len() == before {
            return Err(SedimentKbError::RepoNotFound);
        }
        save_repos_unlocked(&repos)
    })
}

pub fn update_repo_category(full_name: &str, category_id: &str) -> Result<(), SedimentKbError> {
    with_write_lock(|| {
        ensure_uncategorized_unlocked()?;
        let normalized = normalize_full_name(full_name)?;
        let categories = load_categories()?;
        if !category_exists(&categories, category_id) {
            return Err(SedimentKbError::CategoryNotFound);
        }
        let mut repos = load_repos()?;
        let Some(entry) = repos.repos.iter_mut().find(|r| r.full_name == normalized) else {
            return Err(SedimentKbError::RepoNotFound);
        };
        entry.category_id = category_id.to_string();
        save_repos_unlocked(&repos)
    })
}

pub fn add_category(name: &str) -> Result<String, SedimentKbError> {
    with_write_lock(|| {
        ensure_uncategorized_unlocked()?;
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(SedimentKbError::InvalidName);
        }
        let mut categories = load_categories()?;
        let id = format!("cat{}", random_hex12());
        categories.categories.push(Category {
            id: id.clone(),
            name: trimmed.to_string(),
        });
        save_categories_unlocked(&categories)?;
        Ok(id)
    })
}

pub fn rename_category(id: &str, name: &str) -> Result<(), SedimentKbError> {
    with_write_lock(|| {
        ensure_uncategorized_unlocked()?;
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(SedimentKbError::InvalidName);
        }
        let mut categories = load_categories()?;
        let Some(entry) = categories.categories.iter_mut().find(|c| c.id == id) else {
            return Err(SedimentKbError::CategoryNotFound);
        };
        entry.name = trimmed.to_string();
        save_categories_unlocked(&categories)
    })
}

pub fn remove_category(id: &str) -> Result<(), SedimentKbError> {
    with_write_lock(|| {
        ensure_uncategorized_unlocked()?;
        if id == UNCATEGORIZED_ID {
            return Err(SedimentKbError::ProtectedCategory);
        }
        let mut categories = load_categories()?;
        let before = categories.categories.len();
        categories.categories.retain(|c| c.id != id);
        if categories.categories.len() == before {
            return Err(SedimentKbError::CategoryNotFound);
        }

        let mut repos = load_repos()?;
        for repo in &mut repos.repos {
            if repo.category_id == id {
                repo.category_id = UNCATEGORIZED_ID.to_string();
            }
        }
        save_categories_unlocked(&categories)?;
        save_repos_unlocked(&repos)
    })
}

#[cfg(test)]
pub fn validate_paths_ssot_constraints() {
    let src = include_str!("sediment_kb.rs");
    let prod = src
        .split("pub fn validate_paths_ssot_constraints")
        .next()
        .expect("prod section");
    for forbidden in [".cache/sediment-kb", "cache_dir()", r#"join("sediment-kb")"#] {
        assert!(
            !prod.contains(forbidden),
            "sediment_kb must resolve paths via paths::sediment_kb_* (found {forbidden:?})"
        );
    }
    for required in [
        "paths::sediment_kb_dir",
        "paths::sediment_kb_categories_path",
        "paths::sediment_kb_repos_path",
        "atomic_json::write_json",
    ] {
        assert!(
            prod.contains(required),
            "sediment_kb must use {required}"
        );
    }
}

#[cfg(test)]
pub fn validate_description_source_constraints() {
    let sediment_kb = include_str!("sediment_kb.rs");
    let sediment_prod = sediment_kb
        .split("pub fn validate_description_source_constraints")
        .next()
        .expect("sediment prod");
    let write_cmd = include_str!("../commands/write.rs");
    let spark_read = include_str!("spark_read/config.rs");

    let validate_fn = sediment_prod
        .split("fn validate_repo_access")
        .nth(1)
        .and_then(|s| s.split("\nfn ").next())
        .expect("validate_repo_access");
    assert!(
        !validate_fn.contains("description"),
        "validate_repo_access must not extract GitHub description"
    );

    for (label, src) in [
        ("sediment_kb", sediment_prod),
        ("write", write_cmd),
        ("spark_read", spark_read),
    ] {
        for forbidden in ["README", "frontmatter"] {
            assert!(
                !src.contains(forbidden),
                "{label} must not use {forbidden} for description"
            );
        }
    }

    let get_topics = spark_read
        .split("pub fn get_topics")
        .nth(1)
        .and_then(|s| s.split("\npub fn ").next())
        .expect("get_topics");
    assert!(
        !get_topics.contains("index.json"),
        "get_topics must not read index.json for description"
    );
}

#[cfg(test)]
#[path = "../unit-tests/services/sediment_kb.rs"]
mod tests;
