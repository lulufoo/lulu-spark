use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use serde_json::json;

use super::*;
use crate::services::spark_data::DocRow;

const DOC: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const RAW: &str = "notes/202601011200-alpha1.md";

fn row(digest: bool, langs: &[&str]) -> DocRow {
    let mut doc = DocRow::new(DOC, RAW, "srcp");
    if digest {
        doc.digest_path = Some(RAW.to_string());
    }
    doc.translations = json!(langs);
    doc
}

fn suffixes(list: &[&str]) -> impl FnMut() -> String {
    let mut queue: Vec<String> = list.iter().rev().map(|s| s.to_string()).collect();
    move || queue.pop().expect("suffix")
}

fn workspace(tmp: &tempfile::TempDir, name: &str) -> PathBuf {
    let root = tmp.path().join(name);
    fs::create_dir_all(root.join("raw")).expect("raw");
    fs::create_dir_all(root.join("digest")).expect("digest");
    root
}

fn write_under(root: &std::path::Path, layer: &str, project: &str, rel: &str, bytes: &[u8]) -> PathBuf {
    let path = root.join(layer).join(project).join(rel);
    fs::create_dir_all(path.parent().unwrap()).expect("parent");
    fs::write(&path, bytes).expect("write");
    path
}

#[test]
fn plan_relocate_puts_attached_before_body_files() {
    let plan = plan_relocate(&row(true, &["zh"]), "destp", &HashSet::new(), &HashSet::new(), suffixes(&[]))
        .expect("plan");
    let steps = stage_sequence(&plan);
    assert_eq!(steps[0], "attached");
    assert!(steps[1].starts_with("raw:"), "{steps:?}");
    assert!(steps.iter().any(|s| s.starts_with("digest:")), "{steps:?}");
}

#[test]
fn plan_relocate_reuses_merge_suffix_on_conflict() {
    let taken = HashSet::from([RAW.to_string()]);
    let plan = plan_relocate(&row(false, &[]), "destp", &taken, &HashSet::new(), suffixes(&["ab12cd"]))
        .expect("plan");
    assert!(plan.target.renamed);
    assert_eq!(plan.target.raw_path, "notes/202601011200-alpha1-ab12cd.md");
}

#[test]
fn stage_relocate_copies_attached_then_bodies_and_leaves_the_source() {
    let tmp = tempfile::tempdir().expect("tmp");
    let src = workspace(&tmp, "src");
    let dest = workspace(&tmp, "dest");
    write_under(&src, "raw", "srcp", RAW, b"# raw\n");
    write_under(&src, "digest", "srcp", RAW, b"# digest\n");
    write_under(
        &src,
        "raw",
        "srcp",
        &format!("notes/.attached/{DOC}/a.png"),
        b"png",
    );
    let plan = plan_relocate(&row(true, &[]), "destp", &HashSet::new(), &HashSet::new(), suffixes(&[]))
        .expect("plan");
    stage_relocate(&src, &dest, &plan).expect("stage");
    assert_eq!(fs::read(src.join("raw/srcp").join(RAW)).unwrap(), b"# raw\n");
    assert_eq!(
        fs::read(src.join(format!("raw/srcp/notes/.attached/{DOC}/a.png"))).unwrap(),
        b"png"
    );
    assert_eq!(fs::read(dest.join("raw/destp").join(RAW)).unwrap(), b"# raw\n");
    assert_eq!(
        fs::read(dest.join(format!("raw/destp/notes/.attached/{DOC}/a.png"))).unwrap(),
        b"png"
    );
    assert_eq!(fs::read(dest.join("digest/destp").join(RAW)).unwrap(), b"# digest\n");
}

#[test]
fn cleanup_relocate_removes_source_files_and_the_attached_dir() {
    let tmp = tempfile::tempdir().expect("tmp");
    let src = workspace(&tmp, "src");
    let dest = workspace(&tmp, "dest");
    write_under(&src, "raw", "srcp", RAW, b"# raw\n");
    write_under(
        &src,
        "raw",
        "srcp",
        &format!("notes/.attached/{DOC}/a.png"),
        b"png",
    );
    let plan = plan_relocate(&row(false, &[]), "destp", &HashSet::new(), &HashSet::new(), suffixes(&[]))
        .expect("plan");
    stage_relocate(&src, &dest, &plan).expect("stage");
    cleanup_relocate(&src, &plan).expect("cleanup");
    assert!(!src.join("raw/srcp").join(RAW).exists());
    assert!(!src.join(format!("raw/srcp/notes/.attached/{DOC}")).exists());
    assert_eq!(fs::read(dest.join("raw/destp").join(RAW)).unwrap(), b"# raw\n");
    assert_eq!(
        fs::read(dest.join(format!("raw/destp/notes/.attached/{DOC}/a.png"))).unwrap(),
        b"png"
    );
}
