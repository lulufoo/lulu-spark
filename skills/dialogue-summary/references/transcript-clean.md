# Transcript clean

Mechanical Cursor transcript → `clean-raw.json` for **dialogue-summary** only.

Script: `$TRANSCRIPT_CLEAN` in [`../SKILL.md`](../SKILL.md). Do not call `transcript-clean-schema.py`. Do not hand-parse jsonl. Subcommands: `$TRANSCRIPT_CLEAN --help`.

## When

Cursor session input: run `$TRANSCRIPT_CLEAN from-jsonl` before topic confirmation or writing. Bind `$FEEDSTOCK` from stdout `written`. Exit non-zero, `chrome_tags_remaining`, or `user_turns==0` → stop.

Pasted Markdown: skip this file; bind `$FEEDSTOCK` to that path.

Workers use only the clean-raw path. Do not re-read raw jsonl to compose.

Optional window: `from-jsonl --turn-from N --turn-to N` (inclusive; renumbered 1..K).

## Out path

`{workspace}/.cache/dialogue-summary-<sid>-clean-raw.json`

## clean-raw shape

```json
{
  "sid": "<session-id>",
  "title": "<string>",
  "src": "<source path>",
  "turns": [{ "n": 1, "u": "...", "a": "...", "ts": "..." }]
}
```
