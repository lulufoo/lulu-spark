use super::types::{
    AppSettings, LlmSettings, LlmSettingsEntry, SettingsError, HOST_LLM_BASE_URL, HOST_LLM_PLATFORM,
};

pub fn normalize_engine_value(raw: &str) -> Option<&'static str> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "host" => Some("host"),
        "openai" => Some("openai"),
        "claude" => Some("claude"),
        "grok" => Some("grok"),
        "kimi" => Some("kimi"),
        "qwen" => Some("qwen"),
        _ => None,
    }
}

/// Resolve the list entry for a known LLM category.
/// Missing type or empty list → `None` (no panic). Illegal type → `None`.
pub fn llm_entry_by_type<'a>(
    entries: &'a [LlmSettingsEntry],
    engine_type: &str,
) -> Option<&'a LlmSettingsEntry> {
    let normalized = normalize_engine_value(engine_type)?;
    entries
        .iter()
        .find(|e| normalize_engine_value(&e.engine_type) == Some(normalized))
}

/// Update or insert a typed LLM entry. Illegal/unknown type is rejected (not written).
pub fn upsert_llm_entry(
    entries: &mut Vec<LlmSettingsEntry>,
    engine_type: &str,
    fields: &LlmSettings,
) -> Result<(), SettingsError> {
    let Some(normalized) = normalize_engine_value(engine_type) else {
        return Err(SettingsError::ConfigGuard(format!(
            "invalid llm entry type: {engine_type}"
        )));
    };
    if let Some(existing) = entries
        .iter_mut()
        .find(|e| normalize_engine_value(&e.engine_type) == Some(normalized))
    {
        existing.engine_type = normalized.to_string();
        existing.platform = fields.platform.clone();
        existing.base_url = fields.base_url.clone();
        existing.model = fields.model.clone();
    } else {
        entries.push(LlmSettingsEntry {
            engine_type: normalized.to_string(),
            platform: fields.platform.clone(),
            base_url: fields.base_url.clone(),
            model: fields.model.clone(),
        });
    }
    Ok(())
}

/// Built-in preset metadata (aligned with frontend `engine-presets.ts`).
/// Platform is never client-writable. `base_url` is client-writable; empty falls back
/// to that category's default.
pub(super) fn builtin_preset_fields(engine: &str) -> Option<(&'static str, &'static str)> {
    match normalize_engine_value(engine) {
        Some("host") => Some((HOST_LLM_PLATFORM, HOST_LLM_BASE_URL)),
        Some("openai") => Some(("openai", "https://api.openai.com/v1")),
        Some("claude") => Some(("claude", "https://api.anthropic.com/v1")),
        Some("grok") => Some(("grok", "https://api.x.ai/v1")),
        Some("kimi") => Some(("kimi", "https://api.moonshot.cn/v1")),
        Some("qwen") => Some(("qwen", "https://dashscope.aliyuncs.com/compatible-mode/v1")),
        _ => None,
    }
}

/// Stamp Host platform (always) and default `base_url` when the stored value is empty.
pub fn stamp_readonly_preset_fields(settings: &mut AppSettings) {
    let Some(engine) = normalize_engine_value(&settings.assistant_engine) else {
        return;
    };
    if let Some((platform, default_base_url)) = builtin_preset_fields(engine) {
        let mut fields = llm_entry_by_type(&settings.llm, engine)
            .map(LlmSettingsEntry::fields)
            .unwrap_or_default();
        fields.platform = platform.to_string();
        if fields.base_url.trim().is_empty() {
            fields.base_url = default_base_url.to_string();
        }
        let _ = upsert_llm_entry(&mut settings.llm, engine, &fields);
    }
}

pub(super) fn stamp_host_preset_on_load(settings: &mut AppSettings) {
    let Some(engine) = normalize_engine_value(&settings.assistant_engine) else {
        return;
    };
    let needs_stamp = match llm_entry_by_type(&settings.llm, engine) {
        Some(entry) => entry.platform.trim().is_empty() || entry.base_url.trim().is_empty(),
        None => true,
    };
    if needs_stamp {
        stamp_readonly_preset_fields(settings);
    }
}
