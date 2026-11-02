use super::*;

const A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const C: &str = "cccccccccccccccccccccccccccccccc";

fn local(docs: &[(&str, i64)]) -> Vec<LocalDoc> {
    docs.iter().map(|(id, version)| LocalDoc { doc_id: id.to_string(), version: *version }).collect()
}

fn cloud(docs: &[(&str, i64)]) -> Vec<CloudDoc> {
    docs.iter().map(|(id, version)| CloudDoc { document_id: id.to_string(), version: *version }).collect()
}

fn upload(id: &str, version: i64) -> SyncAction {
    SyncAction::Upload { doc_id: id.into(), version }
}

fn pull(id: &str, version: i64) -> SyncAction {
    SyncAction::Pull { doc_id: id.into(), version }
}

fn skip(id: &str) -> SyncAction {
    SyncAction::Skip { doc_id: id.into() }
}

#[test]
fn the_five_cases() {
    // (what, local, cloud, expected)
    let cases: [(&str, Vec<LocalDoc>, Vec<CloudDoc>, Vec<SyncAction>); 5] = [
        ("local is higher", local(&[(A, 5)]), cloud(&[(A, 3)]), vec![upload(A, 5)]),
        ("the cloud has none", local(&[(A, 5)]), cloud(&[]), vec![upload(A, 5)]),
        ("both are the same", local(&[(A, 5)]), cloud(&[(A, 5)]), vec![skip(A)]),
        ("the cloud is higher", local(&[(A, 3)]), cloud(&[(A, 5)]), vec![pull(A, 5)]),
        ("it is missing here", local(&[]), cloud(&[(A, 5)]), vec![pull(A, 5)]),
    ];
    for (what, mine, theirs, expected) in cases {
        assert_eq!(plan(&mine, &theirs), expected, "{what}");
    }
}

#[test]
fn version_zero_is_a_version_like_any_other() {
    assert_eq!(plan(&local(&[(A, 0)]), &cloud(&[])), [upload(A, 0)], "a new document is sent");
    assert_eq!(plan(&local(&[(A, 0)]), &cloud(&[(A, 0)])), [skip(A)]);
    assert_eq!(plan(&local(&[(A, 1)]), &cloud(&[(A, 0)])), [upload(A, 1)]);
    assert_eq!(plan(&local(&[(A, 0)]), &cloud(&[(A, 1)])), [pull(A, 1)]);
}

#[test]
fn only_one_step_apart_is_enough_to_tell() {
    assert_eq!(plan(&local(&[(A, 8)]), &cloud(&[(A, 7)])), [upload(A, 8)]);
    assert_eq!(plan(&local(&[(A, 7)]), &cloud(&[(A, 7)])), [skip(A)]);
    assert_eq!(plan(&local(&[(A, 7)]), &cloud(&[(A, 8)])), [pull(A, 8)]);
}

#[test]
fn every_document_of_either_side_gets_one_action_ordered_by_id() {
    let actions = plan(
        &local(&[(C, 1), (A, 2), (B, 9)]),
        &cloud(&[(B, 9), ("dddddddddddddddddddddddddddddddd", 4), (A, 1)]),
    );

    assert_eq!(
        actions,
        [upload(A, 2), skip(B), upload(C, 1), pull("dddddddddddddddddddddddddddddddd", 4)]
    );
}

#[test]
fn an_id_only_the_cloud_has_sorts_in_among_the_others() {
    let actions = plan(&local(&[(A, 1), (C, 1)]), &cloud(&[(B, 1)]));

    assert_eq!(actions, [upload(A, 1), pull(B, 1), upload(C, 1)]);
}

#[test]
fn the_order_of_the_input_does_not_matter() {
    let forward = plan(&local(&[(A, 1), (B, 2), (C, 3)]), &cloud(&[(A, 3), (B, 2)]));
    let backward = plan(&local(&[(C, 3), (B, 2), (A, 1)]), &cloud(&[(B, 2), (A, 3)]));

    assert_eq!(forward, backward);
    assert_eq!(forward, [pull(A, 3), skip(B), upload(C, 3)]);
}

#[test]
fn nothing_on_either_side_plans_nothing() {
    assert_eq!(plan(&[], &[]), []);
}

#[test]
fn a_repeated_id_counts_with_its_highest_version() {
    assert_eq!(plan(&local(&[(A, 2), (A, 6), (A, 4)]), &cloud(&[(A, 5)])), [upload(A, 6)]);
    assert_eq!(plan(&local(&[(A, 5)]), &cloud(&[(A, 2), (A, 6), (A, 4)])), [pull(A, 6)]);
    assert_eq!(plan(&local(&[(A, 6)]), &cloud(&[(A, 2), (A, 6), (A, 4)])), [skip(A)]);
    assert_eq!(plan(&[], &cloud(&[(A, 2), (A, 6), (A, 4)])), [pull(A, 6)]);
}

#[test]
fn an_action_names_its_document() {
    assert_eq!(upload(A, 1).doc_id(), A);
    assert_eq!(pull(B, 2).doc_id(), B);
    assert_eq!(skip(C).doc_id(), C);
}
