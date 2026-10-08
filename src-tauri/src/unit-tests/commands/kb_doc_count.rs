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
    assert!(
        lib.contains("commands::kb_viewer_state::get_kb_viewer_state"),
        "lib.rs invoke handler must register commands::kb_viewer_state::get_kb_viewer_state"
    );
    assert!(
        lib.contains("commands::kb_viewer_state::set_kb_viewer_state"),
        "lib.rs invoke handler must register commands::kb_viewer_state::set_kb_viewer_state"
    );
    assert!(
        lib.contains("commands::write::kb_rename"),
        "lib.rs invoke handler must register commands::write::kb_rename"
    );
    assert!(
        lib.contains("commands::kb_entry::kb_create"),
        "lib.rs invoke handler must register commands::kb_entry::kb_create"
    );
    assert!(
        lib.contains("commands::kb_entry::kb_delete"),
        "lib.rs invoke handler must register commands::kb_entry::kb_delete"
    );
    assert!(
        lib.contains("commands::kb_entry::remember_knowledge_doc"),
        "lib.rs invoke handler must register commands::kb_entry::remember_knowledge_doc"
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
