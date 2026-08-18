use std::collections::BTreeMap;
use std::sync::Arc;

use rmcp::model::{Tool, ToolAnnotations};
use serde_json::json;

use super::{catalog_from_mcp_tools_for_tests, headers_from_config_for_tests};

#[test]
fn catalog_uses_mcp_schema_and_read_only_hint_for_model_tools() {
    let schema = Arc::new(
        json!({
            "type": "object",
            "properties": { "id": { "type": "string" } },
            "required": ["id"]
        })
        .as_object()
        .expect("schema object")
        .clone(),
    );
    let read = Tool::new("read_entry", "Read one entry", schema.clone())
        .with_annotations(ToolAnnotations::new().read_only(true));
    let write = Tool::new("write_entry", "Write one entry", schema)
        .with_annotations(ToolAnnotations::new().read_only(false));

    let catalog = catalog_from_mcp_tools_for_tests(vec![read, write]);
    assert!(catalog.contains("read_entry"));
    assert!(!catalog.is_mutating("read_entry"));
    assert!(catalog.is_mutating("write_entry"));
    assert!(catalog.validate_arguments("read_entry", &json!({"id": "abc"})).is_ok());
    assert!(
        catalog
            .validate_arguments("read_entry", &json!({}))
            .is_err(),
        "required fields must be rejected before MCP tools/call"
    );
    assert_eq!(
        catalog.definitions[0]["function"]["parameters"]["required"],
        json!(["id"]),
        "the model receives the server-provided JSON Schema"
    );
}

#[test]
fn registry_accept_header_is_left_to_rmcp_transport_defaults() {
    let headers = BTreeMap::from([
        (
            "Accept".to_string(),
            "application/json, text/event-stream".to_string(),
        ),
        ("X-Scene-Token".to_string(), "token".to_string()),
    ]);
    let headers = headers_from_config_for_tests(headers).expect("valid headers");

    assert!(
        !headers.keys().any(|name| name.as_str() == "accept"),
        "rmcp must own the Accept header"
    );
    assert_eq!(
        headers
            .iter()
            .find(|(name, _)| name.as_str() == "x-scene-token")
            .and_then(|(_, value)| value.to_str().ok()),
        Some("token")
    );
}
