use super::*;

fn workspace_folder(base: &Path, id: &str) -> PathBuf {
    let root = workspace_dir(base, id).expect("workspace dir");
    fs::create_dir_all(&root).expect("mkdir");
    root
}

#[test]
fn workspace_dir_joins_a_valid_id_and_refuses_anything_else() {
    let base = Path::new("/data/workspaces");
    assert_eq!(workspace_dir(base, "default").expect("ok"), base.join("default"));
    assert_eq!(workspace_dir(base, "ab12_x").expect("ok"), base.join("ab12_x"));
    for bad in ["", "..", "../x", "a/b", "A1", "a b", "a\\b"] {
        let err = workspace_dir(base, bad).expect_err(bad);
        assert_eq!(err["_status"], json!(400), "{bad}");
    }
}

#[test]
fn every_layer_of_a_document_lands_inside_its_workspace_folder() {
    let tmp = tempfile::tempdir().expect("tmp");
    let root = workspace_folder(tmp.path(), "ws1");
    let canon = root.canonicalize().expect("canon");

    let raw = doc_file_path(&root, "raw", "proj", "notes/a.md").expect("raw");
    let digest = doc_file_path(&root, "digest", "proj", "notes/a.md").expect("digest");
    let zh = doc_file_path(&root, "raw", "proj", "notes/a-zh.md").expect("translation");
    assert_eq!(raw, canon.join("raw/proj/notes/a.md"));
    assert_eq!(digest, canon.join("digest/proj/notes/a.md"));
    assert_eq!(zh, canon.join("raw/proj/notes/a-zh.md"));
    for path in [&raw, &digest, &zh] {
        assert!(path.starts_with(&canon), "{path:?}");
    }
}

#[test]
fn two_workspaces_never_share_a_path_for_the_same_project_and_file() {
    let tmp = tempfile::tempdir().expect("tmp");
    let a = workspace_folder(tmp.path(), "wa");
    let b = workspace_folder(tmp.path(), "wb");
    let in_a = doc_file_path(&a, "raw", "inbox", "notes/x.md").expect("a");
    let in_b = doc_file_path(&b, "raw", "inbox", "notes/x.md").expect("b");
    assert_ne!(in_a, in_b);
}

#[test]
fn doc_file_path_refuses_escapes_and_unknown_layers() {
    let tmp = tempfile::tempdir().expect("tmp");
    let root = workspace_folder(tmp.path(), "ws1");
    for (layer, folder, rel) in [
        ("raw", "", "notes/a.md"),
        ("raw", "..", "notes/a.md"),
        ("raw", "a/b", "notes/a.md"),
        ("raw", "proj", ""),
        ("raw", "proj", "../a.md"),
        ("other", "proj", "notes/a.md"),
    ] {
        let err = doc_file_path(&root, layer, folder, rel).expect_err(rel);
        assert_eq!(err["_status"], json!(400), "{layer} {folder} {rel}");
    }
}
