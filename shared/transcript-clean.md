# Transcript clean (shared)

Mechanical Cursor transcript → `clean-raw.json` for `dialogue-summary` and `dialogue-archive`.

**SSOT scripts:** `$SKILL_DIR/scripts/transcript-clean-control.py` (+ sibling `transcript-clean-schema.py`).  
Do **not** call schema from SKILL. Do **not** hand-parse jsonl in the agent.

`$SKILL_DIR` = platform install of `lulu-workbench-skills` (Cursor: `~/.cursor/skills/lulu-workbench-skills`).

---

## Macro

```bash
$TRANSCRIPT_CLEAN = python3 "$SKILL_DIR/scripts/transcript-clean-control.py"
```

| Subcommand | Purpose |
|------------|---------|
| `from-jsonl` | Cursor agent-transcript jsonl → clean-raw.json |
| `from-raw-md` | Messy TURN_SEP / User·AI markdown → clean-raw.json |
| `to-archive-md` | clean-raw.json → dialogue-archive Core Output Shape markdown (`--omit-empty-ai` skips empty assistant turns; `--omit-digest-nav` for sink=local-md) |

Contracts: script `--help` / module docstring.

---

## Hard rules

1. For session/jsonl input: run `$TRANSCRIPT_CLEAN from-jsonl` **before** any worker compose or archive render.
2. Workers **MUST** use only the clean-raw path (+ user scope notes). **MUST NOT** re-read raw jsonl for extract/compose.
3. Fail closed when stdout reports `chrome_tags_remaining` non-empty or `user_turns == 0` (non-zero exit).
4. `to-archive-md` is **verbatim** on cleaned `u`/`a` — structure + header only; no summarization.
5. Assistant clean drops English thinking / process paragraphs and previous-turn echoes (see `clean_assistant_text` / `_drop_prev_turn_echo` in schema).
6. Optional turn window: `from-jsonl --turn-from N --turn-to N` (inclusive; renumbered 1..K after slice).

---

## clean-raw shape

```json
{
  "sid": "<session-id>",
  "title": "<string>",
  "src": "<source path>",
  "turns": [{ "n": 1, "u": "...", "a": "...", "ts": "..." }]
}
```

Default out paths (caller supplies `--out`):

| Skill | Suggested `--out` |
|-------|-------------------|
| dialogue-summary | `{workspace}/.cache/dialogue-summary-<sid>-clean-raw.json` |
| dialogue-archive | `{workspace}/.cache/dialogue-archive-<sid>-clean-raw.json` |

---

## Typical invocations

```bash
$TRANSCRIPT_CLEAN from-jsonl \
  --session-id "$SESSION_ID" \
  --jsonl "$ABS_JSONL" \
  --out "$ABS_CLEAN_RAW" \
  --title "$TITLE"

$TRANSCRIPT_CLEAN to-archive-md \
  --clean-raw "$ABS_CLEAN_RAW" \
  --out "$ABS_ARCHIVE_MD" \
  --title "$TITLE" \
  --project "$PROJECT" \
  --doc-theme "$DOC_THEME" \
  --slug "$SLUG" \
  --ts "$TS"
```

Paste path (user supplies finished markdown): skip `from-jsonl`; archive may go straight to MCP.
