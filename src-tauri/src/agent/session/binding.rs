use serde_json::{json, Value};

use super::live::live_context_owner;
use super::types::{value_exposes_engine_selection, Binding, SetError};

fn tools_applicable(tools: &Value) -> bool {
    match tools {
        // Empty array is legal: Host Agent business session may submit tools:[].
        Value::Array(_) => true,
        Value::Object(map) => !map.is_empty(),
        Value::String(s) => !s.trim().is_empty(),
        _ => false,
    }
}

fn prompt_applicable(prompt: &Value) -> bool {
    match prompt {
        Value::String(s) => !s.trim().is_empty(),
        Value::Object(map) => !map.is_empty(),
        _ => false,
    }
}

fn callbacks_slot_ok(callbacks: &Value) -> bool {
    callbacks.is_object()
}

const KEY_BINDING_TOOL_NAME: &str = "__binding_key__";

/// Business key carried by a key-only Binding (public Set contract).
pub fn binding_business_key(binding: &Binding) -> Option<String> {
    let arr = binding.tools.as_array()?;
    let first = arr.first()?;
    if first.get("name").and_then(|n| n.as_str()) != Some(KEY_BINDING_TOOL_NAME) {
        return None;
    }
    let key = first.get("handle").and_then(|h| h.as_str())?.trim();
    if key.is_empty() {
        None
    } else {
        Some(key.to_string())
    }
}

/// Business routing identity derived from the current key-only Binding.
///
/// Public key-only parsing keeps the key in a Host-owned live slot rather than
/// exposing it as executable tools. The live chat `session_id` remains
/// history/cancellation metadata and is deliberately not an alternative source
/// for Agent selection.
pub fn binding_business_id(binding: &Binding) -> Option<String> {
    if let Some(key) = binding_business_key(binding) {
        return Some(key);
    }
    let live = live_context_owner();
    (live.current_binding.as_ref() == Some(binding))
        .then_some(live.current_business_id)
        .flatten()
}

/// Set validation: tools/prompt must be applicable; callbacks slot present.
pub fn validate_binding(binding: &Binding) -> Result<(), SetError> {
    if !tools_applicable(&binding.tools)
        || !prompt_applicable(&binding.prompt)
        || !callbacks_slot_ok(&binding.callbacks)
    {
        return Err(SetError::set_invalid());
    }
    Ok(())
}

/// Parse key-only Binding input. Rejects legacy `tools`/`prompt`/`callbacks` payload
/// and engine selection parameters. Side-effect free (no registry I/O).
pub fn binding_from_json(v: &Value) -> Result<Binding, SetError> {
    let obj = v.as_object().ok_or_else(SetError::set_invalid)?;
    // Legacy payload must not bypass key→MCP lookup at the public boundary.
    if obj.contains_key("tools") || obj.contains_key("prompt") || obj.contains_key("callbacks") {
        return Err(SetError::set_invalid());
    }
    if value_exposes_engine_selection(v) {
        return Err(SetError::set_invalid());
    }
    let key = match obj.get("key") {
        Some(Value::String(s)) => s.clone(),
        _ => return Err(SetError::set_invalid()),
    };
    if key.trim().is_empty() {
        return Err(SetError::set_invalid());
    }
    // Host-internal parse placeholders only — not client-supplied MCP/tools
    // payload. `prompt: "pending"` is never the final live prompt: key-only Set
    // (`try_set_binding_json`) replaces it with WORKBENCH_HOST_SYSTEM_PROMPT.
    Ok(Binding {
        tools: json!([{ "name": KEY_BINDING_TOOL_NAME, "handle": key }]),
        prompt: json!("pending"),
        callbacks: json!({}),
    })
}
