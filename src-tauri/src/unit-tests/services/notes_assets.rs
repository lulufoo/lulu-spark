use std::fs;
use std::path::PathBuf;

use serde_json::json;

use crate::services::notes::{create_note, create_note_content, update_note};
use crate::services::workbench_read::get_notes_asset;
use crate::test_support::TestSandbox;

const SAMPLE_DOC: &str = r#"# Test Title

> 创建时间：2026年6月19日 14:30

---

摘要正文，enough content.
"#;

const PNG: &[u8] = b"\x89PNG\r\n";

fn setup_notes() -> (TestSandbox, PathBuf) {
    let sandbox = TestSandbox::new();
    let notes = sandbox.workbench_root().join("notes");
    fs::create_dir_all(notes.join("raw")).expect("raw dir");
    fs::create_dir_all(notes.join("digest")).expect("digest dir");
    fs::write(notes.join("index.json"), br#"{"entries":{}}"#).expect("index");
    let repo_root = sandbox.config_dir().to_path_buf();
    (sandbox, repo_root)
}

fn stage_md(sandbox: &TestSandbox) -> PathBuf {
    stage_bytes(sandbox, "source.md", SAMPLE_DOC.as_bytes())
}

fn stage_bytes(sandbox: &TestSandbox, name: &str, bytes: &[u8]) -> PathBuf {
    let dir = sandbox.cache_dir().join("archive_source_stage");
    fs::create_dir_all(&dir).expect("stage dir");
    let p = dir.join(name);
    fs::write(&p, bytes).expect("write stage");
    p.canonicalize().expect("canon")
}

fn create_with_assets(sandbox: &TestSandbox, repo_root: &std::path::Path, assets: Vec<PathBuf>) -> serde_json::Value {
    let source = stage_md(sandbox);
    let paths: Vec<String> = assets
        .iter()
        .map(|p| p.to_str().unwrap().to_string())
        .collect();
    create_note(
        repo_root,
        &json!({
            "source_path": source.to_str().unwrap(),
            "title": "Test Title",
            "digest": "never",
            "asset_paths": paths,
        }),
    )
}

fn theme_dir(repo_root: &std::path::Path) -> PathBuf {
    crate::config::roots::notes_root_path(repo_root)
        .join("raw")
        .join("inbox")
        .join("notes")
}

#[test]
fn create_note_copies_png_beside_raw() {
    let (sandbox, repo_root) = setup_notes();
    let png = stage_bytes(&sandbox, "overview-diagram.png", PNG);
    let v = create_with_assets(&sandbox, &repo_root, vec![png]);
    assert_eq!(v.get("ok"), Some(&json!(true)), "{v}");
    let common_path = v["common_path"].as_str().unwrap();
    assert_eq!(
        v["asset_paths"],
        json!(["raw/inbox/notes/overview-diagram.png"])
    );
    let dest = theme_dir(&repo_root).join("overview-diagram.png");
    assert_eq!(fs::read(&dest).unwrap(), PNG);
    let loaded = get_notes_asset(&repo_root, "raw", common_path, "overview-diagram.png");
    assert_eq!(loaded["mime_type"], "image/png");
    assert!(loaded.get("data_b64").and_then(|x| x.as_str()).is_some());
}

#[test]
fn create_note_accepts_jpg_and_jpeg() {
    let (sandbox, repo_root) = setup_notes();
    let jpg = stage_bytes(&sandbox, "a.jpg", b"jpeg");
    let jpeg = stage_bytes(&sandbox, "b.jpeg", b"jpeg");
    let v = create_with_assets(&sandbox, &repo_root, vec![jpg, jpeg]);
    assert_eq!(v.get("ok"), Some(&json!(true)), "{v}");
    assert_eq!(
        v["asset_paths"],
        json!(["raw/inbox/notes/a.jpg", "raw/inbox/notes/b.jpeg"])
    );
}

#[test]
fn create_note_rejects_svg_and_gif() {
    let (sandbox, repo_root) = setup_notes();
    let svg = create_with_assets(
        &sandbox,
        &repo_root,
        vec![stage_bytes(&sandbox, "x.svg", b"<svg")],
    );
    assert_eq!(svg.get("_status"), Some(&json!(400)));
    assert_eq!(svg["error"], "Unsupported media type");
    let gif = create_with_assets(
        &sandbox,
        &repo_root,
        vec![stage_bytes(&sandbox, "x.gif", b"GIF89")],
    );
    assert_eq!(gif.get("_status"), Some(&json!(400)));
    assert!(theme_dir(&repo_root).join("x.svg").exists() == false);
    assert!(!theme_dir(&repo_root).exists() || theme_dir(&repo_root).read_dir().unwrap().count() == 0);
}

#[test]
fn create_note_rejects_too_many_or_too_large() {
    let (sandbox, repo_root) = setup_notes();
    let many: Vec<PathBuf> = (0..9)
        .map(|i| stage_bytes(&sandbox, &format!("n{i}.png"), PNG))
        .collect();
    let over = create_with_assets(&sandbox, &repo_root, many);
    assert_eq!(over.get("_status"), Some(&json!(400)));
    assert!(
        over["error"].as_str().unwrap_or("").contains("max"),
        "{over}"
    );
    let huge = vec![0u8; 4 * 1024 * 1024 + 1];
    let big = create_with_assets(
        &sandbox,
        &repo_root,
        vec![stage_bytes(&sandbox, "big.png", &huge)],
    );
    assert_eq!(big.get("_status"), Some(&json!(413)));
    assert_eq!(big["error"], "Asset file too large");
}

#[test]
fn create_note_collision_is_409_and_does_not_write() {
    let (sandbox, repo_root) = setup_notes();
    let first = create_with_assets(
        &sandbox,
        &repo_root,
        vec![stage_bytes(&sandbox, "overview-diagram.png", PNG)],
    );
    assert_eq!(first.get("ok"), Some(&json!(true)), "{first}");
    let first_md = first["common_path"].as_str().unwrap().to_string();
    let second = create_with_assets(
        &sandbox,
        &repo_root,
        vec![stage_bytes(&sandbox, "overview-diagram.png", PNG)],
    );
    assert_eq!(second.get("_status"), Some(&json!(409)), "{second}");
    let dir = theme_dir(&repo_root);
    let mds: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("md"))
        .collect();
    assert_eq!(mds.len(), 1);
    assert!(dir.join("overview-diagram.png").is_file());
    let notes = crate::config::roots::notes_root_path(&repo_root);
    assert!(notes.join("raw").join(&first_md).is_file());
}

#[test]
fn create_note_duplicate_basenames_are_409() {
    let (sandbox, repo_root) = setup_notes();
    let a = stage_bytes(&sandbox, "dup.png", PNG);
    let nested = sandbox.cache_dir().join("archive_source_stage").join("sub");
    fs::create_dir_all(&nested).unwrap();
    let b = nested.join("dup.png");
    fs::write(&b, PNG).unwrap();
    let v = create_with_assets(&sandbox, &repo_root, vec![a, b.canonicalize().unwrap()]);
    assert_eq!(v.get("_status"), Some(&json!(409)), "{v}");
    assert!(!theme_dir(&repo_root).exists() || !theme_dir(&repo_root).join("dup.png").exists());
}

#[test]
fn create_note_digest_relative_image_is_400() {
    let (sandbox, repo_root) = setup_notes();
    let source = stage_md(&sandbox);
    let png = stage_bytes(&sandbox, "overview-diagram.png", PNG);
    let v = create_note(
        &repo_root,
        &json!({
            "source_path": source.to_str().unwrap(),
            "title": "Test Title",
            "digest": "always",
            "digest_body": "# 摘要\n\n![x](./overview-diagram.png)\n",
            "asset_paths": [png.to_str().unwrap()],
        }),
    );
    assert_eq!(v.get("_status"), Some(&json!(400)), "{v}");
    assert!(
        v["error"].as_str().unwrap_or("").contains("relative images"),
        "{v}"
    );
    assert!(!theme_dir(&repo_root).exists() || theme_dir(&repo_root).read_dir().unwrap().count() == 0);
}

#[test]
fn create_note_digest_allows_https_image() {
    let (sandbox, repo_root) = setup_notes();
    let source = stage_md(&sandbox);
    let digest_body = "# Test — 摘要\n\n> 创建时间：2026年6月19日 14:30\n\n## 概述\n\n![x](https://example.com/a.png)";
    let v = create_note(
        &repo_root,
        &json!({
            "source_path": source.to_str().unwrap(),
            "title": "Test Title",
            "digest": "always",
            "digest_body": digest_body,
        }),
    );
    assert_eq!(v.get("ok"), Some(&json!(true)), "{v}");
}

#[test]
fn create_note_content_and_update_reject_asset_paths() {
    let (sandbox, repo_root) = setup_notes();
    let content = create_note_content(
        &repo_root,
        &json!({
            "content": SAMPLE_DOC,
            "title": "Test Title",
            "digest": "never",
            "asset_paths": ["/tmp/x.png"],
        }),
    );
    assert_eq!(content.get("_status"), Some(&json!(400)));
    assert!(
        content["error"].as_str().unwrap_or("").contains("asset_paths"),
        "{content}"
    );
    let created = create_note_content(
        &repo_root,
        &json!({
            "content": SAMPLE_DOC,
            "title": "Test Title",
            "digest": "never",
        }),
    );
    let id = created["id"].as_str().unwrap();
    let path = stage_md(&sandbox);
    let updated = update_note(
        &repo_root,
        &json!({
            "id": id,
            "source_path": path.to_str().unwrap(),
            "digest": "never",
            "asset_paths": [path.to_str().unwrap()],
        }),
    );
    assert_eq!(updated.get("_status"), Some(&json!(400)));
    assert!(
        updated["error"].as_str().unwrap_or("").contains("asset_paths"),
        "{updated}"
    );
}
