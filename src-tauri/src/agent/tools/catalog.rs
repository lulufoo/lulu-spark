//! Model-facing tool catalog and shared result types.

use std::collections::{BTreeMap, BTreeSet};

use rmcp::model::Tool;
use serde_json::{json, Value};

/// Model-facing function definitions (MCP and/or Host).
#[derive(Debug, Clone, PartialEq)]
pub struct ToolCatalog {
    pub definitions: Vec<Value>,
    names: BTreeSet<String>,
    read_only_names: BTreeSet<String>,
    input_schemas: BTreeMap<String, Value>,
}

impl ToolCatalog {
    pub fn contains(&self, name: &str) -> bool {
        self.names.contains(name)
    }

    pub fn is_mutating(&self, name: &str) -> bool {
        self.contains(name) && !self.read_only_names.contains(name)
    }

    /// Validate model arguments against the published JSON Schema.
    pub fn validate_arguments(&self, name: &str, arguments: &Value) -> Result<(), String> {
        let schema = self
            .input_schemas
            .get(name)
            .ok_or_else(|| format!("tool '{name}' is not discovered"))?;
        validate_json_schema(schema, arguments, "$")
    }

    pub(crate) fn from_parts(
        definitions: Vec<Value>,
        names: BTreeSet<String>,
        read_only_names: BTreeSet<String>,
        input_schemas: BTreeMap<String, Value>,
    ) -> Self {
        Self {
            definitions,
            names,
            read_only_names,
            input_schemas,
        }
    }

    pub fn merge(mut self, other: ToolCatalog) -> Self {
        for def in other.definitions {
            let Some(name) = def
                .pointer("/function/name")
                .and_then(Value::as_str)
                .map(str::to_string)
            else {
                continue;
            };
            if self.names.contains(&name) {
                continue;
            }
            if other.read_only_names.contains(&name) {
                self.read_only_names.insert(name.clone());
            }
            if let Some(schema) = other.input_schemas.get(&name) {
                self.input_schemas.insert(name.clone(), schema.clone());
            }
            self.names.insert(name);
            self.definitions.push(def);
        }
        self
    }

    pub(crate) fn from_local_tools(tools: Vec<LocalTool>) -> Self {
        let mut definitions = Vec::new();
        let mut names = BTreeSet::new();
        let mut read_only_names = BTreeSet::new();
        let mut input_schemas = BTreeMap::new();
        for tool in tools {
            names.insert(tool.name.to_string());
            if tool.read_only {
                read_only_names.insert(tool.name.to_string());
            }
            input_schemas.insert(tool.name.to_string(), tool.parameters.clone());
            definitions.push(json!({
                "type": "function",
                "function": {
                    "name": tool.name,
                    "description": tool.description,
                    "parameters": tool.parameters,
                }
            }));
        }
        Self::from_parts(definitions, names, read_only_names, input_schemas)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolResult {
    pub content: String,
    pub is_error: bool,
}

pub(crate) struct LocalTool {
    pub name: &'static str,
    pub description: &'static str,
    pub parameters: Value,
    pub read_only: bool,
}

/// Build a catalog from an MCP `tools/list` response.
pub fn from_rmcp_tools(tools: Vec<Tool>) -> ToolCatalog {
    let mut definitions = Vec::with_capacity(tools.len());
    let mut names = BTreeSet::new();
    let mut read_only_names = BTreeSet::new();
    let mut input_schemas = BTreeMap::new();

    for tool in tools {
        let name = tool.name.to_string();
        if tool
            .annotations
            .as_ref()
            .and_then(|annotations| annotations.read_only_hint)
            .unwrap_or(false)
        {
            read_only_names.insert(name.clone());
        }
        input_schemas.insert(name.clone(), Value::Object((*tool.input_schema).clone()));
        names.insert(name);
        definitions.push(openai_tool_definition(&tool));
    }

    ToolCatalog::from_parts(definitions, names, read_only_names, input_schemas)
}

fn openai_tool_definition(tool: &Tool) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": tool.name,
            "description": tool.description.as_deref().unwrap_or_default(),
            "parameters": Value::Object((*tool.input_schema).clone()),
        }
    })
}

fn validate_json_schema(schema: &Value, value: &Value, path: &str) -> Result<(), String> {
    if let Some(expected) = schema.get("const") {
        if value != expected {
            return Err(format!("{path} must equal {expected}"));
        }
    }

    match schema.get("type").and_then(Value::as_str) {
        Some("object") => {
            let object = value
                .as_object()
                .ok_or_else(|| format!("{path} must be a JSON object"))?;
            let properties = schema
                .get("properties")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            for required in schema
                .get("required")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
            {
                if !object.contains_key(required) {
                    return Err(format!("{path} is missing required property '{required}'"));
                }
            }
            if schema.get("additionalProperties") == Some(&Value::Bool(false)) {
                if let Some(unknown) = object.keys().find(|key| !properties.contains_key(*key)) {
                    return Err(format!("{path} contains unsupported property '{unknown}'"));
                }
            }
            for (name, property_schema) in properties {
                if let Some(property) = object.get(&name) {
                    validate_json_schema(&property_schema, property, &format!("{path}.{name}"))?;
                }
            }
        }
        Some("array") => {
            let items = value
                .as_array()
                .ok_or_else(|| format!("{path} must be an array"))?;
            if let Some(item_schema) = schema.get("items") {
                for (index, item) in items.iter().enumerate() {
                    validate_json_schema(item_schema, item, &format!("{path}[{index}]"))?;
                }
            }
        }
        Some("string") if !value.is_string() => {
            return Err(format!("{path} must be a string"));
        }
        Some("boolean") if !value.is_boolean() => {
            return Err(format!("{path} must be a boolean"));
        }
        Some("number") if !value.is_number() => {
            return Err(format!("{path} must be a number"));
        }
        Some(_) | None => {}
    }
    Ok(())
}
