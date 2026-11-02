//! What the Worker sends back, checked and typed.

use std::fmt;

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
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

/// A signed link that writes one object once (`POST /uploads`). The link is a credential for its
/// few minutes of life, so `Debug` leaves it out.
#[derive(Clone)]
pub struct UploadLink {
    pub url: String,
    /// Sent with the bytes exactly as given: Google checks them against the signature.
    pub headers: HeaderMap,
}

impl fmt::Debug for UploadLink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UploadLink").field("url", &"<redacted>").finish_non_exhaustive()
    }
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

/// Google's bucket naming rules, which is all that goes into a URL: 3 to 63 characters of
/// `a-z 0-9 - _ .`, starting and ending with a letter or digit.
fn is_valid_bucket_name(name: &str) -> bool {
    let edge_ok = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit();
    (3..=63).contains(&name.len())
        && name.chars().all(|c| edge_ok(c) || matches!(c, '-' | '_' | '.'))
        && name.starts_with(edge_ok)
        && name.ends_with(edge_ok)
}

/// A document id goes into a URL path, so anything but 32 lower-case hex characters is refused.
pub(super) fn check_document_id(id: &str) -> Result<(), CloudSyncError> {
    if is_valid_doc_id(id) {
        Ok(())
    } else {
        Err(CloudSyncError::InvalidInput("not a document id".into()))
    }
}

pub(super) fn parse_credentials(body: &Value) -> Result<Credentials, CloudSyncError> {
    let bucket = text(body, "bucket")?;
    if !is_valid_bucket_name(bucket) {
        return Err(bad("`bucket` is not a bucket name"));
    }
    Ok(Credentials {
        bucket: bucket.to_string(),
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

pub(super) fn parse_upload_link(body: &Value) -> Result<UploadLink, CloudSyncError> {
    if text(body, "method")? != "PUT" {
        return Err(bad("the upload link is not a PUT"));
    }
    let mut headers = HeaderMap::new();
    let given = body
        .get("headers")
        .and_then(Value::as_object)
        .ok_or_else(|| bad("`headers` is not an object"))?;
    for (name, value) in given {
        let value = value.as_str().ok_or_else(|| bad("a header value is not a string"))?;
        let name = HeaderName::from_bytes(name.as_bytes()).map_err(|_| bad("a header name is not valid"))?;
        let value = HeaderValue::from_str(value).map_err(|_| bad("a header value is not valid"))?;
        headers.insert(name, value);
    }
    Ok(UploadLink {
        url: text(body, "url")?.to_string(),
        headers,
    })
}

#[cfg(test)]
#[path = "../../unit-tests/services/cloud_sync_types.rs"]
mod tests;
