use std::fs;
use std::path::PathBuf;

use serde_json::json;

use crate::services::docs::{create_doc, create_doc_content, update_doc};
use crate::services::spark_read::get_doc_asset;
use crate::test_support::TestSandbox;

const SAMPLE_DOC: &str = r#"# Test Title

> 创建时间：2026年6月19日 14:30

---

摘要正文，enough content.
"#;

const PNG: &[u8] = b"\x89PNG\r\n";

fn setup_notes() -> TestSandbox {
    let sandbox = TestSandbox::new();
    let notes = crate::test_support::default_workspace_dir();
    fs::create_dir_all(notes.join("raw")).expect("raw dir");
    fs::create_dir_all(notes.join("digest")).expect("digest dir");
    sandbox
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

fn create_with_assets(sandbox: &TestSandbox, assets: Vec<PathBuf>) -> serde_json::Value {
    let source = stage_md(sandbox);
    let paths: Vec<String> = assets
        .iter()
        .map(|p| p.to_str().unwrap().to_string())
        .collect();
    create_doc(&json!({
            "workspace_id": crate::test_support::default_workspace().id,
            "source_path": source.to_str().unwrap(),
            "title": "Test Title",
            "digest": "never",
            "source_type": "summary",
            "asset_paths": paths,
        }),
    )
}

fn theme_dir() -> PathBuf {
    crate::test_support::default_workspace_dir()
        .join("raw")
        .join(crate::test_support::default_inbox_id())
        .join("notes")
}

#[test]
fn create_note_copies_png_into_attached() {
    let sandbox = setup_notes();
    let png = stage_bytes(&sandbox, "overview-diagram.png", PNG);
    let v = create_with_assets(&sandbox, vec![png]);
    assert_eq!(v.get("ok"), Some(&json!(true)), "{v}");
    let id = v["id"].as_str().unwrap();
    let rel = format!(".attached/{id}/overview-diagram.png");
    assert_eq!(v["asset_paths"], json!([rel]));
    let dest = theme_dir().join(".attached").join(id).join("overview-diagram.png");
    assert_eq!(fs::read(&dest).unwrap(), PNG);
    let loaded = get_doc_asset("raw", id, &rel, None);
    assert_eq!(loaded["mime_type"], "image/png");
    assert!(loaded.get("data_b64").and_then(|x| x.as_str()).is_some());
}

#[test]
fn create_note_accepts_jpg_and_jpeg() {
    let sandbox = setup_notes();
    let jpg = stage_bytes(&sandbox, "a.jpg", b"jpeg");
    let jpeg = stage_bytes(&sandbox, "b.jpeg", b"jpeg");
    let v = create_with_assets(&sandbox, vec![jpg, jpeg]);
    assert_eq!(v.get("ok"), Some(&json!(true)), "{v}");
    let id = v["id"].as_str().unwrap();
    assert_eq!(
        v["asset_paths"],
        json!([
            format!(".attached/{id}/a.jpg"),
            format!(".attached/{id}/b.jpeg")
        ])
    );
}

#[test]
fn create_note_rejects_svg_and_gif() {
    let sandbox = setup_notes();
    let svg = create_with_assets(
        &sandbox,
        vec![stage_bytes(&sandbox, "x.svg", b"<svg")],
    );
    assert_eq!(svg.get("_status"), Some(&json!(400)));
    assert_eq!(svg["error"], "Unsupported media type");
    let gif = create_with_assets(
        &sandbox,
        vec![stage_bytes(&sandbox, "x.gif", b"GIF89")],
    );
    assert_eq!(gif.get("_status"), Some(&json!(400)));
    assert!(theme_dir().join("x.svg").exists() == false);
    assert!(!theme_dir().exists() || theme_dir().read_dir().unwrap().count() == 0);
}

#[test]
fn create_note_rejects_too_many_or_too_large() {
    let sandbox = setup_notes();
    let many: Vec<PathBuf> = (0..9)
        .map(|i| stage_bytes(&sandbox, &format!("n{i}.png"), PNG))
        .collect();
    let over = create_with_assets(&sandbox, many);
    assert_eq!(over.get("_status"), Some(&json!(400)));
    assert!(
        over["error"].as_str().unwrap_or("").contains("max"),
        "{over}"
    );
    let huge = vec![0u8; 4 * 1024 * 1024 + 1];
    let big = create_with_assets(
        &sandbox,
        vec![stage_bytes(&sandbox, "big.png", &huge)],
    );
    assert_eq!(big.get("_status"), Some(&json!(413)));
    assert_eq!(big["error"], "Asset file too large");
}

#[test]
fn create_note_same_basename_in_two_docs_is_ok() {
    let sandbox = setup_notes();
    let first = create_with_assets(
        &sandbox,
        vec![stage_bytes(&sandbox, "overview-diagram.png", PNG)],
    );
    assert_eq!(first.get("ok"), Some(&json!(true)), "{first}");
    let second = create_with_assets(
        &sandbox,
        vec![stage_bytes(&sandbox, "overview-diagram.png", PNG)],
    );
    assert_eq!(second.get("ok"), Some(&json!(true)), "{second}");
    let a = first["id"].as_str().unwrap();
    let b = second["id"].as_str().unwrap();
    assert_ne!(a, b);
    let dir = theme_dir();
    assert!(dir.join(".attached").join(a).join("overview-diagram.png").is_file());
    assert!(dir.join(".attached").join(b).join("overview-diagram.png").is_file());
}

#[test]
fn create_note_duplicate_basenames_are_409() {
    let sandbox = setup_notes();
    let a = stage_bytes(&sandbox, "dup.png", PNG);
    let nested = sandbox.cache_dir().join("archive_source_stage").join("sub");
    fs::create_dir_all(&nested).unwrap();
    let b = nested.join("dup.png");
    fs::write(&b, PNG).unwrap();
    let v = create_with_assets(&sandbox, vec![a, b.canonicalize().unwrap()]);
    assert_eq!(v.get("_status"), Some(&json!(409)), "{v}");
    assert!(!theme_dir().exists() || !theme_dir().join("dup.png").exists());
}

#[test]
fn create_note_digest_relative_image_is_400() {
    let sandbox = setup_notes();
    let source = stage_md(&sandbox);
    let png = stage_bytes(&sandbox, "overview-diagram.png", PNG);
    let v = create_doc(&json!({
            "source_path": source.to_str().unwrap(),
            "title": "Test Title",
            "digest": "always",
            "source_type": "summary",
            "digest_body": "# 摘要\n\n![x](./overview-diagram.png)\n",
            "asset_paths": [png.to_str().unwrap()],
        }),
    );
    assert_eq!(v.get("_status"), Some(&json!(400)), "{v}");
    assert!(
        v["error"].as_str().unwrap_or("").contains("relative images"),
        "{v}"
    );
    assert!(!theme_dir().exists() || theme_dir().read_dir().unwrap().count() == 0);
}

#[test]
fn create_note_digest_allows_https_image() {
    let sandbox = setup_notes();
    let source = stage_md(&sandbox);
    let digest_body = "# Test — 摘要\n\n> 创建时间：2026年6月19日 14:30\n\n## 概述\n\n![x](https://example.com/a.png)";
    let v = create_doc(&json!({
            "workspace_id": crate::test_support::default_workspace().id,
            "source_path": source.to_str().unwrap(),
            "title": "Test Title",
            "digest": "always",
            "source_type": "summary",
            "digest_body": digest_body,
        }),
    );
    assert_eq!(v.get("ok"), Some(&json!(true)), "{v}");
}

#[test]
fn create_note_content_and_update_reject_asset_paths() {
    let sandbox = setup_notes();
    let content = create_doc_content(&json!({
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
    let created = create_doc_content(&json!({
            "workspace_id": crate::test_support::default_workspace().id,
            "content": SAMPLE_DOC,
            "title": "Test Title",
            "digest": "never",
            "source_type": "summary",
        }),
    );
    let id = created["id"].as_str().unwrap();
    let path = stage_md(&sandbox);
    let updated = update_doc(&json!({
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

#[test]
fn create_note_rewrites_declared_refs_and_lists_unmatched() {
    let sandbox = setup_notes();
    let png = stage_bytes(&sandbox, "overview-diagram.png", PNG);
    let source = stage_bytes(
        &sandbox,
        "source.md",
        format!("{SAMPLE_DOC}\n\n![x](overview-diagram.png)\n![y](gone.png)\n").as_bytes(),
    );
    let v = create_doc(&json!({
            "workspace_id": crate::test_support::default_workspace().id,
            "source_path": source.to_str().unwrap(),
            "title": "Test Title",
            "digest": "never",
            "source_type": "summary",
            "asset_paths": [png.to_str().unwrap()],
            "translations": [{
                "lang": "zh",
                "content": "# 标题\n\n![图](overview-diagram.png)\n"
            }],
        }),
    );
    assert_eq!(v.get("ok"), Some(&json!(true)), "{v}");
    assert_eq!(v["unmatched_image_refs"], json!(["gone.png"]));
    let id = v["id"].as_str().unwrap();
    let href = format!(".attached/{id}/overview-diagram.png");
    let raw = theme_dir().read_dir().unwrap().find_map(|e| {
        let p = e.ok()?.path();
        (p.extension()?.to_str()? == "md" && !p.file_name()?.to_str()?.contains("-zh")).then_some(p)
    }).expect("raw");
    let zh = raw.with_file_name(format!("{}-zh.md", raw.file_stem().unwrap().to_str().unwrap()));
    let raw_body = fs::read_to_string(&raw).unwrap();
    let zh_body = fs::read_to_string(&zh).unwrap();
    assert!(raw_body.contains(&format!("![x]({href})")), "{raw_body}");
    assert!(raw_body.contains("![y](gone.png)"), "{raw_body}");
    assert!(zh_body.contains(&format!("![图]({href})")), "{zh_body}");
}
