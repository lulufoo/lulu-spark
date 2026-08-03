//! Agent tools interface layer (OpenAI tool defs / Binding.tools parsing).
//! In-process business dispatch was removed; todos capability is MCP/HTTP only.

use serde_json::{json, Value};

pub fn openai_tool_definitions() -> Vec<Value> {
    vec![
        tool_def(
            "get_plan",
            "获取当前绑定计划标题/状态/子计划摘要",
            json!({ "type": "object", "properties": {}, "additionalProperties": false }),
        ),
        tool_def(
            "list_sub_tasks",
            "列出子计划 id 与标题",
            json!({ "type": "object", "properties": {}, "additionalProperties": false }),
        ),
        tool_def(
            "add_sub_task",
            "新增子计划",
            json!({
                "type": "object",
                "properties": { "title": { "type": "string" } },
                "required": ["title"],
                "additionalProperties": false
            }),
        ),
        tool_def(
            "update_sub_title",
            "改子标题",
            json!({
                "type": "object",
                "properties": {
                    "sub_task_id": { "type": "string" },
                    "title": { "type": "string" }
                },
                "required": ["sub_task_id", "title"],
                "additionalProperties": false
            }),
        ),
        tool_def(
            "update_master_title",
            "改主标题",
            json!({
                "type": "object",
                "properties": { "title": { "type": "string" } },
                "required": ["title"],
                "additionalProperties": false
            }),
        ),
    ]
}

fn tool_def(name: &str, description: &str, parameters: Value) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": name,
            "description": description,
            "parameters": parameters
        }
    })
}

/// Read opaque tool-handle `ctx.master_task_id` for `tool_name` from Binding.tools.
/// Host does not treat this as a Binding Contract primary key — consumer-owned handle data.
pub fn master_id_from_binding_tools(tools: &Value, tool_name: &str) -> Option<String> {
    let arr = tools.as_array()?;
    for item in arr {
        let name = item
            .get("name")
            .and_then(|v| v.as_str())
            .or_else(|| item.as_str())?;
        if name != tool_name {
            continue;
        }
        let id = item
            .get("ctx")
            .and_then(|c| c.get("master_task_id"))
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())?;
        return Some(id.to_string());
    }
    None
}

/// Tool names declared in Binding.tools (string or `{name}` objects).
pub fn tool_names_from_binding(tools: &Value) -> Vec<String> {
    match tools {
        Value::Array(arr) => arr
            .iter()
            .filter_map(|item| {
                item.get("name")
                    .and_then(|v| v.as_str())
                    .or_else(|| item.as_str())
                    .map(|s| s.to_string())
            })
            .collect(),
        Value::Object(map) => map.keys().cloned().collect(),
        Value::String(s) if !s.trim().is_empty() => vec![s.clone()],
        _ => Vec::new(),
    }
}

/// OpenAI tool defs intersected with Binding-declared names (interface layer).
pub fn openai_tool_definitions_for_binding(tools: &Value) -> Vec<Value> {
    let declared = tool_names_from_binding(tools);
    openai_tool_definitions()
        .into_iter()
        .filter(|def| {
            def.get("function")
                .and_then(|f| f.get("name"))
                .and_then(|n| n.as_str())
                .is_some_and(|n| declared.iter().any(|d| d == n))
        })
        .collect()
}
