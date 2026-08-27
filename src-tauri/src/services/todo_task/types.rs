//! Todo task master/sub types aligned with `todo_tasks.schema.json`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Built-in default category id (same entity as unspecified create fallback).
pub const DEFAULT_CATEGORY_ID: &str = "uncategorized";

/// Built-in default category display name.
pub const DEFAULT_CATEGORY_NAME: &str = "待分类";

fn default_category_id() -> String {
    DEFAULT_CATEGORY_ID.to_string()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SubTaskStatus {
    Incomplete,
    Complete,
    Abandoned,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MasterTaskStatus {
    #[default]
    Incomplete,
    Complete,
    Abandoned,
}

fn is_absent_or_empty_content(value: &Option<String>) -> bool {
    value.as_ref().map_or(true, |s| s.is_empty())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Category {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub is_default: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CategoriesFile {
    pub version: u32,
    pub categories: Vec<Category>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubTask {
    pub sub_task_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "is_absent_or_empty_content")]
    pub content: Option<String>,
    pub status: SubTaskStatus,
    pub implicit: bool,
    #[serde(default)]
    pub linked_archive_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MasterTask {
    pub master_task_id: String,
    pub title: String,
    #[serde(default)]
    pub status: MasterTaskStatus,
    pub created_at: String,
    pub sub_tasks: Vec<SubTask>,
    #[serde(default = "default_category_id")]
    pub category_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TodoTasksFile {
    pub version: u32,
    pub tasks: HashMap<String, MasterTask>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TodoTasksIndex {
    pub version: u32,
    pub tasks: HashMap<String, IndexEntry>,
}

impl Default for TodoTasksIndex {
    fn default() -> Self {
        Self {
            version: 2,
            tasks: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexEntry {
    pub master_task_id: String,
    pub title: String,
    #[serde(default)]
    pub status: MasterTaskStatus,
    pub created_at: String,
    pub task_dir: String,
    #[serde(default = "default_category_id")]
    pub category_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubTasksFile {
    pub sub_tasks: Vec<SubTask>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachmentEntry {
    pub file_name: String,
    pub original_file_name: String,
    pub added_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachmentsFile {
    pub attachments: Vec<AttachmentEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommentEntry {
    pub id: String,
    pub body: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommentsFile {
    pub comments: Vec<CommentEntry>,
}

pub fn index_entry_task_dir(master_task_id: &str) -> String {
    format!("tasks/{master_task_id}")
}

pub fn attachments_json_rel_path(master_task_id: &str) -> String {
    format!("tasks/{master_task_id}/attachments.json")
}

pub fn comments_json_rel_path(master_task_id: &str) -> String {
    format!("tasks/{master_task_id}/comments.json")
}

#[cfg(test)]
#[path = "../../unit-tests/services/todo_task_types.rs"]
mod types_tests;
