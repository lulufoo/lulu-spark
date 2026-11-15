use super::*;

fn sketch_body(token: &str) -> String {
    format!(r#"{{"bmd":"style abc123\nsketch \"Hi\"\ncontent {token}\n","title":"Hi","version":7}}"#)
}

#[test]
fn source_versions_must_be_positive_database_integers_without_a_default() {
    for version in [1, 7, i32::MAX] {
        let body = serde_json::json!({"bmd":"sketch\ncontent PDw+Pz89", "version": version});
        let doc = parse_document(&body.to_string()).expect("valid integer");
        assert_eq!(doc.version, version);
        assert_eq!(project_sketch(&doc).version, Some(version));
    }
    for version in [
        serde_json::Value::Null,
        serde_json::json!(0),
        serde_json::json!(-1),
        serde_json::json!(1.0),
        serde_json::json!(1.5),
        serde_json::json!("1"),
        serde_json::json!(true),
        serde_json::json!(i64::from(i32::MAX) + 1),
        serde_json::json!(u64::MAX),
    ] {
        let body = serde_json::json!({"bmd":"sketch\ncontent PDw+Pz89", "version": version});
        let failure = parse_document(&body.to_string()).expect_err("invalid version");
        assert_eq!(failure.status, "invalid_document");
        assert_eq!(failure.version, None);
        assert_eq!(failure.invalidation_scope, None);
    }
    assert_eq!(
        parse_document(r#"{"bmd":"sketch\ncontent PDw+Pz89"}"#)
            .expect_err("missing version").status,
        "invalid_document"
    );
}

#[test]
fn source_metadata_failures_are_separate_from_preview_failures() {
    for body in [
        "invalid JSON",
        "[]",
        r#"{"version":1}"#,
        r#"{"version":1,"bmd":null}"#,
        r#"{"version":1,"bmd":"boxes","title":42}"#,
    ] {
        let failure = parse_document(body).expect_err("invalid source metadata");
        assert_eq!(failure.status, "invalid_document");
        assert_eq!(failure.version, None);
        assert_eq!(failure.invalidation_scope, None);
    }
    let doc = parse_document(&sketch_body("not-base64!")).expect("valid Board document");
    assert!(doc.bmd.contains("not-base64!"));
    let projection = project_sketch(&doc);
    assert_eq!(projection.status, "invalid_source");
    assert_eq!(projection.version, Some(7));
    assert_eq!(projection.invalidation_scope, None);
    assert!(doc.bmd.contains("not-base64!"), "projection must not change source");
}

#[test]
fn generic_source_is_preserved_without_type_filtering_or_renderer_requirements() {
    for bmd in [
        "boxes \"Plan\"\r\nbox a \"图\"\n",
        "flowchart TD\n a --> b\n",
        "a future source type",
        "",
    ] {
        let body = serde_json::json!({"bmd":bmd,"title":"  原始标题  ","version":4});
        let doc = parse_document(&body.to_string()).expect("generic source");
        assert_eq!(doc.bmd, bmd);
        assert_eq!(doc.title, "  原始标题  ");
        let projection = project_sketch(&doc);
        assert_eq!(projection.status, "not_sketch");
        assert_eq!(projection.title.as_deref(), Some("原始标题"));
        assert_eq!(projection.version, Some(4));
        assert_eq!(projection.invalidation_scope, None);
    }
}
