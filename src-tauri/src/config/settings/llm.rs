use super::types::{
    AppSettings, LlmSettings, LlmSettingsEntry, SettingsError, HOST_LLM_BASE_URL, HOST_LLM_PLATFORM,
};

pub(super) fn normalize_engine_value(raw: &str) -> Option<&'static str> {
    (raw.trim().eq_ignore_ascii_case("host")).then_some("host")
}

/// Resolve the list entry for the supported engine type (`host`).
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

/// Built-in readonly preset metadata (aligned with frontend `engine-presets.js`).
/// Model remains independently editable; platform/base_url are never client-writable.
fn builtin_preset_fields(engine: &str) -> Option<(&'static str, &'static str)> {
    match normalize_engine_value(engine) {
        // OpenAI-compatible path (docs.bigmodel.cn); chat_url appends /chat/completions when base ends in /v4.
        Some("host") => Some((HOST_LLM_PLATFORM, HOST_LLM_BASE_URL)),
        _ => None,
    }
}

/// Stamp readonly preset fields from the current Engine category.
/// Ensures Host `load_llm_config` receives a usable base_url matching the UI preset.
pub fn stamp_readonly_preset_fields(settings: &mut AppSettings) {
    let Some(engine) = normalize_engine_value(&settings.assistant_engine) else {
        return;
    };
    if let Some((platform, base_url)) = builtin_preset_fields(engine) {
        let mut fields = llm_entry_by_type(&settings.llm, engine)
            .map(LlmSettingsEntry::fields)
            .unwrap_or_default();
        fields.platform = platform.to_string();
        fields.base_url = base_url.to_string();
        let _ = upsert_llm_entry(&mut settings.llm, engine, &fields);
    }
}

pub(super) fn stamp_host_preset_on_load(settings: &mut AppSettings) {
    let is_host = normalize_engine_value(&settings.assistant_engine).is_some();
    let needs_stamp = match llm_entry_by_type(&settings.llm, "host") {
        Some(entry) => entry.platform.trim().is_empty() && entry.base_url.trim().is_empty(),
        None => true,
    };
    if is_host && needs_stamp {
        stamp_readonly_preset_fields(settings);
    }
}
