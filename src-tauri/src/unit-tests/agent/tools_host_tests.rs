use std::fs;
use std::path::PathBuf;

use serde_json::json;

use crate::agent::tools::host;
use crate::agent::tools::ToolCatalog;
use crate::services::path_fence::PathFence;

fn tool_param_description(catalog: &ToolCatalog, name: &str, field: &str) -> String {
    catalog
        .definitions
        .iter()
        .find_map(|def| {
            let found = def.pointer("/function/name")?.as_str()?;
            if found == name {
                def.pointer(&format!("/function/parameters/properties/{field}/description"))?
                    .as_str()
                    .map(str::to_string)
            } else {
                None
            }
        })
        .unwrap_or_else(|| panic!("missing {name}.{field}"))
}

fn tool_description(catalog: &ToolCatalog, name: &str) -> String {
    catalog
        .definitions
        .iter()
        .find_map(|def| {
            let found = def.pointer("/function/name")?.as_str()?;
            if found == name {
                def.pointer("/function/description")?
                    .as_str()
                    .map(str::to_string)
            } else {
                None
            }
        })
        .unwrap_or_else(|| panic!("missing {name}"))
}

fn unique_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "wb-fs-tools-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos(),
        label
    ));
    fs::create_dir_all(&dir).expect("create unique dir");
    dir
}

fn live_fence() -> (PathFence, PathBuf, PathBuf) {
    let read_root = unique_dir("read");
    let scratch_parent = unique_dir("scratch-parent");
    let fence = PathFence {
        read_allow: vec![read_root.clone()],
        read_deny: vec![read_root.join(".git")],
        write_allow: vec![],
        write_deny: vec![],
        scratch_parent: Some(scratch_parent.clone()),
    }
    .with_session_scratch("sess_fs")
    .expect("scratch");
    (fence, read_root, scratch_parent.join("sess_fs"))
}

#[test]
fn catalog_exposes_the_five_host_file_tools() {
    let catalog = host::catalog();
    for name in ["grep", "read", "write", "str_replace", "copy"] {
        assert!(catalog.contains(name), "missing {name}");
        assert!(host::is_builtin(name), "{name}");
    }
    for name in ["stage", "list_staged", "get_staged"] {
        assert!(!catalog.contains(name), "stage moved out of host: {name}");
        assert!(!host::is_builtin(name), "{name}");
    }
    assert!(catalog.is_mutating("write"));
    assert!(catalog.is_mutating("str_replace"));
    assert!(catalog.is_mutating("copy"));
    assert!(!catalog.contains("edit"));
    assert!(!host::is_builtin("edit"));
    assert!(!catalog.contains("StrReplace"));
    assert!(!host::is_builtin("StrReplace"));
    assert!(!catalog.is_mutating("read"));
    assert!(!catalog.is_mutating("grep"));
}

#[test]
fn host_file_tool_descriptions_use_dir_names_not_absolute_paths() {
    let catalog = host::catalog();
    for name in ["grep", "read", "write", "str_replace", "copy"] {
        let desc = tool_description(&catalog, name);
        assert!(
            !desc.contains("{session_workspace"),
            "{name} must not inject a scratch placeholder"
        );
        assert!(
            !desc.contains("/agent-workspace/"),
            "{name} static text must not bake a scratch path"
        );
        assert!(
            !desc.contains("list_staged"),
            "{name} must not point at list_staged"
        );
    }
    for name in ["grep", "read"] {
        let desc = tool_description(&catalog, name);
        assert!(
            desc.contains("SESSION_WORKSPACE_DIR"),
            "{name} must name SESSION_WORKSPACE_DIR"
        );
        assert!(
            !desc.contains("SPARK_DATA_DIR"),
            "{name} must not name SPARK_DATA_DIR"
        );
        assert!(
            desc.contains("files on this Chat's Stage"),
            "{name} must say Stage files are readable"
        );
    }
    for name in ["write", "str_replace"] {
        let desc = tool_description(&catalog, name);
        assert!(
            desc.contains("SESSION_WORKSPACE_DIR"),
            "{name} must name SESSION_WORKSPACE_DIR"
        );
        assert!(
            desc.contains("on this Chat's Stage"),
            "{name} must name Stage"
        );
        assert!(
            !desc.contains("SPARK_DATA_DIR"),
            "{name} must not name SPARK_DATA_DIR"
        );
    }
    let grep_desc = tool_description(&catalog, "grep");
    assert!(grep_desc.contains("Readable paths:"));
    assert!(grep_desc.contains("every staged file"));
    assert!(grep_desc.starts_with("Search text."));
    let read_desc = tool_description(&catalog, "read");
    assert!(read_desc.contains("Readable paths:"));
    let write_desc = tool_description(&catalog, "write");
    assert_eq!(
        write_desc,
        "Create or overwrite a text file in SESSION_WORKSPACE_DIR or on this Chat's Stage."
    );
    let replace_desc = tool_description(&catalog, "str_replace");
    assert_eq!(
        replace_desc,
        "Replace exact text in an existing text file in SESSION_WORKSPACE_DIR or on this Chat's Stage."
    );
    let copy_desc = tool_description(&catalog, "copy");
    assert_eq!(
        copy_desc,
        "Copy a regular file. Does not modify the source. Overwrites dest_path if it already exists."
    );
}

#[test]
fn host_file_tool_params_do_not_repeat_dir_names() {
    let catalog = host::catalog();
    assert_eq!(
        tool_param_description(&catalog, "grep", "path"),
        "Optional absolute file or directory."
    );
    assert_eq!(
        tool_param_description(&catalog, "read", "path"),
        "Absolute file path."
    );
    let write_path =
        "Absolute file path. Must be in SESSION_WORKSPACE_DIR or an existing file on Stage.";
    assert_eq!(tool_param_description(&catalog, "write", "path"), write_path);
    assert_eq!(
        tool_param_description(&catalog, "str_replace", "path"),
        write_path
    );
    assert_eq!(
        tool_param_description(&catalog, "str_replace", "old_string"),
        "Text to replace. Must match exactly once unless replace_all is true."
    );
    assert_eq!(
        tool_param_description(&catalog, "copy", "source_path"),
        "Absolute readable file in SESSION_WORKSPACE_DIR or on Stage."
    );
    assert_eq!(
        tool_param_description(&catalog, "copy", "dest_path"),
        "Absolute file path in SESSION_WORKSPACE_DIR or an existing file on Stage. Not a directory."
    );
    for name in ["grep", "read"] {
        let blob = catalog
            .definitions
            .iter()
            .find(|def| def.pointer("/function/name").and_then(|v| v.as_str()) == Some(name))
            .and_then(|def| def.pointer("/function/parameters"))
            .map(|v| v.to_string())
            .expect("parameters");
        assert!(
            !blob.contains("SESSION_WORKSPACE_DIR"),
            "{name} params must not name SESSION_WORKSPACE_DIR"
        );
        assert!(
            !blob.contains("SPARK_DATA_DIR"),
            "{name} params must not name SPARK_DATA_DIR"
        );
    }
}

#[test]
fn read_defaults_to_fifty_lines_and_reports_remaining() {
    let (fence, read_root, _scratch) = live_fence();
    let body = (1..=51)
        .map(|n| format!("line-{n}"))
        .collect::<Vec<_>>()
        .join("\n");
    let path = read_root.join("paged.txt");
    fs::write(&path, body).expect("write");

    let first = host::call("read", &json!({ "path": path.to_string_lossy() }), &fence);
    assert!(!first.is_error, "{}", first.content);
    let first_body: serde_json::Value = serde_json::from_str(&first.content).expect("read json");
    assert_eq!(first_body["offset"], 1);
    assert_eq!(first_body["limit"], 50);
    assert_eq!(first_body["remaining_lines"], 1);
    let first_text = first_body["content"].as_str().expect("content");
    assert!(first_text.starts_with("1:line-1\n"));
    assert!(first_text.ends_with("50:line-50"));
    assert_eq!(
        first_body.as_object().expect("object").keys().count(),
        4
    );

    let last = host::call(
        "read",
        &json!({
            "path": path.to_string_lossy(),
            "offset": 51,
            "limit": 50
        }),
        &fence,
    );
    assert!(!last.is_error, "{}", last.content);
    let last_body: serde_json::Value = serde_json::from_str(&last.content).expect("read json");
    assert_eq!(last_body["offset"], 51);
    assert_eq!(last_body["limit"], 50);
    assert_eq!(last_body["remaining_lines"], 0);
    assert_eq!(last_body["content"], "51:line-51");
    assert_eq!(last_body.as_object().expect("object").keys().count(), 4);

    let past_end = host::call(
        "read",
        &json!({
            "path": path.to_string_lossy(),
            "offset": 52,
            "limit": 50
        }),
        &fence,
    );
    assert!(!past_end.is_error, "{}", past_end.content);
    let past_body: serde_json::Value = serde_json::from_str(&past_end.content).expect("read json");
    assert_eq!(past_body["offset"], 52);
    assert_eq!(past_body["limit"], 50);
    assert_eq!(past_body["remaining_lines"], 0);
    assert_eq!(past_body["content"], "");
}

#[test]
fn grep_and_read_stay_inside_the_read_fence() {
    let (fence, read_root, _scratch) = live_fence();
    fs::write(read_root.join("a.txt"), "alpha\nfind-me\nomega\n").expect("write");
    let grep = host::call(
        "grep",
        &json!({ "pattern": "find-me", "path": read_root.to_string_lossy() }),
        &fence,
    );
    assert!(!grep.is_error, "{}", grep.content);
    assert!(grep.content.contains("find-me"));

    let read = host::call(
        "read",
        &json!({
            "path": read_root.join("a.txt").to_string_lossy(),
            "offset": 2,
            "limit": 1
        }),
        &fence,
    );
    assert!(!read.is_error, "{}", read.content);
    let body: serde_json::Value = serde_json::from_str(&read.content).expect("read json");
    assert_eq!(body["offset"], 2);
    assert_eq!(body["limit"], 1);
    assert_eq!(body["remaining_lines"], 1);
    assert_eq!(body["content"], "2:find-me");

    let denied = host::call("read", &json!({ "path": "/etc/hosts" }), &fence);
    assert!(denied.is_error);
    assert!(denied.content.contains("outside the read fence"));
}

#[test]
fn write_and_edit_are_scratch_only() {
    let (fence, read_root, scratch) = live_fence();
    let outside = read_root.join("todo.md");
    let denied = host::call(
        "write",
        &json!({ "path": outside.to_string_lossy(), "content": "nope" }),
        &fence,
    );
    assert!(denied.is_error);
    assert_eq!(denied.content, "path is outside the write fence");
    assert!(!outside.exists());

    let pad = scratch.join("pad.md");
    let wrote = host::call(
        "write",
        &json!({ "path": pad.to_string_lossy(), "content": "hello world" }),
        &fence,
    );
    assert!(!wrote.is_error, "{}", wrote.content);
    assert_eq!(fs::read_to_string(&pad).expect("read pad"), "hello world");

    let edited = host::call(
        "str_replace",
        &json!({
            "path": pad.to_string_lossy(),
            "old_string": "world",
            "new_string": "scratch"
        }),
        &fence,
    );
    assert!(!edited.is_error, "{}", edited.content);
    assert_eq!(fs::read_to_string(&pad).expect("read pad"), "hello scratch");

    let twice = host::call(
        "str_replace",
        &json!({
            "path": pad.to_string_lossy(),
            "old_string": "hello",
            "new_string": "x"
        }),
        &fence,
    );
    assert!(!twice.is_error);
    let twice_again = host::call(
        "str_replace",
        &json!({
            "path": pad.to_string_lossy(),
            "old_string": "x",
            "new_string": "y"
        }),
        &fence,
    );
    assert!(!twice_again.is_error);
    let dup = host::call(
        "write",
        &json!({ "path": pad.to_string_lossy(), "content": "aa aa" }),
        &fence,
    );
    assert!(!dup.is_error);
    let many = host::call(
        "str_replace",
        &json!({
            "path": pad.to_string_lossy(),
            "old_string": "aa",
            "new_string": "bb"
        }),
        &fence,
    );
    assert!(many.is_error);
    assert!(many.content.contains("more than once"));

    let all = host::call(
        "str_replace",
        &json!({
            "path": pad.to_string_lossy(),
            "old_string": "aa",
            "new_string": "bb",
            "replace_all": true
        }),
        &fence,
    );
    assert!(!all.is_error, "{}", all.content);
    assert_eq!(fs::read_to_string(&pad).expect("read pad"), "bb bb");
    assert!(all.content.contains("2 occurrence"));
}

#[test]
fn write_and_str_replace_deny_readable_files_outside_scratch() {
    let (fence, read_root, scratch) = live_fence();
    let file = read_root.join("note.md");
    fs::write(&file, "hello").expect("seed");

    let denied_write = host::call(
        "write",
        &json!({ "path": file.to_string_lossy(), "content": "hello world" }),
        &fence,
    );
    assert!(denied_write.is_error);
    assert_eq!(denied_write.content, "path is outside the write fence");
    assert_eq!(fs::read_to_string(&file).expect("read"), "hello");

    let denied_edit = host::call(
        "str_replace",
        &json!({
            "path": file.to_string_lossy(),
            "old_string": "hello",
            "new_string": "staged"
        }),
        &fence,
    );
    assert!(denied_edit.is_error);
    assert_eq!(denied_edit.content, "path is outside the write fence");
    assert_eq!(fs::read_to_string(&file).expect("read"), "hello");

    let pad = scratch.join("pad.md");
    let wrote = host::call(
        "write",
        &json!({ "path": pad.to_string_lossy(), "content": "scratch" }),
        &fence,
    );
    assert!(!wrote.is_error, "{}", wrote.content);
}

#[test]
fn write_str_replace_copy_allow_exact_staged_file() {
    let (mut fence, read_root, scratch) = live_fence();
    let file = read_root.join("note.md");
    let sibling = read_root.join("other.md");
    let source = read_root.join("src.md");
    fs::write(&file, "hello").expect("seed");
    fs::write(&source, "copied").expect("source");
    fence.grant_staged_file(file.clone());

    let wrote = host::call(
        "write",
        &json!({ "path": file.to_string_lossy(), "content": "hello world" }),
        &fence,
    );
    assert!(!wrote.is_error, "{}", wrote.content);
    assert_eq!(fs::read_to_string(&file).expect("read"), "hello world");

    let edited = host::call(
        "str_replace",
        &json!({
            "path": file.to_string_lossy(),
            "old_string": "world",
            "new_string": "staged"
        }),
        &fence,
    );
    assert!(!edited.is_error, "{}", edited.content);
    assert_eq!(fs::read_to_string(&file).expect("read"), "hello staged");

    let denied_sibling = host::call(
        "write",
        &json!({ "path": sibling.to_string_lossy(), "content": "no" }),
        &fence,
    );
    assert!(denied_sibling.is_error);
    assert_eq!(denied_sibling.content, "path is outside the write fence");
    assert!(!sibling.exists());

    let copied = host::call(
        "copy",
        &json!({
            "source_path": source.to_string_lossy(),
            "dest_path": file.to_string_lossy()
        }),
        &fence,
    );
    assert!(!copied.is_error, "{}", copied.content);
    assert_eq!(fs::read_to_string(&file).expect("read"), "copied");

    let pad = scratch.join("pad.md");
    let to_scratch = host::call(
        "copy",
        &json!({
            "source_path": source.to_string_lossy(),
            "dest_path": pad.to_string_lossy()
        }),
        &fence,
    );
    assert!(!to_scratch.is_error, "{}", to_scratch.content);
}

#[test]
fn copy_reads_source_and_writes_only_under_scratch() {
    let (fence, read_root, scratch) = live_fence();
    let source = read_root.join("note.md");
    fs::write(&source, "hello copy").expect("seed");

    let dest = scratch.join("note.md");
    let copied = host::call(
        "copy",
        &json!({
            "source_path": source.to_string_lossy(),
            "dest_path": dest.to_string_lossy()
        }),
        &fence,
    );
    assert!(!copied.is_error, "{}", copied.content);
    assert_eq!(fs::read_to_string(&dest).expect("dest"), "hello copy");
    assert_eq!(fs::read_to_string(&source).expect("source"), "hello copy");

    let overwrite = host::call(
        "copy",
        &json!({
            "source_path": source.to_string_lossy(),
            "dest_path": dest.to_string_lossy()
        }),
        &fence,
    );
    assert!(!overwrite.is_error, "{}", overwrite.content);

    let denied_dest = host::call(
        "copy",
        &json!({
            "source_path": source.to_string_lossy(),
            "dest_path": read_root.join("out.md").to_string_lossy()
        }),
        &fence,
    );
    assert!(denied_dest.is_error);
    assert_eq!(denied_dest.content, "path is outside the write fence");
    assert!(!read_root.join("out.md").exists());

    let root_dest = host::call(
        "copy",
        &json!({
            "source_path": source.to_string_lossy(),
            "dest_path": scratch.to_string_lossy()
        }),
        &fence,
    );
    assert!(root_dest.is_error);
    assert_eq!(root_dest.content, "dest_path must be a file");

    let denied_source = host::call(
        "copy",
        &json!({
            "source_path": "/etc/hosts",
            "dest_path": dest.to_string_lossy()
        }),
        &fence,
    );
    assert!(denied_source.is_error);
    assert!(denied_source.content.contains("read fence"));
}
