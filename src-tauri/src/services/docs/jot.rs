use serde_json::{json, Value};

use crate::services::docs;
use crate::services::translation_gate;

use super::create_meta::format_created_at_zh;

/// Options for Host-side jot shell synthesis.
/// `source_type` is always `jot` — callers cannot override it.
#[derive(Debug, Clone, Default)]
pub struct JotCreateOpts {
    /// Fixed `YYYYMMDDHHMM` (UTC+8) for tests; `None` uses current time.
    pub ts: Option<String>,
}

/// Build a minimal archive MD shell for a jot (H1 / 创建时间). No navigation.
/// No persistence.
pub fn synthesize_jot_document(body: &str, opts: &JotCreateOpts) -> Result<String, String> {
    if body.trim().is_empty() {
        return Err("empty body".into());
    }
    let ts = opts.ts.clone().unwrap_or_else(|| {
        chrono::Utc::now()
            .with_timezone(&chrono::FixedOffset::east_opt(8 * 3600).expect("UTC+8"))
            .format("%Y%m%d%H%M")
            .to_string()
    });
    let first = body.lines().next().unwrap_or("").trim();
    let title = if first.is_empty() {
        ts.clone()
    } else {
        first.to_string()
    };
    let created_at_zh = format_created_at_zh(&ts)?;
    Ok(format!(
        "# {title}\n\n> 创建时间：{created_at_zh}\n\n---\n\n{body}"
    ))
}

/// Write a jot through the docs service (`{project}/notes/{ts}-{rand}.md`) into `workspace`
/// (the default workspace when `None`) and `project` (`inbox` when `None`). A project that does
/// not exist there or is disabled is refused before anything is written.
pub fn create_jot(
    workspace: Option<&str>,
    project: Option<&str>,
    body: &str,
    opts: &JotCreateOpts,
) -> Result<Value, String> {
    if body.trim().is_empty() {
        return Err("empty body".into());
    }
    if translation_gate::is_full_english(body) {
        return Err("400: full English note requires translations.zh".into());
    }
    let doc = synthesize_jot_document(body, opts).map_err(|e| format!("400: {e}"))?;
    let first = body.lines().next().unwrap_or("").trim();
    let ts = opts.ts.clone().unwrap_or_default();
    let title = if first.is_empty() {
        if ts.is_empty() {
            "jot".to_string()
        } else {
            ts.clone()
        }
    } else {
        first.to_string()
    };
    let mut payload = json!({
        "title": title,
        "source_type": "jot",
        "project": project.unwrap_or("inbox"),
        "folder": "notes",
        "digest": "never",
        "content": doc,
    });
    if let Some(workspace) = workspace {
        payload["workspace"] = json!(workspace);
    }
    if !ts.is_empty() {
        payload["created_at"] = json!(ts);
    }
    let result = docs::create_doc_content(&payload);
    if result.get("ok") == Some(&json!(true)) {
        Ok(result)
    } else {
        let status = result
            .get("_status")
            .and_then(|v| v.as_u64())
            .unwrap_or(500);
        let error = result
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("archive failed");
        Err(format!("{status}: {error}"))
    }
}
