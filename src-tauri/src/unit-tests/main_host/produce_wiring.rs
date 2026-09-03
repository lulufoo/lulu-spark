use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::commands::read_later::{
    create_read_later_json, delete_read_later_json, get_read_later_json, mark_read_later_json,
};
use crate::config::paths;
use crate::services::read_later;

use super::{
    http_get, http_post_with_response, setup_repo_for_read_later, with_server,
};

fn channel_record_count(channel: &str) -> usize {
    let Ok(path) = paths::message_center_path() else {
        return 0;
    };
    if !path.is_file() {
        return 0;
    }
    let Ok(text) = fs::read_to_string(path) else {
        return 0;
    };
    let Ok(value) = serde_json::from_str::<Value>(&text) else {
        return 0;
    };
    value
        .get("records")
        .and_then(|v| v.as_array())
        .map(|records| {
            records
                .iter()
                .filter(|record| record.get("channel").and_then(|c| c.as_str()) == Some(channel))
                .count()
        })
        .unwrap_or(0)
}

fn occupy_message_center_path_as_dir() {
    let path = paths::message_center_path().expect("message_center_path");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("persist parent");
    }
    fs::create_dir_all(&path).expect("occupy persist path as directory");
}

fn src(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src").join(rel)
}

fn repo_file(rel: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.push(rel);
    path
}

fn assert_file_has_no_message_center(path: &Path) {
    let text = fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    assert!(
        !text.contains("message_center") && !text.contains("::produce("),
        "{} must not wire message center",
        path.display()
    );
}

fn post_read_later(port: u16, url: &str, title: &str) -> (u16, Value) {
    http_post_with_response(
        port,
        "/api/read-later",
        &json!({ "url": url, "title": title }),
    )
}

#[test]
fn handle_read_later_post_201_adds_read_later_record() {
    let fixture = setup_repo_for_read_later();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let before = channel_record_count("read_later");
        let (status, body) = post_read_later(port, "https://example.com/produce", "Produce");
        assert_eq!(status, 201);
        assert!(body.get("error").is_none(), "{body}");
        assert!(body["entry"].is_object(), "{body}");
        assert_eq!(channel_record_count("read_later"), before + 1);
    });
}

#[test]
fn chrome_and_l0_inbound_still_only_url_and_title() {
    let chrome = fs::read_to_string(repo_file(
        "extensions/chrome-read-later/lib/readLaterApi.js",
    ))
    .expect("chrome readLaterApi");
    assert!(
        chrome.contains("JSON.stringify({ url, title })"),
        "Chrome inbound must still POST only url/title"
    );

    let host = fs::read_to_string(src("main_host/read_later.rs")).expect("main host handler");
    assert!(host.contains("payload.get(\"url\")"));
    assert!(host.contains("payload.get(\"title\")"));
    assert_eq!(
        host.matches("payload.get(").count(),
        2,
        "Main Host must still only read url/title from inbound JSON"
    );

    let gateway = fs::read_to_string(src("gateway/mod.rs")).expect("gateway");
    assert!(gateway.contains(".route(\"/read-later\", any(read_later_named))"));
    assert!(gateway.contains("\"/api/read-later\""));
    assert!(
        gateway.contains("method != Method::POST && method != Method::OPTIONS"),
        "L0 /read-later must still only forward POST/OPTIONS"
    );

    let fixture = setup_repo_for_read_later();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (status, body) = http_post_with_response(
            port,
            "/api/read-later",
            &json!({
                "url": "https://example.com/extra-field",
                "title": "Extra",
                "channel": "must-be-ignored"
            }),
        );
        assert_eq!(status, 201, "{body}");
        assert_eq!(body["entry"]["url"], "https://example.com/extra-field");
        assert_eq!(body["entry"]["title"], "Extra");
        assert!(body["entry"].get("channel").is_none());
    });
}

#[test]
fn get_mark_read_delete_and_tauri_create_are_not_wired() {
    assert_file_has_no_message_center(&src("commands/read_later.rs"));
    assert_file_has_no_message_center(&src("services/read_later/mod.rs"));

    let dispatch = fs::read_to_string(src("main_host/dispatch.rs")).expect("dispatch");
    assert!(
        !dispatch.contains("message_center") && !dispatch.contains("::produce("),
        "GET/dispatch must not wire message center"
    );

    let fixture = setup_repo_for_read_later();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (status, body) = post_read_later(port, "https://example.com/host-only", "Host");
        assert_eq!(status, 201, "{body}");
        let after_post = channel_record_count("read_later");
        assert_eq!(after_post, 1);

        let (get_status, _) = http_get(port, "/api/read-later");
        assert_ne!(get_status, 201);
        assert_eq!(channel_record_count("read_later"), after_post);

        let tauri = create_read_later_json("https://example.com/tauri-create", Some("Tauri"))
            .expect("tauri create");
        assert!(tauri.get("entry").is_some(), "{tauri}");
        assert_eq!(channel_record_count("read_later"), after_post);

        let listed = get_read_later_json().expect("list");
        assert!(listed.as_array().is_some(), "{listed}");
        assert_eq!(channel_record_count("read_later"), after_post);

        let created = read_later::create_entry("https://example.com/mark-delete", Some("ops"));
        assert_eq!(created["_status"], 201);
        let id = created["entry"]["id"].as_str().expect("id").to_string();
        let after_service = channel_record_count("read_later");

        let marked = mark_read_later_json(&id, true).expect("mark");
        assert_eq!(marked["entry"]["read"], true);
        assert_eq!(channel_record_count("read_later"), after_service);

        let deleted = delete_read_later_json(&id).expect("delete");
        assert_eq!(deleted["entry"]["id"], id);
        assert_eq!(channel_record_count("read_later"), after_service);
    });
}

#[test]
fn recent_duplicate_409_does_not_produce() {
    let fixture = setup_repo_for_read_later();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let payload = json!({
            "url": "https://example.com/duplicate-produce",
            "title": "Dup"
        });
        let (first_status, _) = http_post_with_response(port, "/api/read-later", &payload);
        assert_eq!(first_status, 201);
        assert_eq!(channel_record_count("read_later"), 1);

        let (status, body) = http_post_with_response(port, "/api/read-later", &payload);
        assert_eq!(status, 409);
        assert_eq!(body["code"], "read_later_recent_duplicate");
        assert_eq!(channel_record_count("read_later"), 1);
    });
}

#[test]
fn message_center_failure_does_not_change_successful_http_201() {
    let fixture = setup_repo_for_read_later();
    let repo_root = fixture.repo_root.clone();
    occupy_message_center_path_as_dir();
    with_server(repo_root, |port| {
        let url = "https://example.com/keep-201";
        let (status, body) = post_read_later(port, url, "Keep");
        assert_eq!(status, 201, "{body}");
        assert!(body.get("error").is_none(), "{body}");
        assert_eq!(body["entry"]["url"], url);
        assert_eq!(channel_record_count("read_later"), 0);

        let (again_status, again) = post_read_later(port, url, "Keep");
        assert_eq!(again_status, 409, "{again}");
        assert_eq!(again["code"], "read_later_recent_duplicate");
        assert_eq!(channel_record_count("read_later"), 0);
    });
}
