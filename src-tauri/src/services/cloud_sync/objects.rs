//! The bytes of one document in the cloud: written through a signed link, read with the storage
//! token. Neither passes through the Worker.

use std::io::Read;
use std::time::Duration;

use reqwest::header::CONTENT_TYPE;
use serde_json::Value;

use super::client::{network, CloudSync};
use super::error::CloudSyncError;
use super::types::check_document_id;

/// The longest a transfer may take. An upload link lives 10 minutes and Google refuses an upload
/// that ends after that, so waiting longer for the bytes is no use.
const TRANSFER_TIMEOUT: Duration = Duration::from_secs(10 * 60);
/// The biggest answer read from Google: as much as `doc_zip` unpacks. The Worker holds the
/// per-document limit; this only keeps one wrong object from filling memory.
const MAX_DOWNLOAD_BYTES: u64 = 100 * 1024 * 1024;
/// Error bodies are short. Longer ones are cut.
const MAX_ERROR_BODY_BYTES: u64 = 16 * 1024;

fn read_capped(reader: impl Read, cap: u64) -> Result<Vec<u8>, CloudSyncError> {
    let mut bytes = Vec::new();
    reader
        .take(cap + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| CloudSyncError::Network(e.to_string()))?;
    if bytes.len() as u64 > cap {
        return Err(CloudSyncError::BadResponse(format!("the object is larger than {cap} bytes")));
    }
    Ok(bytes)
}

/// The error name in a Google answer: `error.errors[0].reason` of the JSON API, or `<Code>` of the
/// XML API. Only a plain name is passed on.
fn storage_code(body: &[u8]) -> String {
    let text = String::from_utf8_lossy(body);
    let json_reason = serde_json::from_str::<Value>(&text)
        .ok()
        .and_then(|v| v.pointer("/error/errors/0/reason")?.as_str().map(str::to_string));
    let xml_code = || {
        let rest = text.split_once("<Code>")?.1;
        Some(rest.split_once("</Code>")?.0.to_string())
    };
    json_reason
        .or_else(xml_code)
        .filter(|c| !c.is_empty() && c.len() <= 64 && c.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_'))
        .unwrap_or_else(|| "unknown".to_string())
}

/// What Google answered, for the cases every call treats alike.
fn storage_failure(status: u16, body: &[u8]) -> CloudSyncError {
    let code = storage_code(body);
    match status {
        429 | 500..=599 => CloudSyncError::Unavailable { status, code },
        400..=499 => CloudSyncError::Rejected { status, code },
        _ => CloudSyncError::BadResponse(format!("unexpected status {status}")),
    }
}

fn error_body(response: reqwest::blocking::Response) -> Vec<u8> {
    read_capped(response, MAX_ERROR_BODY_BYTES).unwrap_or_default()
}

impl CloudSync {
    /// Writes the package as the document's one object and stamps `version` on it.
    ///
    /// Every call asks the Worker for a new link; a link is never used twice. A link Google
    /// refuses as expired is replaced once; any other refusal is returned as it is.
    pub fn upload(&self, document_id: &str, package: &[u8], version: i64) -> Result<(), CloudSyncError> {
        check_document_id(document_id)?;
        if package.is_empty() {
            return Err(CloudSyncError::InvalidInput("the package is empty".into()));
        }
        if version < 0 {
            return Err(CloudSyncError::InvalidInput("the version is negative".into()));
        }
        let first = self.upload_once(document_id, package, version);
        match first {
            Err(CloudSyncError::UploadLinkExpired) => self.upload_once(document_id, package, version),
            other => other,
        }
    }

    fn upload_once(&self, document_id: &str, package: &[u8], version: i64) -> Result<(), CloudSyncError> {
        let link = self.upload_link(document_id, package.len(), version)?;
        // The bytes go only to Google. A link that points anywhere else is not one the Worker made.
        if !link.url.starts_with(&format!("{}/", self.storage_url)) {
            return Err(CloudSyncError::BadResponse("the upload link points outside Google Cloud Storage".into()));
        }
        let response = self
            .http()?
            .put(&link.url)
            .timeout(TRANSFER_TIMEOUT)
            .headers(link.headers)
            .header(CONTENT_TYPE, "application/zip")
            .body(package.to_vec())
            .send()
            .map_err(network)?;
        let status = response.status();
        if status.is_success() {
            return Ok(());
        }
        let body = error_body(response);
        match status.as_u16() {
            412 => Err(CloudSyncError::ObjectChanged),
            400 if storage_code(&body) == "ExpiredToken" => Err(CloudSyncError::UploadLinkExpired),
            other => Err(storage_failure(other, &body)),
        }
    }

    /// The document's package as stored in the cloud, read with the storage token. A token Google
    /// refuses is replaced once.
    pub fn read(&self, document_id: &str) -> Result<Vec<u8>, CloudSyncError> {
        check_document_id(document_id)?;
        match self.read_once(document_id) {
            Err(CloudSyncError::StorageTokenRejected) => {
                self.forget_credentials();
                self.read_once(document_id)
            }
            other => other,
        }
    }

    fn read_once(&self, document_id: &str) -> Result<Vec<u8>, CloudSyncError> {
        let credentials = self.credentials()?;
        let url = format!(
            "{}/storage/v1/b/{}/o/{document_id}?alt=media",
            self.storage_url, credentials.bucket
        );
        let response = self
            .http()?
            .get(url)
            .timeout(TRANSFER_TIMEOUT)
            .bearer_auth(&credentials.access_token)
            .send()
            .map_err(network)?;
        let status = response.status();
        if status.is_success() {
            return read_capped(response, MAX_DOWNLOAD_BYTES);
        }
        match status.as_u16() {
            401 => Err(CloudSyncError::StorageTokenRejected),
            404 => Err(CloudSyncError::ObjectNotFound),
            other => Err(storage_failure(other, &error_body(response))),
        }
    }
}

#[cfg(test)]
#[path = "../../unit-tests/services/cloud_sync_objects.rs"]
mod tests;
