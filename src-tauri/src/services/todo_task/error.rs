//! Domain error for todo_task. `_status` is attached only by `into_wire` for L1 / tests.

use serde_json::{json, Value};

pub(super) const CORRUPT_STORAGE_ERROR: &str = "Invalid todo_tasks storage";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TodoError {
    pub message: String,
    pub status: u16,
}

impl TodoError {
    pub fn with_status(status: u16, message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            status,
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::with_status(400, message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::with_status(404, message)
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::with_status(409, message)
    }

    pub fn payload_too_large(message: impl Into<String>) -> Self {
        Self::with_status(413, message)
    }

    pub fn unavailable(message: impl Into<String>) -> Self {
        Self::with_status(503, message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::with_status(500, message)
    }

    pub fn corrupt() -> Self {
        Self::internal(CORRUPT_STORAGE_ERROR)
    }

    /// L1 / test wire only. Domain functions return `TodoError`, they do not call this.
    pub fn into_wire(self) -> Value {
        json!({ "error": self.message, "_status": self.status })
    }
}

pub(super) fn bootstrap_error(err: String) -> TodoError {
    TodoError::internal(err)
}

pub(super) fn corrupt_storage_error() -> TodoError {
    TodoError::corrupt()
}

/// L1 / test wire: success objects get `_status`; arrays stay bare (list_all).
pub fn into_wire(result: Result<Value, TodoError>, ok_status: u16) -> Value {
    match result {
        Ok(Value::Array(items)) => Value::Array(items),
        Ok(mut body) => {
            if let Some(obj) = body.as_object_mut() {
                obj.insert("_status".to_string(), json!(ok_status));
            }
            body
        }
        Err(err) => err.into_wire(),
    }
}

/// L1 / test wire for get/list-shaped success (no `_status` on Ok).
pub fn into_wire_read(result: Result<Value, TodoError>) -> Value {
    match result {
        Ok(value) => value,
        Err(err) => err.into_wire(),
    }
}
