use super::*;
use std::fs;

use crate::config::paths;
use crate::test_support::TestSandbox;

fn local_path() -> std::path::PathBuf {
    paths::doc_categories_path().expect("doc_categories path")
}

#[test]
fn builtin_has_four_exposed_and_hidden_jot() {
    let items = builtin_categories();
    assert_eq!(
        items
            .iter()
            .map(|c| (c.id.as_str(), c.label.as_str(), c.exposed))
            .collect::<Vec<_>>(),
        vec![
            ("summary", "Summary", true),
            ("dialogue", "Dialogue", true),
            ("video", "Video", true),
            ("article", "Article", true),
            ("jot", "Jot", false),
        ]
    );
}

#[test]
fn missing_file_writes_builtin_default() {
    let sandbox = TestSandbox::new();
    let path = local_path();
    assert!(!path.exists());
    let items = load().expect("load");
    assert_eq!(items, builtin_categories());
    assert!(path.is_file());
    sandbox.assert_not_prod_path(&path).expect("sandbox file");
    assert_eq!(path, sandbox.data_dir().join("doc_categories.json"));
}

#[test]
fn existing_file_is_read_as_is_and_not_reset() {
    let _sandbox = TestSandbox::new();
    let custom = vec![
        DocCategory {
            id: "summary".into(),
            label: "Summary".into(),
            exposed: true,
        },
        DocCategory {
            id: "custom".into(),
            label: "Custom".into(),
            exposed: false,
        },
    ];
    save(&custom).expect("save");
    let again = load().expect("load");
    assert_eq!(again, custom);
    assert_ne!(again, builtin_categories());
}

#[test]
fn invalid_existing_file_is_rejected_and_left_in_place() {
    let _sandbox = TestSandbox::new();
    let path = local_path();
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(&path, "{not json").expect("seed");
    assert!(load().is_err());
    assert_eq!(fs::read_to_string(&path).expect("kept"), "{not json");
}

#[test]
fn save_rejects_empty_id_duplicate_id_empty_label_and_does_not_write() {
    let _sandbox = TestSandbox::new();
    let path = local_path();
    let ok = vec![DocCategory {
        id: "summary".into(),
        label: "Summary".into(),
        exposed: true,
    }];
    save(&ok).expect("seed");
    let before = fs::read_to_string(&path).expect("before");

    assert!(save(&[DocCategory {
        id: "".into(),
        label: "X".into(),
        exposed: true,
    }])
    .is_err());
    assert!(save(&[DocCategory {
        id: "a".into(),
        label: "".into(),
        exposed: true,
    }])
    .is_err());
    assert!(save(&[
        DocCategory {
            id: "a".into(),
            label: "A".into(),
            exposed: true,
        },
        DocCategory {
            id: "a".into(),
            label: "B".into(),
            exposed: false,
        },
    ])
    .is_err());
    assert_eq!(fs::read_to_string(&path).expect("unchanged"), before);
}

#[test]
fn parse_rejects_missing_or_non_bool_exposed() {
    assert!(parse_list(r#"[{"id":"a","label":"A"}]"#).is_err());
    assert!(parse_list(r#"[{"id":"a","label":"A","exposed":"yes"}]"#).is_err());
}

#[test]
fn is_legal_and_is_exposed_follow_the_local_list() {
    let _sandbox = TestSandbox::new();
    load().expect("seed default");
    assert!(is_legal("summary"));
    assert!(is_legal("jot"));
    assert!(is_exposed("summary"));
    assert!(!is_exposed("jot"));
    assert!(!is_legal("theme-line"));
    assert!(!is_exposed("theme-line"));

    save(&[DocCategory {
        id: "jot".into(),
        label: "Jot".into(),
        exposed: true,
    }])
    .expect("expose jot");
    assert!(is_exposed("jot"));
    assert!(!is_legal("summary"));
}
