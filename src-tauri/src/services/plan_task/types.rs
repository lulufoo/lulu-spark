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
