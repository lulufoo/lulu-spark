# Handoff → theme-archive

Load after `$TRANSCRIBE_CTL route`. Follow [`theme-archive`](../../theme-archive/SKILL.md) Embedded. Do not translate here.

## When

- Default: after `route` stdout `ok` is true.
- Skip: user says 不归档 / skip archive / local only.

## Payload

| Field | Source |
|-------|--------|
| Primary | `route` stdout `primary` (add archive header if missing) |
| `source_type` | `route` stdout `source_type` (`dialogue` or `transcript`) |
| `skip_translate` | only when the user forbids Chinese |

Do not pass `translations`. `content_constraint` is required when `source_type` is `dialogue`.

## Digest

theme-archive `[AR-3]` owns digest. Theme-transcribe does not call `create_note_digest`.

## Revisions

New archive needs a new `ts` / path. `create_note` does not overwrite `raw/`.
