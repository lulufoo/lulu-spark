# Handoff → note-task

Load after `$TRANSCRIBE_CTL route`. Load note-task and route Create. Need `-zh.md`: run `$SKILL_DIR/note-task/scripts/detect_full_english.py` then `check_zh_parity.py`, pass `translations`.

## When

- Default: after `route` stdout `ok` is true.
- Skip: user says 不归档 / skip archive / local only.

## Payload

| Field | Source |
|-------|--------|
| Primary | `route` stdout `primary` (add archive header if missing) |
| `source_type` | `route` stdout `source_type` (`dialogue` or `transcript`) |
| Chinese companion | only when the caller translated first (`translations` on `create_note`) |

`content_constraint` is required in `digest_body` when `source_type` is `dialogue`.

## Digest

`digest` / `digest_body` are `create_note` fields. Theme-transcribe does not call a digest-only tool.

## Revisions

New archive needs a new staging `ts` / path. `create_note` does not overwrite `raw/`.
