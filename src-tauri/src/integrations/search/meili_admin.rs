//! Meilisearch admin HTTP (index wipe, settings, batch documents, tasks).

use std::thread;
use std::time::Duration;

use reqwest::blocking::Client;
use reqwest::Method;
use serde_json::Value;

use super::meili_backend::{documents_path, MeiliBackend};

#[derive(Debug)]
pub enum MeiliAdminError {
    Unavailable(String),
    Request(String),
    TaskFailed(String),
}

impl MeiliAdminError {
    pub fn into_message(self) -> String {
        match self {
            MeiliAdminError::Unavailable(msg) => format!("MEILI_UNAVAILABLE: {msg}"),
            MeiliAdminError::Request(msg) => msg,
            MeiliAdminError::TaskFailed(msg) => msg,
        }
    }
}

impl MeiliBackend {
    fn admin_client(&self) -> Result<Client, MeiliAdminError> {
        Client::builder()
            .timeout(Duration::from_secs(15))
            .no_proxy()
            .build()
            .map_err(|e| MeiliAdminError::Request(e.to_string()))
    }

    pub fn require_health(&self) -> Result<(), MeiliAdminError> {
        let client = self.admin_client()?;
        let url = format!("{}/health", self.url.trim_end_matches('/'));
        let resp = client
            .get(&url)
            .send()
            .map_err(|e| MeiliAdminError::Unavailable(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(MeiliAdminError::Unavailable(format!(
                "HTTP {}",
                resp.status()
            )));
        }
        let body: Value = resp
            .json()
            .map_err(|e| MeiliAdminError::Unavailable(e.to_string()))?;
        if body.get("status").and_then(|v| v.as_str()) != Some("available") {
            return Err(MeiliAdminError::Unavailable(format!(
                "status={}",
                body.get("status").unwrap_or(&Value::Null)
            )));
        }
        Ok(())
    }

    pub fn admin_request(
        &self,
        method: Method,
        path: &str,
        body: Option<&Value>,
    ) -> Result<Value, MeiliAdminError> {
        let client = self.admin_client()?;
        let url = format!("{}{}", self.url.trim_end_matches('/'), path);
        let mut req = client
            .request(method, &url)
            .header("Authorization", format!("Bearer {}", self.master_key))
            .header("Content-Type", "application/json");
        if let Some(b) = body {
            req = req.json(b);
        }
        let resp = req
            .send()
            .map_err(|e| MeiliAdminError::Request(e.to_string()))?;
        let status = resp.status();
        let text = resp.text().unwrap_or_default();
        if status.is_success() {
            if text.trim().is_empty() {
                return Ok(Value::Null);
            }
            serde_json::from_str(&text).map_err(|e| MeiliAdminError::Request(e.to_string()))
        } else if status.as_u16() == 404 {
            Err(MeiliAdminError::Request(format!("HTTP 404: {text}")))
        } else {
            Err(MeiliAdminError::Request(format!("HTTP {status}: {text}")))
        }
    }

    pub fn task_uid(task: &Value) -> u64 {
        task.get("taskUid")
            .or_else(|| task.get("uid"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
    }

    pub fn wait_for_task(&self, task_uid: u64, max_wait_secs: u64) -> Result<(), MeiliAdminError> {
        if task_uid == 0 {
            return Ok(());
        }
        for _ in 0..max_wait_secs {
            let r = self.admin_request(Method::GET, &format!("/tasks/{task_uid}"), None)?;
            let status = r.get("status").and_then(|v| v.as_str()).unwrap_or("");
            if matches!(status, "succeeded" | "failed" | "canceled") {
                if status == "succeeded" {
                    return Ok(());
                }
                return Err(MeiliAdminError::TaskFailed(format!(
                    "task {task_uid} {status}"
                )));
            }
            thread::sleep(Duration::from_secs(1));
        }
        Err(MeiliAdminError::TaskFailed(format!(
            "task {task_uid} timeout after {max_wait_secs}s"
        )))
    }

    pub fn wipe_index(&self, index_uid: &str) -> Result<(), MeiliAdminError> {
        match self.admin_request(Method::DELETE, &format!("/indexes/{index_uid}"), None) {
            Ok(task) => self.wait_for_task(Self::task_uid(&task), 30),
            Err(MeiliAdminError::Request(msg)) if msg.contains("404") => Ok(()),
            Err(e) => Err(e),
        }
    }

    pub fn ensure_index(&self, index_uid: &str, primary_key: &str) -> Result<(), MeiliAdminError> {
        if self
            .admin_request(Method::GET, &format!("/indexes/{index_uid}"), None)
            .is_err()
        {
            let task = self.admin_request(
                Method::POST,
                "/indexes",
                Some(&serde_json::json!({
                    "uid": index_uid,
                    "primaryKey": primary_key,
                })),
            )?;
            self.wait_for_task(Self::task_uid(&task), 30)?;
        }
        Ok(())
    }

    pub fn put_settings(
        &self,
        index_uid: &str,
        setting: &str,
        body: &Value,
    ) -> Result<(), MeiliAdminError> {
        let path = format!("/indexes/{index_uid}/settings/{setting}");
        let task = self.admin_request(Method::PUT, &path, Some(body))?;
        self.wait_for_task(Self::task_uid(&task), 30)
    }

    pub fn post_documents(&self, index_uid: &str, batch: &[Value]) -> Result<(), MeiliAdminError> {
        let path = documents_path(index_uid);
        let task = self.admin_request(Method::POST, &path, Some(&serde_json::json!(batch)))?;
        self.wait_for_task(Self::task_uid(&task), 120)
    }

    pub fn put_documents(&self, index_uid: &str, batch: &[Value]) -> Result<(), MeiliAdminError> {
        let path = documents_path(index_uid);
        let task = self.admin_request(Method::PUT, &path, Some(&serde_json::json!(batch)))?;
        self.wait_for_task(Self::task_uid(&task), 120)
    }

    pub fn delete_documents_by_filter(
        &self,
        index_uid: &str,
        filter: &str,
    ) -> Result<(), MeiliAdminError> {
        let path = format!("/indexes/{index_uid}/documents/delete");
        let task = self.admin_request(
            Method::POST,
            &path,
            Some(&serde_json::json!({ "filter": filter })),
        )?;
        self.wait_for_task(Self::task_uid(&task), 60)
    }
}
