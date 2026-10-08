use super::*;

#[test]
fn snapshot_fields_match_compile_and_locked_strings() {
    let snap = snapshot();
    assert_eq!(snap.is_debug, cfg!(debug_assertions));
    assert_eq!(snap.version, "Beta v0.1");
    assert_eq!(snap.product_name, "Lulu Spark");
}

#[test]
fn snapshot_json_uses_locked_keys() {
    let value = serde_json::to_value(snapshot()).expect("json");
    assert!(value.get("is_debug").is_some());
    assert!(value.get("version").is_some());
    assert!(value.get("product_name").is_some());
    assert_eq!(value["is_debug"], cfg!(debug_assertions));
    assert_eq!(value["version"], "Beta v0.1");
    assert_eq!(value["product_name"], "Lulu Spark");
    assert_eq!(value.as_object().expect("object").len(), 3);
}

#[test]
fn production_module_declares_the_test_tree() {
    let prod = include_str!("../../host/package.rs");
    assert!(
        prod.contains("#[path = \"../unit-tests/host/package.rs\"]"),
        "production file only declares that tests exist"
    );
    assert!(!prod.contains("#[test]"), "tests belong in the test tree");
    assert!(prod.contains("cfg!(debug_assertions)"));
    assert!(!prod.contains("config.toml"));
    assert!(!prod.contains("BuildConfig"));
}
