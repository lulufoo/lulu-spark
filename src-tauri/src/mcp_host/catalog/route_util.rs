//! Shared MCP tool route constructors.

use serde_json::{json, Value};

use crate::mcp_host::{HttpMethod, ToolRoute};

pub fn object_schema(properties: Value, required: &[&str]) -> Value {
    let mut schema = json!({
        "type": "object",
        "properties": properties,
        "additionalProperties": false,
    });
    if !required.is_empty() {
        schema["required"] = json!(required);
    }
    schema
}

pub fn route(
    name: &str,
    description: &str,
    method: HttpMethod,
    api_path: &str,
    input_schema: Value,
    read_only: bool,
    destructive: bool,
) -> ToolRoute {
    ToolRoute {
        name: name.into(),
        description: description.into(),
        method,
        api_path: api_path.into(),
        read_only,
        destructive,
        input_schema,
    }
}
