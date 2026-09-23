use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde_json::{json, Value};

use crate::services::id::random_entry_id;

use super::types::Turn;

pub(crate) struct StoredStep {
    step_id: String,
    turn: Turn,
}

pub(crate) fn load_steps(conn: &Connection) -> Result<Vec<StoredStep>, String> {
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
            turn: decode_step(&kind, &content, tool_call_id, tool_name),
        });
    }
    Ok(steps)
}

pub(crate) fn load_turns(conn: &Connection) -> Result<Vec<Turn>, String> {
    Ok(load_steps(conn)?.into_iter().map(|step| step.turn).collect())
}

pub(crate) fn sync(
    tx: &Transaction<'_>,
    stored_steps: &[StoredStep],
    current_turns: &[Turn],
    now: i64,
) -> Result<(), String> {
    let shared_turns = stored_steps
        .iter()
        .zip(current_turns)
        .take_while(|(stored, current)| stored.turn == **current)
        .count();
    if shared_turns < stored_steps.len() {
        delete_suffix(tx, stored_steps, shared_turns)?;
    }
    append_turns(tx, &current_turns[shared_turns..], now)
}

fn delete_suffix(
    tx: &Transaction<'_>,
    stored_steps: &[StoredStep],
    shared_turns: usize,
) -> Result<(), String> {
    let retained_messages = stored_steps[..shared_turns]
        .iter()
        .filter(|step| is_ui_message(&step.turn))
        .count() as i64;
    for step in &stored_steps[shared_turns..] {
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

fn append_turns(tx: &Transaction<'_>, turns: &[Turn], now: i64) -> Result<(), String> {
    if turns.is_empty() {
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

    for turn in turns {
        if turn.role == "user" || turn_state.is_none() {
            let next_turn_seq = turn_state.as_ref().map(|(_, seq)| seq + 1).unwrap_or(1);
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
        let (kind, content, tool_call_id, tool_name) = encode_step(turn);
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
        if is_ui_message(turn) {
            message_seq += 1;
            tx.execute(
                "INSERT INTO messages (message_id, seq, role, content, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    format!("message_{}", random_entry_id()),
                    message_seq,
                    turn.role,
                    turn.content.clone().unwrap_or_default(),
                    now
                ],
            )
            .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn is_ui_message(turn: &Turn) -> bool {
    (turn.role == "user" || turn.role == "assistant")
        && turn
            .content
            .as_deref()
            .map(|text| !text.is_empty())
            .unwrap_or(false)
}

fn encode_step(turn: &Turn) -> (String, String, Option<String>, Option<String>) {
    if turn.role == "tool" {
        return (
            "tool_result".into(),
            turn.content.clone().unwrap_or_default(),
            turn.tool_call_id.clone(),
            turn.name.clone(),
        );
    }
    if turn.role == "user" {
        return (
            "user".into(),
            turn.content.clone().unwrap_or_default(),
            None,
            None,
        );
    }
    if turn.tool_calls.is_some() {
        return (
            "tool_call".into(),
            json!({
                "content": turn.content,
                "tool_calls": turn.tool_calls,
            })
            .to_string(),
            turn.tool_call_id.clone(),
            turn.name.clone(),
        );
    }
    (
        "assistant".into(),
        turn.content.clone().unwrap_or_default(),
        turn.tool_call_id.clone(),
        turn.name.clone(),
    )
}

fn decode_step(
    kind: &str,
    content: &str,
    tool_call_id: Option<String>,
    tool_name: Option<String>,
) -> Turn {
    if kind == "tool_call" {
        if let Ok(Value::Object(map)) = serde_json::from_str::<Value>(content) {
            let text = match map.get("content") {
                Some(Value::String(s)) => Some(s.clone()),
                Some(Value::Null) | None => None,
                Some(other) => Some(other.to_string()),
            };
            return Turn {
                role: "assistant".into(),
                content: text,
                tool_call_id,
                tool_calls: map.get("tool_calls").cloned(),
                name: tool_name,
            };
        }
    }
    Turn {
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
