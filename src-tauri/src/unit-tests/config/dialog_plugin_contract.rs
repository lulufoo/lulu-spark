//! Host dialog plugin contract (T7): dependency, registration, capabilities, `.md` filter.

use std::fs;
use std::path::PathBuf;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_repo_file(rel: &str) -> String {
    fs::read_to_string(manifest_dir().join(rel)).unwrap_or_else(|e| panic!("read {rel}: {e}"))
}

#[test]
fn cargo_toml_declares_tauri_plugin_dialog() {
    let cargo = read_repo_file("Cargo.toml");
    assert!(
        cargo.contains("tauri-plugin-dialog"),
        "Cargo.toml must declare tauri-plugin-dialog (Host file picker)"
    );
}

#[test]
fn lib_rs_registers_dialog_plugin() {
    let lib_rs = read_repo_file("src/lib.rs");
    assert!(
        lib_rs.contains("tauri_plugin_dialog::init()"),
        "lib.rs must register tauri_plugin_dialog::init()"
    );
}

#[test]
fn default_capability_grants_dialog_permission() {
    let raw = read_repo_file("capabilities/default.json");
    let v: serde_json::Value = serde_json::from_str(&raw).expect("default.json");
    let perms = v["permissions"]
        .as_array()
        .expect("permissions array")
        .iter()
        .filter_map(|p| p.as_str())
        .collect::<Vec<_>>();
    let has_dialog = perms.iter().any(|p| p.starts_with("dialog:"));
    assert!(
        has_dialog,
        "capabilities/default.json must grant a dialog:* permission; got {perms:?}"
    );
}

#[test]
fn plan_attachment_dialog_filter_is_markdown_only() {
    let lib_rs = read_repo_file("src/lib.rs");
    assert!(
        lib_rs.contains("PLAN_ATTACHMENT_DIALOG_EXTENSIONS"),
        "lib.rs must expose PLAN_ATTACHMENT_DIALOG_EXTENSIONS for Host `.md` filter contract"
    );
    // Exact allow-list: only "md" (no other extensions in the slice).
    let re = regex::Regex::new(
        r#"PLAN_ATTACHMENT_DIALOG_EXTENSIONS\s*:\s*&\[&str\]\s*=\s*&\s*\[\s*"md"\s*\]"#,
    )
    .expect("regex");
    assert!(
        re.is_match(&lib_rs),
        "PLAN_ATTACHMENT_DIALOG_EXTENSIONS must be &[\"md\"] only"
    );
}

#[test]
fn host_dialog_surface_has_no_attachment_business() {
    // Dialog enablement must not embed copy/associate attachment business on the Host surface.
    let cargo = read_repo_file("Cargo.toml");
    let caps = read_repo_file("capabilities/default.json");
    let lib_rs = read_repo_file("src/lib.rs");

    for (label, text) in [
        ("Cargo.toml", cargo.as_str()),
        ("capabilities/default.json", caps.as_str()),
    ] {
        assert!(
            !text.contains("attachments.json"),
            "{label} must not carry attachment business metadata"
        );
        assert!(
            !text.contains("add_attachment"),
            "{label} must not carry RS attachment business entrypoints"
        );
    }

    // Plugin registration line must stay independent of attachment invoke handlers.
    let plugin_line = lib_rs
        .lines()
        .find(|l| l.contains("tauri_plugin_dialog::init()"))
        .expect("dialog plugin registration line");
    assert!(
        !plugin_line.contains("add_plan_attachment") && !plugin_line.contains("attachments"),
        "dialog plugin init must not embed attachment business: {plugin_line}"
    );
}
