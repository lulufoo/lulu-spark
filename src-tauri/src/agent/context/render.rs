//! Render the next request the way `assets/glm-5.2/chat_template.jinja` does
//! for the message shapes this app sends.
//!
//! Defaults match an undefined `enable_thinking` and `reasoning_effort`:
//! the reasoning-effort line is `Max`, and the generation prompt ends in `<think>`.

use serde_json::{Map, Value};

const TOOLS_HEAD: &str = "<|system|>\n# Tools\n\nYou may call one or more functions to assist with the user query.\n\nYou are provided with function signatures within <tools></tools> XML tags:\n<tools>\n";
const TOOLS_TAIL: &str = "</tools>\n\nFor each function call, output the function name and arguments within the following XML format:\n<tool_call>{function-name}<arg_key>{arg-key-1}</arg_key><arg_value>{arg-value-1}</arg_value><arg_key>{arg-key-2}</arg_key><arg_value>{arg-value-2}</arg_value>...</tool_call>";

/// Render messages and tool definitions, including the generation prompt.
pub fn render_request(messages: &[Value], tools: &[Value]) -> String {
    let mut out = String::from("[gMASK]<sop><|system|>Reasoning Effort: Max");
    render_tools(&mut out, tools);
    for (index, message) in messages.iter().enumerate() {
        render_message(&mut out, messages, index, message);
    }
    out.push_str("<|assistant|><think>");
    out
}

fn render_tools(out: &mut String, tools: &[Value]) {
    if tools.is_empty() {
        return;
    }
    out.push_str(TOOLS_HEAD);
    for tool in tools {
        let Some(body) = included_function(tool) else {
            continue;
        };
        out.push('\n');
        out.push_str(&tool_to_json(body));
        out.push_str("\n\n");
    }
    out.push('\n');
    out.push_str(TOOLS_TAIL);
}

fn included_function(tool: &Value) -> Option<&Map<String, Value>> {
    let object = tool.as_object()?;
    let body = object
        .get("function")
        .and_then(Value::as_object)
        .unwrap_or(object);
    if body.get("defer_loading").and_then(Value::as_bool) == Some(true) {
        return None;
    }
    Some(body)
}

fn tool_to_json(tool: &Map<String, Value>) -> String {
    let mut parts = Vec::new();
    for (key, value) in tool {
        if key == "defer_loading" || key == "strict" {
            continue;
        }
        parts.push(format!("{}: {}", json_string(key), jinja_tojson(value)));
    }
    format!("{{{}}}", parts.join(", "))
}

fn render_message(out: &mut String, messages: &[Value], index: usize, message: &Value) {
    match message.get("role").and_then(Value::as_str).unwrap_or("") {
        "system" => {
            out.push_str("<|system|>");
            out.push_str(&text_content(message));
        }
        "user" => {
            out.push_str("<|user|>");
            out.push_str(&text_content(message));
        }
        "assistant" => render_assistant(out, message),
        "tool" => render_tool(out, messages, index, message),
        _ => {}
    }
}

fn render_assistant(out: &mut String, message: &Value) {
    out.push_str("<|assistant|><think></think>");
    let content = text_content(message);
    let trimmed = content.trim();
    if !trimmed.is_empty() {
        out.push_str(trimmed);
    }
    let Some(calls) = message.get("tool_calls").and_then(Value::as_array) else {
        return;
    };
    if calls.is_empty() {
        return;
    }
    out.push('\n');
    for call in calls {
        render_call(out, call);
    }
    out.push('\n');
}

fn render_call(out: &mut String, call: &Value) {
    let func = call.get("function").unwrap_or(call);
    let name = func.get("name").and_then(Value::as_str).unwrap_or("");
    out.push_str("<tool_call>");
    out.push_str(name);
    if let Some(arguments) = func.get("arguments") {
        for (key, value) in argument_pairs(arguments) {
            out.push_str("<arg_key>");
            out.push_str(&key);
            out.push_str("</arg_key><arg_value>");
            if let Some(text) = value.as_str() {
                out.push_str(text);
            } else {
                out.push_str(&jinja_tojson(&value));
            }
            out.push_str("</arg_value>");
        }
    }
    out.push_str("</tool_call>");
}

fn argument_pairs(raw: &Value) -> Vec<(String, Value)> {
    if let Some(text) = raw.as_str() {
        let Ok(parsed) = serde_json::from_str::<Value>(text) else {
            return Vec::new();
        };
        let Some(map) = parsed.as_object() else {
            return Vec::new();
        };
        let order = super::argument_order::object_key_order(text)
            .unwrap_or_else(|| map.keys().cloned().collect());
        return order
            .into_iter()
            .filter_map(|key| map.get(&key).map(|value| (key, value.clone())))
            .collect();
    }
    match raw {
        Value::Object(map) => map
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
        _ => Vec::new(),
    }
}

fn render_tool(out: &mut String, messages: &[Value], index: usize, message: &Value) {
    let previous_is_tool = index > 0
        && messages[index - 1].get("role").and_then(Value::as_str) == Some("tool");
    if !previous_is_tool {
        out.push_str("<|observation|>");
    }
    out.push_str("<tool_response>");
    out.push_str(&text_content(message));
    out.push_str("</tool_response>");
}

fn text_content(message: &Value) -> String {
    message
        .get("content")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

fn jinja_tojson(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(true) => "true".to_string(),
        Value::Bool(false) => "false".to_string(),
        Value::Number(number) => number.to_string(),
        Value::String(text) => json_string(text),
        Value::Array(items) => {
            let parts: Vec<String> = items.iter().map(jinja_tojson).collect();
            format!("[{}]", parts.join(", "))
        }
        Value::Object(map) => {
            let parts: Vec<String> = map
                .iter()
                .map(|(key, item)| format!("{}: {}", json_string(key), jinja_tojson(item)))
                .collect();
            format!("{{{}}}", parts.join(", "))
        }
    }
}

fn json_string(text: &str) -> String {
    let mut out = String::from("\"");
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{0008}' => out.push_str("\\b"),
            '\u{000c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 || c == '\u{007f}' => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
