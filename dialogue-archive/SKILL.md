---
name: dialogue-archive
description: >-
  Normalize dialogue via node-range Python script on Cursor JSONL, then sink to
  Workbench MCP (source_path + constrained digest) or local-md under .cache.
  Verbatim — no compression. Use when: dialogue-archive、对话原文归档、
  逐轮归档、同步对话到 raw（原文）. Not for process retrospective — use
  dialogue-summary.
---

# dialogue-archive

> **Read this file in full before executing.** Then load
> [`references/execution.md`](references/execution.md).
>
> 1. **Locate** — transcript + `start_node` / `end_node`
> 2. **Normalize** — script writes raw markdown (never hand-assemble body)
> 3. **Sink** — `workbench` (load note-task, Create) or `local-md` (`.cache` only)

**Not** process summary (`dialogue-summary`). Body stays **verbatim** after mechanical strip — no compression.

MCP / `source_path`: [`references/archive.md`](references/archive.md).

---

## Script Macros

`$SKILL_DIR` = `lulu-workbench-skills` install root (Cursor: `~/.cursor/skills/lulu-workbench-skills`).  
`$WORKSPACE` = active repo root (`--out` under `{workspace}/.cache/…`).

| Macro | Command |
|-------|---------|
| `$NORMALIZE` | `python3 "$SKILL_DIR/dialogue-archive/scripts/dialogue_archive_normalize.py"` |

**Hard:** Prefer `$NORMALIZE` for raw. Do **not** hand-parse jsonl into TURN_SEP. If `$NORMALIZE` is missing, stop — do not fall back to assembling `document` for MCP.

---

## Core Input

| Source | Description |
|--------|-------------|
| A. Current / named session | Cursor agent transcript `.jsonl` (one JSON per non-empty line) |
| B. User paste | Finished TURN_SEP markdown on disk → skip normalize; still path-based sink |

Text anchors (“从「xxx」开始”) → **you** resolve to 1-based node indices (prefer **user** on multi-hit). Script does **not** fuzzy-search.

---

## Sink

Resolve **before** MCP checks.

| `sink` | When | Phase B |
|--------|------|---------|
| `workbench` | Default | Load note-task; route Create. Digest under content constraint when written |
| `local-md` | User intent refuses Workbench persist | Keep normalized md under workspace `.cache`; no MCP; no digest |

Understand intent — do **not** maintain a phrase list. Unclear → default `workbench`.

---

## Hard constraints

1. **Script-only raw** for session transcripts — no hand-built TURN_SEP for MCP.
2. **Path-only `create_note`** — `source_path` only.
3. **Node indices are AI’s job** — script does not search anchors.
4. **Digest needs content_constraint** when writing digest.
5. **MCP only** for notes writes when `sink=workbench`.
6. **Do not invent Host `common_path`** — staging is `{workspace}/.cache/dialogue-archive/<ts>-<slug>.md`. Host assigns the write identity.

---

## Defaults

- Title: from dialogue topic or first user theme
- Project: closest topics match; else `inbox`
- Language: preserve per turn
- `ts`: archive moment (UTC+8 `YYYYMMDDHHMM`) — staging filename only
- `sink`: `workbench`

## References

| Doc | Purpose |
|-----|---------|
| [references/execution.md](references/execution.md) | Locate → Normalize → Sink / receipts |
| [references/archive.md](references/archive.md) | MCP / HARD-GATE / `source_path` |
