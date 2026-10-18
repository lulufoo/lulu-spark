use std::collections::HashSet;

use serde_json::json;

use super::{plan_merge_targets, MergeTarget};
use crate::services::archive_parse::{expected_lang_common_path, is_valid_common_path};
use crate::services::spark_data::DocRow;

const A: &str = "notes/202601011200-alpha1.md";
const B: &str = "notes/202601021300-beta22.md";

fn doc(id: &str, raw: &str, digest: bool, langs: &[&str]) -> DocRow {
    let mut row = DocRow::new(id, raw, "p1");
    row.digest_path = digest.then(|| raw.to_string());
    row.translations = json!(langs);
    row
}

fn set(paths: &[&str]) -> HashSet<String> {
    paths.iter().map(|p| p.to_string()).collect()
}

/// A suffix source that hands out the given suffixes in order.
fn suffixes(list: &[&str]) -> impl FnMut() -> String {
    let mut queue: Vec<String> = list.iter().rev().map(|s| s.to_string()).collect();
    move || queue.pop().expect("suffix source ran dry")
}

fn plan(docs: &[DocRow], raw: &[&str], digest: &[&str], list: &[&str]) -> Vec<MergeTarget> {
    plan_merge_targets(docs, &set(raw), &set(digest), suffixes(list)).expect("plan")
}

#[test]
fn a_document_with_no_conflict_keeps_its_paths() {
    let targets = plan(&[doc("d1", A, true, &["zh"])], &[B], &[B], &[]);
    assert_eq!(
        targets,
        vec![MergeTarget { doc_id: "d1".into(), raw_path: A.into(), digest_path: Some(A.into()), renamed: false }]
    );
}

#[test]
fn a_primary_path_conflict_renames_the_file_name_only() {
    let targets = plan(&[doc("d1", A, true, &[])], &[A], &[], &["ab12cd"]);
    assert_eq!(targets[0].raw_path, "notes/202601011200-alpha1-ab12cd.md");
    assert_eq!(targets[0].digest_path.as_deref(), Some("notes/202601011200-alpha1-ab12cd.md"));
    assert!(targets[0].renamed);
}

#[test]
fn a_document_in_the_project_root_renames_like_any_other() {
    let root_doc = "202601011200-alpha1.md";
    let untouched = plan(&[doc("d1", root_doc, true, &["zh"])], &[A], &[A], &[]);
    assert_eq!(untouched[0].raw_path, root_doc);
    assert!(!untouched[0].renamed);
    let targets = plan(&[doc("d1", root_doc, true, &["zh"])], &[root_doc], &[], &["ab12cd"]);
    assert_eq!(targets[0].raw_path, "202601011200-alpha1-ab12cd.md");
    assert_eq!(targets[0].digest_path.as_deref(), Some("202601011200-alpha1-ab12cd.md"));
    assert!(targets[0].renamed);
    assert!(is_valid_common_path(&targets[0].raw_path));
}

#[test]
fn a_translation_only_conflict_renames_the_whole_document() {
    let taken_zh = expected_lang_common_path(A, "zh").expect("zh path");
    let targets = plan(&[doc("d1", A, false, &["zh"])], &[taken_zh.as_str()], &[], &["ab12cd"]);
    assert!(targets[0].renamed);
    assert_eq!(targets[0].raw_path, "notes/202601011200-alpha1-ab12cd.md");
    assert_eq!(targets[0].digest_path, None);
    // The translation follows the new primary name.
    let zh = expected_lang_common_path(&targets[0].raw_path, "zh").expect("zh path");
    assert_eq!(zh, "notes/202601011200-alpha1-ab12cd-zh.md");
}

#[test]
fn a_digest_only_conflict_renames_the_whole_document() {
    let targets = plan(&[doc("d1", A, true, &[])], &[], &[A], &["ab12cd"]);
    assert!(targets[0].renamed);
    assert_eq!(targets[0].raw_path, "notes/202601011200-alpha1-ab12cd.md");
    assert_eq!(targets[0].digest_path.as_deref(), Some("notes/202601011200-alpha1-ab12cd.md"));
}

#[test]
fn a_digest_with_its_own_name_gets_the_suffix_too() {
    let mut row = doc("d1", A, false, &[]);
    row.digest_path = Some(B.to_string());
    let targets = plan(&[row], &[A], &[], &["ab12cd"]);
    assert_eq!(targets[0].digest_path.as_deref(), Some("notes/202601021300-beta22-ab12cd.md"));
}

#[test]
fn a_suffix_that_is_taken_is_drawn_again() {
    let taken = "notes/202601011200-alpha1-ab12cd.md";
    let targets = plan(&[doc("d1", A, false, &[])], &[A, taken], &[], &["ab12cd", "ef34gh"]);
    assert_eq!(targets[0].raw_path, "notes/202601011200-alpha1-ef34gh.md");
}

#[test]
fn two_documents_never_end_on_the_same_path() {
    // The second document is renamed onto the name the first one just took.
    let first = doc("d1", A, false, &[]);
    let second = doc("d2", A, false, &[]);
    let targets = plan(&[first, second], &[A], &[], &["ab12cd", "ab12cd", "ef34gh"]);
    assert_eq!(targets[0].raw_path, "notes/202601011200-alpha1-ab12cd.md");
    assert_eq!(targets[1].raw_path, "notes/202601011200-alpha1-ef34gh.md");
}

#[test]
fn every_renamed_path_is_a_valid_common_path() {
    let docs = [doc("d1", A, true, &["zh", "ja"]), doc("d2", B, false, &[])];
    let targets = plan(&docs, &[A, B], &[A], &["ab12cd", "ef34gh"]);
    for target in &targets {
        assert!(target.renamed);
        assert!(is_valid_common_path(&target.raw_path), "{}", target.raw_path);
    }
}

#[test]
fn a_document_that_cannot_get_a_free_name_fails() {
    let taken: Vec<String> = (0..16).map(|i| format!("notes/202601011200-alpha1-aaaa{i:02}.md")).collect();
    let mut raw: HashSet<String> = taken.iter().cloned().collect();
    raw.insert(A.to_string());
    let list: Vec<String> = (0..16).map(|i| format!("aaaa{i:02}")).collect();
    let mut queue = list.into_iter();
    let result = plan_merge_targets(&[doc("d1", A, false, &[])], &raw, &HashSet::new(), || queue.next().expect("suffix"));
    assert!(result.is_err());
}

#[test]
fn a_path_that_cannot_take_a_suffix_fails() {
    let result = plan_merge_targets(&[doc("d1", "notes/odd.txt", false, &[])], &set(&["notes/odd.txt"]), &HashSet::new(), suffixes(&["ab12cd"]));
    assert!(result.is_err());
}
