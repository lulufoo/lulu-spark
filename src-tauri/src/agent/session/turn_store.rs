use rusqlite::{params, Connection, OptionalExtension, Transaction};
use crate::services::id::random_entry_id;

use super::step_codec::{decode_step, encode_step, StepRow};
use super::types::Step;

pub(crate) struct StoredStep {
    step_id: String,
    step: Step,
}

pub(crate) fn load_stored_steps(conn: &Connection) -> Result<Vec<StoredStep>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT s.step_id, s.kind, s.content, s.tool_call_id, s.tool_name,
                    s.finish_reason, s.model, s.usage, s.reasoning_content
             FROM model_steps s
             JOIN model_turns t ON t.turn_id = s.turn_id
             ORDER BY t.seq ASC, s.seq ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                StepRow {
                    kind: row.get(1)?,
                    content: row.get(2)?,
                    tool_call_id: row.get(3)?,
                    tool_name: row.get(4)?,
                    finish_reason: row.get(5)?,
                    model: row.get(6)?,
                    usage: row.get(7)?,
                    reasoning_content: row.get(8)?,
                },
            ))
        })
        .map_err(|e| e.to_string())?;
    let mut steps = Vec::new();
    for row in rows {
        let (step_id, step_row) = row.map_err(|e| e.to_string())?;
        steps.push(StoredStep {
            step_id,
            step: decode_step(step_row),
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
        let row = encode_step(step);
        tx.execute(
            "INSERT INTO model_steps (step_id, turn_id, seq, kind, content, tool_call_id, tool_name,
                                      finish_reason, model, usage, reasoning_content)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                format!("step_{}", random_entry_id()),
                turn_id,
                step_seq,
                row.kind,
                row.content,
                row.tool_call_id,
                row.tool_name,
                row.finish_reason,
                row.model,
                row.usage,
                row.reasoning_content
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

pub(crate) fn is_ui_message(step: &Step) -> bool {
    (step.role == "user" || step.role == "assistant")
        && step
            .content
            .as_deref()
            .map(|text| !text.is_empty())
            .unwrap_or(false)
}
