use std::fs;
use std::path::PathBuf;

use crate::services::docs::DocCategory;
use crate::test_support::TestSandbox;

fn source(rel: &str) -> String {
    fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel))
        .unwrap_or_else(|err| panic!("{rel}: {err}"))
}

fn ids(value: &serde_json::Value) -> Vec<(&str, bool)> {
    value["categories"]
        .as_array()
        .expect("categories")
        .iter()
        .map(|row| {
            (
                row["id"].as_str().expect("id"),
                row["exposed"].as_bool().expect("exposed"),
            )
        })
        .collect()
}

#[test]
fn commands_are_declared_and_registered_in_generate_handler() {
    let commands_mod = source("src/commands/mod.rs");
    assert!(
        commands_mod.contains("pub mod doc_categories"),
        "commands/mod.rs must declare doc_categories"
    );
    let lib = source("src/lib.rs");
    for name in [
        "commands::doc_categories::get_doc_categories",
        "commands::doc_categories::set_doc_categories",
    ] {
        assert!(
            lib.contains(name),
            "lib.rs generate_handler must register {name}"
        );
    }
}

#[test]
fn get_writes_builtin_and_lists_hidden_jot() {
    let sandbox = TestSandbox::new();
    let got = super::get_doc_categories().expect("get");
    assert_eq!(got["ok"], true);
    assert_eq!(
        ids(&got),
        vec![
            ("summary", true),
            ("dialogue", true),
            ("video", true),
            ("article", true),
            ("jot", false),
        ]
    );
    sandbox
        .assert_not_prod_path(&sandbox.data_dir().join("doc_categories.json"))
        .expect("sandbox file");
}

#[test]
fn set_round_trips_and_can_expose_jot() {
    let _sandbox = TestSandbox::new();
    let mut items = crate::services::docs::load_doc_categories().expect("load");
    for item in &mut items {
        if item.id == "jot" {
            item.exposed = true;
        }
    }
    let resp = super::set_doc_categories(items.clone()).expect("set");
    assert_eq!(resp["ok"], true);
    let again = super::get_doc_categories().expect("get after");
    assert_eq!(ids(&again).iter().find(|(id, _)| *id == "jot"), Some(&("jot", true)));
    assert!(crate::services::docs::is_exposed("jot"));
}

#[test]
fn set_rejects_whitespace_id_and_keeps_the_previous_list() {
    let _sandbox = TestSandbox::new();
    super::get_doc_categories().expect("seed");
    let before = super::get_doc_categories().expect("before");
    let err = super::set_doc_categories(vec![DocCategory {
        id: "foo bar".into(),
        label: "Foo".into(),
        exposed: true,
    }]);
    assert!(err.is_err(), "{err:?}");
    let after = super::get_doc_categories().expect("after");
    assert_eq!(after, before);
}
