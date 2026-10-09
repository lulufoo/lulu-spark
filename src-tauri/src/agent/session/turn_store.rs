use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde_json::{json, Value};

use crate::services::id::random_entry_id;

use super::types::Step;

pub(crate) struct StoredStep {
    step_id: String,
    step: Step,
}

pub(crate) fn load_stored_steps(conn: &Connection) -> Result<Vec<StoredStep>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT s.step_id, s.kind, s.content, s.tool_call_id, s.tool_name
             FROM model_steps s
             JOIN model_turns t ON t.turn_id = s.turn_id
             ORDER BY t.seq ASC, s.seq ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    let mut steps = Vec::new();
    for row in rows {
        let (step_id, kind, content, tool_call_id, tool_name) = row.map_err(|e| e.to_string())?;
        steps.push(StoredStep {
            step_id,
            step: decode_step(&kind, &content, tool_call_id, tool_name),
        });
    }
    Ok(steps)
}

pub(crate) fn load_steps(conn: &Connection) -> Result<Vec<Step>, String> {
    Ok(load_stored_steps(conn)?.into_iter().map(|stored| stored.step).collect())
}

pub(crate) fn sync(
    tx: &Transaction<'_>,
    stored_steps: &[StoredStep],
    current_steps: &[Step],
    now: i64,
) -> Result<(), String> {
    let shared_steps = stored_steps
        .iter()
        .zip(current_steps)
        .take_while(|(stored, current)| stored.step == **current)
        .count();
    if shared_steps < stored_steps.len() {
        delete_suffix(tx, stored_steps, shared_steps)?;
    }
    append_steps(tx, &current_steps[shared_steps..], now)
}

fn delete_suffix(
    tx: &Transaction<'_>,
    stored_steps: &[StoredStep],
    shared_steps: usize,
) -> Result<(), String> {
    let retained_messages = stored_steps[..shared_steps]
        .iter()
        .filter(|stored| is_ui_message(&stored.step))
        .count() as i64;
    for step in &stored_steps[shared_steps..] {
        tx.execute(
            "DELETE FROM model_steps WHERE step_id = ?1",
            params![step.step_id],
        )
        .map_err(|e| e.to_string())?;
    }
    tx.execute(
        "DELETE FROM messages WHERE seq > ?1",
        params![retained_messages],
    )
    .map_err(|e| e.to_string())?;
    tx.execute(
        "DELETE FROM model_turns
         WHERE NOT EXISTS (
           SELECT 1 FROM model_steps WHERE model_steps.turn_id = model_turns.turn_id
         )",
        [],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn append_steps(tx: &Transaction<'_>, steps: &[Step], now: i64) -> Result<(), String> {
    if steps.is_empty() {
        return Ok(());
    }
    let mut turn_state = tx
        .query_row(
            "SELECT turn_id, seq FROM model_turns ORDER BY seq DESC LIMIT 1",
            [],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    let mut step_seq = match &turn_state {
        Some((turn_id, _)) => tx
            .query_row(
                "SELECT COALESCE(MAX(seq), 0) FROM model_steps WHERE turn_id = ?1",
                params![turn_id],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|e| e.to_string())?,
        None => 0,
    };
    let mut message_seq: i64 = tx
        .query_row("SELECT COALESCE(MAX(seq), 0) FROM messages", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;

    for step in steps {
        if step.role == "user" || turn_state.is_none() {
            let next_turn_seq = next_model_turn_seq(tx)?;
            let turn_id = format!("turn_{}", random_entry_id());
            tx.execute(
                "INSERT INTO model_turns (turn_id, seq, created_at) VALUES (?1, ?2, ?3)",
                params![turn_id, next_turn_seq, now],
            )
            .map_err(|e| e.to_string())?;
            turn_state = Some((turn_id, next_turn_seq));
            step_seq = 0;
        }
        let turn_id = &turn_state.as_ref().expect("turn state").0;
        step_seq += 1;
        let (kind, content, tool_call_id, tool_name) = encode_step(step);
        tx.execute(
            "INSERT INTO model_steps (step_id, turn_id, seq, kind, content, tool_call_id, tool_name)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                format!("step_{}", random_entry_id()),
                turn_id,
                step_seq,
                kind,
                content,
                tool_call_id,
                tool_name
            ],
        )
        .map_err(|e| e.to_string())?;
        if is_ui_message(step) {
            message_seq += 1;
            tx.execute(
                "INSERT INTO messages (message_id, seq, role, content, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    format!("message_{}", random_entry_id()),
                    message_seq,
                    step.role,
                    step.content.clone().unwrap_or_default(),
                    now
                ],
            )
            .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn next_model_turn_seq(tx: &Transaction<'_>) -> Result<i64, String> {
    let turn_max: Option<i64> = tx
        .query_row("SELECT MAX(seq) FROM model_turns", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;
    let summary_max: Option<i64> = tx
        .query_row("SELECT MAX(replaced_to_seq) FROM summaries", [], |row| {
            row.get(0)
        })
        .map_err(|e| e.to_string())?;
    Ok(turn_max.into_iter().chain(summary_max).max().unwrap_or(0) + 1)
}

fn is_ui_message(step: &Step) -> bool {
    (step.role == "user" || step.role == "assistant")
        && step
            .content
            .as_deref()
            .map(|text| !text.is_empty())
            .unwrap_or(false)
}

fn encode_step(step: &Step) -> (String, String, Option<String>, Option<String>) {
    if step.role == "tool" {
        return (
            "tool_result".into(),
            step.content.clone().unwrap_or_default(),
            step.tool_call_id.clone(),
            step.name.clone(),
        );
    }
    if step.role == "user" {
        return (
            "user".into(),
            step.content.clone().unwrap_or_default(),
            None,
            None,
        );
    }
    if step.tool_calls.is_some() {
        return (
            "tool_call".into(),
            json!({
                "content": step.content,
                "tool_calls": step.tool_calls,
            })
            .to_string(),
            step.tool_call_id.clone(),
            step.name.clone(),
        );
    }
    (
        "assistant".into(),
        step.content.clone().unwrap_or_default(),
        step.tool_call_id.clone(),
        step.name.clone(),
    )
}

fn decode_step(
    kind: &str,
    content: &str,
    tool_call_id: Option<String>,
    tool_name: Option<String>,
) -> Step {
    if kind == "tool_call" {
        if let Ok(Value::Object(map)) = serde_json::from_str::<Value>(content) {
            let text = match map.get("content") {
                Some(Value::String(s)) => Some(s.clone()),
                Some(Value::Null) | None => None,
                Some(other) => Some(other.to_string()),
            };
            return Step {
                role: "assistant".into(),
                content: text,
                tool_call_id,
                tool_calls: map.get("tool_calls").cloned(),
                name: tool_name,
            };
        }
    }
    Step {
        role: match kind {
            "user" => "user",
            "tool_result" => "tool",
            _ => "assistant",
        }
        .into(),
        content: (!content.is_empty()).then(|| content.to_string()),
        tool_call_id,
        tool_calls: None,
        name: tool_name,
    }
}
