//! What the Worker sends back, checked and typed.

use std::fmt;

use serde_json::Value;

use super::error::CloudSyncError;
use crate::services::spark_data::is_valid_doc_id;

/// A short-lived, read-only token for the user's bucket (`POST /credentials`).
#[derive(Clone, PartialEq, Eq)]
pub struct Credentials {
    pub bucket: String,
    pub access_token: String,
    /// Seconds since the Unix epoch.
    pub expires_at: i64,
}

impl fmt::Debug for Credentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Credentials")
            .field("bucket", &self.bucket)
            .field("access_token", &"<redacted>")
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

/// One document in the cloud: its id and the version stored with it (`GET /documents`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloudDoc {
    pub document_id: String,
    pub version: i64,
}

fn bad(what: impl Into<String>) -> CloudSyncError {
    CloudSyncError::BadResponse(what.into())
}

fn text<'a>(object: &'a Value, field: &str) -> Result<&'a str, CloudSyncError> {
    match object.get(field).and_then(Value::as_str) {
        Some(s) if !s.is_empty() => Ok(s),
        _ => Err(bad(format!("`{field}` is not a non-empty string"))),
    }
}

fn whole_number(object: &Value, field: &str) -> Result<i64, CloudSyncError> {
    object
        .get(field)
        .and_then(Value::as_i64)
        .ok_or_else(|| bad(format!("`{field}` is not a whole number")))
}

pub(super) fn parse_credentials(body: &Value) -> Result<Credentials, CloudSyncError> {
    Ok(Credentials {
        bucket: text(body, "bucket")?.to_string(),
        access_token: text(body, "accessToken")?.to_string(),
        expires_at: whole_number(body, "expiresAt")?,
    })
}

pub(super) fn parse_documents(body: &Value) -> Result<Vec<CloudDoc>, CloudSyncError> {
    let list = body
        .get("documents")
        .and_then(Value::as_array)
        .ok_or_else(|| bad("`documents` is not a list"))?;
    list.iter()
        .map(|item| {
            let document_id = text(item, "documentId")?;
            if !is_valid_doc_id(document_id) {
                return Err(bad(format!("`{document_id}` is not a document id")));
            }
            let version = whole_number(item, "version")?;
            if version < 0 {
                return Err(bad(format!("version of `{document_id}` is negative")));
            }
            Ok(CloudDoc {
                document_id: document_id.to_string(),
                version,
            })
        })
        .collect()
}

#[cfg(test)]
#[path = "../../unit-tests/services/cloud_sync_types.rs"]
mod tests;
