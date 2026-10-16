use super::*;

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(path, text).expect("write");
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).expect("read")
}

fn folders(names: &[&str]) -> Vec<String> {
    names.iter().map(|n| n.to_string()).collect()
}

#[test]
fn notes_and_knowledge_files_land_in_their_workspace_folders() {
    let dir = tempfile::tempdir().expect("tmp");
    let data = dir.path();
    write(&data.join("notes/raw/inbox/a.md"), "raw a");
    write(&data.join("notes/digest/inbox/a.md"), "digest a");
    write(&data.join("notes/categories.json"), "{}");
    write(&data.join("knowledge/demo/docs/x.md"), "x");
    write(&data.join("knowledge/demo/.git/HEAD"), "ref");

    let report = copy_legacy_files(data, &folders(&["demo", "ghost"])).expect("copy");

    let ws = data.join("workspaces");
    assert_eq!(read(&ws.join("default/raw/inbox/a.md")), "raw a");
    assert_eq!(read(&ws.join("default/digest/inbox/a.md")), "digest a");
    assert_eq!(read(&ws.join("knowledge/raw/demo/docs/x.md")), "x");
    assert!(!ws.join("knowledge/raw/demo/.git").exists(), ".git is skipped");
    assert!(!ws.join("default/categories.json").exists());
    assert_eq!((report.copied, report.skipped_existing), (3, 0));
    assert_eq!(report.missing_folders, folders(&["ghost"]));
    assert!(ws.join(LEGACY_COPY_MARKER).exists());
    assert_eq!(read(&data.join("notes/raw/inbox/a.md")), "raw a", "sources stay");
    assert!(data.join("knowledge/demo/docs/x.md").exists(), "sources stay");
}

#[test]
fn a_finished_copy_is_not_repeated() {
    let dir = tempfile::tempdir().expect("tmp");
    let data = dir.path();
    write(&data.join("notes/raw/inbox/a.md"), "one");
    copy_legacy_files(data, &[]).expect("first");

    write(&data.join("notes/raw/inbox/b.md"), "two");
    let again = copy_legacy_files(data, &[]).expect("second");
    assert!(again.already_done);
    assert!(!data.join("workspaces/default/raw/inbox/b.md").exists());
}

#[test]
fn an_interrupted_copy_resumes_without_overwriting_existing_files() {
    let dir = tempfile::tempdir().expect("tmp");
    let data = dir.path();
    write(&data.join("notes/raw/inbox/a.md"), "old source");
    write(&data.join("notes/raw/inbox/b.md"), "b");
    write(&data.join("workspaces/default/raw/inbox/a.md"), "edited in new place");

    let report = copy_legacy_files(data, &[]).expect("copy");
    assert_eq!((report.copied, report.skipped_existing), (1, 1));
    assert_eq!(read(&data.join("workspaces/default/raw/inbox/a.md")), "edited in new place");
    assert_eq!(read(&data.join("workspaces/default/raw/inbox/b.md")), "b");
}

#[test]
fn nothing_to_copy_still_finishes_and_writes_the_marker() {
    let dir = tempfile::tempdir().expect("tmp");
    let report = copy_legacy_files(dir.path(), &[]).expect("copy");
    assert_eq!(report, LegacyCopyReport::default());
    assert!(dir.path().join("workspaces").join(LEGACY_COPY_MARKER).exists());
}
