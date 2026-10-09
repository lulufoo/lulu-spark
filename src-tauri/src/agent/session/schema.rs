use std::time::{SystemTime, UNIX_EPOCH};

pub const SESSION_STATUS_IDLE: &str = "idle";

pub const SESSION_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS meta (
  session_id TEXT PRIMARY KEY,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  title TEXT NOT NULL,
  status TEXT NOT NULL,
  llm TEXT
);
CREATE TABLE IF NOT EXISTS messages (
  message_id TEXT PRIMARY KEY,
  seq INTEGER NOT NULL,
  role TEXT NOT NULL,
  content TEXT NOT NULL,
  created_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS model_turns (
  turn_id TEXT PRIMARY KEY,
  seq INTEGER NOT NULL,
  created_at INTEGER NOT NULL,
  thinking_ms INTEGER
);
CREATE TABLE IF NOT EXISTS model_steps (
  step_id TEXT PRIMARY KEY,
  turn_id TEXT NOT NULL,
  seq INTEGER NOT NULL,
  kind TEXT NOT NULL,
  content TEXT NOT NULL,
  tool_call_id TEXT,
  tool_name TEXT,
  finish_reason TEXT,
  model TEXT,
  usage TEXT,
  reasoning_content TEXT,
  create_ts INTEGER,
  start_ts INTEGER,
  end_ts INTEGER
);
CREATE TABLE IF NOT EXISTS summaries (
  summary_id TEXT PRIMARY KEY,
  replaced_from_seq INTEGER NOT NULL,
  replaced_to_seq INTEGER NOT NULL,
  body TEXT NOT NULL,
  created_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS staged (
  staged_id TEXT PRIMARY KEY,
  path TEXT NOT NULL,
  title TEXT NOT NULL,
  source_kind TEXT,
  source_id TEXT
);
"#;

pub const CATALOG_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS sessions (
  session_id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  updated_at INTEGER NOT NULL,
  status TEXT NOT NULL,
  llm TEXT
);
"#;

pub fn unix_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub fn checked_session_id(session_id: &str) -> Result<&str, String> {
    let id = session_id.trim();
    if id.is_empty() {
        return Err("Missing session_id".into());
    }
    if id.contains('/') || id.contains('\\') || id.contains("..") {
        return Err("Invalid session_id".into());
    }
    Ok(id)
}
