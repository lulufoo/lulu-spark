//! Whitelisted Tools that call `plan_task` in-process.

use serde_json::{json, Value};

use crate::services::plan_task;

const TOOL_NAMES: [&str; 5] = [
    "get_plan",
    "list_sub_tasks",
    "add_sub_task",
    "update_sub_title",
    "update_master_title",
];

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

fn ok(data: Value) -> Value {
    json!({ "ok": true, "data": data })
}

fn err(code: &str, error: impl Into<String>) -> Value {
    json!({ "ok": false, "error": error.into(), "code": code })
}

fn binding_master_id(bound: Option<&str>) -> Result<String, Value> {
    let Some(raw) = bound else {
        return Err(err("forbidden", "No bound plan"));
    };
    let id = raw.trim();
    if id.is_empty() {
        return Err(err("forbidden", "No bound plan"));
    }
    let got = plan_task::get_by_id(id);
    let invalid = got.get("error").is_some()
        || got
            .get("_status")
            .and_then(|s| s.as_u64())
            .is_some_and(|s| s >= 400)
        || got.get("master_task_id").and_then(|v| v.as_str()).is_none();
    if invalid {
        return Err(err("forbidden", "Invalid bound plan"));
    }
    Ok(id.to_string())
}

fn map_plan_status(v: &Value) -> Option<Value> {
    let status = v.get("_status").and_then(|s| s.as_u64())?;
    if status < 400 {
        return None;
    }
    let message = v
        .get("error")
        .and_then(|e| e.as_str())
        .unwrap_or("request failed")
        .to_string();
    let code = match status {
        404 => "not_found",
        400 => "bad_request",
        403 => "forbidden",
        _ => "internal",
    };
    Some(err(code, message))
}

fn sub_tasks_summary(task: &Value) -> Vec<Value> {
    task.get("sub_tasks")
        .and_then(|s| s.as_array())
        .map(|arr| {
            arr.iter()
                .map(|s| {
                    json!({
                        "sub_task_id": s.get("sub_task_id").cloned().unwrap_or(Value::Null),
                        "title": s.get("title").cloned().unwrap_or(Value::Null),
                        "status": s.get("status").cloned().unwrap_or(Value::Null),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn dispatch(name: &str, args: &Value, bound_master_task_id: Option<&str>) -> Value {
    if !TOOL_NAMES.contains(&name) {
        return err("unsupported", format!("Unknown tool: {name}"));
    }

    let master_id = match binding_master_id(bound_master_task_id) {
        Ok(id) => id,
        Err(e) => return e,
    };

    match name {
        "get_plan" => {
            let task = plan_task::get_by_id(&master_id);
            if let Some(e) = map_plan_status(&task) {
                return e;
            }
            ok(json!({
                "master_task_id": task.get("master_task_id").cloned().unwrap_or(json!(master_id)),
                "title": task.get("title").cloned().unwrap_or(Value::Null),
                "status": task.get("status").cloned().unwrap_or(Value::Null),
                "sub_tasks": sub_tasks_summary(&task),
            }))
        }
        "list_sub_tasks" => {
            let task = plan_task::get_by_id(&master_id);
            if let Some(e) = map_plan_status(&task) {
                return e;
            }
            ok(json!({
                "master_task_id": master_id,
                "sub_tasks": sub_tasks_summary(&task),
            }))
        }
        "add_sub_task" => {
            let title = args.get("title").and_then(|v| v.as_str()).unwrap_or("");
            let v = plan_task::add_sub(&master_id, title);
            if let Some(e) = map_plan_status(&v) {
                return e;
            }
            let sub_id = v
                .get("sub_task_id")
                .cloned()
                .unwrap_or(Value::Null);
            ok(json!({
                "sub_task_id": sub_id,
                "title": title.trim(),
            }))
        }
        "update_sub_title" => {
            let sub_task_id = args
                .get("sub_task_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let title = args.get("title").and_then(|v| v.as_str()).unwrap_or("");
            let v = plan_task::update_sub_title(&master_id, sub_task_id, title);
            if let Some(e) = map_plan_status(&v) {
                return e;
            }
            ok(json!({
                "sub_task_id": sub_task_id,
                "title": title.trim(),
            }))
        }
        "update_master_title" => {
            let title = args.get("title").and_then(|v| v.as_str()).unwrap_or("");
            let v = plan_task::update_master_title(&master_id, title);
            if let Some(e) = map_plan_status(&v) {
                return e;
            }
            ok(json!({
                "master_task_id": master_id,
                "title": title.trim(),
            }))
        }
        _ => err("unsupported", format!("Unknown tool: {name}")),
    }
}
