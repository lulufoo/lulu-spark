//! Plan task master/sub types aligned with `plan_tasks.schema.json`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SubTaskStatus {
    Incomplete,
    Complete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MasterTaskStatus {
    Incomplete,
    Complete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubTask {
    pub sub_task_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
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
    pub status: MasterTaskStatus,
    pub created_at: String,
    pub sub_tasks: Vec<SubTask>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanTasksFile {
    pub version: u32,
    pub tasks: HashMap<String, MasterTask>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanTasksIndex {
    pub version: u32,
    pub tasks: HashMap<String, IndexEntry>,
}

impl Default for PlanTasksIndex {
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
    pub status: MasterTaskStatus,
    pub created_at: String,
    pub task_dir: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubTasksFile {
    pub sub_tasks: Vec<SubTask>,
}

pub fn index_entry_task_dir(master_task_id: &str) -> String {
    format!("tasks/{master_task_id}")
}

#[cfg(test)]
#[path = "../../unit-tests/services/plan_task_types.rs"]
mod types_tests;
