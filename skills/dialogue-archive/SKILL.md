---
name: dialogue-archive
description: >-
  Normalize dialogue via node-range Python script on Cursor JSONL, then write
  verbatim markdown under workspace .cache. No compression.
  Use when: dialogue-archive、对话原文归档、逐轮归档.
  Not for process retrospective — use dialogue-summary.
---

# dialogue-archive

Write a verbatim node-range transcript to workspace `.cache`. DONE when the
normalized markdown path exists.

> **Read this file in full before executing.** Then load
> [`references/execution.md`](references/execution.md).
>
> 1. **Locate** — transcript + `start_node` / `end_node`
> 2. **Normalize** — script writes raw markdown (never hand-assemble body)
> 3. **Write** — `{workspace}/.cache/dialogue-archive/<ts>-<slug>.md`

**Not** process summary (`dialogue-summary`). Body stays **verbatim** after mechanical strip — no compression.

Write path: [`references/archive.md`](references/archive.md).

---

## Script Macros

`$SKILL_DIR` = `lulu-spark-skills` install root (Cursor: `~/.cursor/skills/lulu-spark-skills`).  
`$WORKSPACE` = active repo root (`--out` under `{workspace}/.cache/…`).

| Macro | Command |
|-------|---------|
| `$NORMALIZE` | `python3 "$SKILL_DIR/dialogue-archive/scripts/dialogue_archive_normalize.py"` |

**Hard:** Prefer `$NORMALIZE` for raw. Do **not** hand-parse jsonl into TURN_SEP. If `$NORMALIZE` is missing, stop.

---

## Core Input

| Source | Description |
|--------|-------------|
| A. Current / named session | Cursor agent transcript `.jsonl` (one JSON per non-empty line) |
| B. User paste | Finished TURN_SEP markdown on disk → skip normalize; confirm the path |

Text anchors (“从「xxx」开始”) → **you** resolve to 1-based node indices (prefer **user** on multi-hit). Script does **not** fuzzy-search.

---

## Hard constraints

1. **Script-only raw** for session transcripts — no hand-built TURN_SEP.
2. **Node indices are AI’s job** — script does not search anchors.
3. Write only to `{workspace}/.cache/dialogue-archive/<ts>-<slug>.md`.

---

## Defaults

- Title: from dialogue topic or first user theme
- Language: preserve per turn
- `ts`: write moment (UTC+8 `YYYYMMDDHHMM`) — filename only

## References

| Doc | Purpose |
|-----|---------|
| [references/execution.md](references/execution.md) | Locate → Normalize → Write / receipt |
| [references/archive.md](references/archive.md) | Path and header |
