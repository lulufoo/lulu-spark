use std::fs;

use serde_json::{json, Value};

use crate::config::paths;

use super::{http_post_with_response, setup_repo_for_read_later, with_server};

fn channel_records(channel: &str) -> Vec<Value> {
    let Ok(path) = paths::message_center_path() else {
        return Vec::new();
    };
    if !path.is_file() {
        return Vec::new();
    }
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_str::<Value>(&text) else {
        return Vec::new();
    };
    value
        .get("records")
        .and_then(|v| v.as_array())
        .map(|records| {
            records
                .iter()
                .filter(|record| record.get("channel").and_then(|c| c.as_str()) == Some(channel))
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

fn occupy_message_center_path_as_dir() {
    let path = paths::message_center_path().expect("message_center_path");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("persist parent");
    }
    fs::create_dir_all(&path).expect("occupy persist path as directory");
}

fn post_read_later(port: u16, url: &str, title: &str) -> (u16, Value) {
    http_post_with_response(
        port,
        "/api/read-later",
        &json!({ "url": url, "title": title }),
    )
}

/// Normal: Main Host `handle_read_later_post` 201 writes a `read_later` record.
#[test]
fn handle_read_later_post_201_records_read_later_channel() {
    let fixture = setup_repo_for_read_later();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let before = channel_records("read_later").len();
        let (status, body) = post_read_later(port, "https://example.com/ac1-produce", "AC1");
        assert_eq!(status, 201);
        assert!(body.get("error").is_none(), "{body}");
        let records = channel_records("read_later");
        assert_eq!(records.len(), before + 1);
        assert_eq!(
            records.last().and_then(|r| r.get("channel")),
            Some(&json!("read_later"))
        );
    });
}

/// Boundary: failed read-later POST does not produce.
#[test]
fn handle_read_later_post_400_does_not_produce() {
    let fixture = setup_repo_for_read_later();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let before = channel_records("read_later").len();
        let (status, body) =
            http_post_with_response(port, "/api/read-later", &json!({ "title": "no url" }));
        assert_eq!(status, 400);
        assert!(body.get("error").is_some(), "{body}");
        assert_eq!(channel_records("read_later").len(), before);
    });
}

/// Boundary: read-later 409 does not produce a second record.
#[test]
fn handle_read_later_post_409_does_not_produce() {
    let fixture = setup_repo_for_read_later();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let payload = json!({
            "url": "https://example.com/ac1-duplicate",
            "title": "Dup"
        });
        let (first_status, _) = http_post_with_response(port, "/api/read-later", &payload);
        assert_eq!(first_status, 201);
        assert_eq!(channel_records("read_later").len(), 1);

        let (status, body) = http_post_with_response(port, "/api/read-later", &payload);
        assert_eq!(status, 409);
        assert_eq!(body["code"], "read_later_recent_duplicate");
        assert_eq!(channel_records("read_later").len(), 1);
    });
}

/// Exception: message-center failure does not change a successful HTTP 201.
#[test]
fn message_center_failure_keeps_successful_http_201() {
    let fixture = setup_repo_for_read_later();
    let repo_root = fixture.repo_root.clone();
    occupy_message_center_path_as_dir();
    with_server(repo_root, |port| {
        let url = "https://example.com/ac1-keep-201";
        let (status, body) = post_read_later(port, url, "Keep");
        assert_eq!(status, 201, "{body}");
        assert!(body.get("error").is_none(), "{body}");
        assert_eq!(body["entry"]["url"], url);
        assert!(channel_records("read_later").is_empty());
    });
}
