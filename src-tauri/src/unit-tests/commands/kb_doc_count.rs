use std::fs;

use crate::commands::read::kb_doc_count;

fn manifest_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn kb_doc_count_registered_in_lib_rs() {
    let lib = fs::read_to_string(manifest_dir().join("src/lib.rs")).expect("lib.rs");
    assert!(
        lib.contains("commands::read::kb_doc_count"),
        "lib.rs invoke handler must register commands::read::kb_doc_count"
    );
    assert!(
        lib.contains("commands::read::get_kb_hide_patterns"),
        "lib.rs invoke handler must register commands::read::get_kb_hide_patterns"
    );
}

#[test]
fn kb_doc_count_command_is_exported() {
    let _ = kb_doc_count as fn(
        tauri::AppHandle,
        String,
        Option<String>,
    ) -> Result<serde_json::Value, String>;
}
