---
name: dialogue-archive
description: >-
  Normalize dialogue via node-range Python script on Cursor JSONL, then sink to
  Workbench MCP (source_path + constrained digest) or local-md under .cache.
  Verbatim — no compression. Use when: dialogue-archive、对话原文归档、
  dtd_raw_dialogue、逐轮归档、同步对话到 raw（原文）. Legacy alias: former
  workbench dialogue-summary (verbatim). Not for process retrospective — use
  dialogue-summary.
---

# dialogue-archive

> **Read this file in full before executing.** Phases:
> 1. **Locate** — transcript + `start_node` / `end_node` (AI; short params)
> 2. **Normalize** — script writes raw markdown (never hand-assemble body)
> 3. **Sink** — `workbench` (theme-archive Embedded) or `local-md` (`.cache` only)

**Not** process summary (`dialogue-summary`). Body stays **verbatim** after mechanical strip — no compression.

Skill-local steps: [`references/execution.md`](references/execution.md).  
Shared digest shape: [`../shared/digest-workflow.md`](../shared/digest-workflow.md) (plus **内容约束** below).

---

## Script Macros

`$SKILL_DIR` = `lulu-workbench-skills` install root (Cursor: `~/.cursor/skills/lulu-workbench-skills`).  
`$WORKSPACE` = active repo root (must contain `scripts/dialogue_archive_normalize.py`).

| Macro | Command |
|-------|---------|
| `$NORMALIZE` | `python3 "$WORKSPACE/scripts/dialogue_archive_normalize.py"` |

**Hard:** Prefer `$NORMALIZE` for raw. Do **not** hand-parse jsonl into TURN_SEP. If `$NORMALIZE` is missing, stop — do not fall back to assembling `document` for MCP.

Legacy `$TRANSCRIPT_CLEAN` (`transcript-clean-control.py`) is **not** the path for Workbench `archive_document` after this contract; do not use it to build MCP payloads.

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
| `workbench` | Default | theme-archive Embedded + digest under content constraint |
| `local-md` | User intent refuses Workbench persist | Keep normalized md under workspace `.cache`; no MCP; no digest |

Understand intent — do **not** maintain a phrase list. Unclear → default `workbench`.

---

## Core Output Shape

`sink=workbench` (script output):

```markdown
# <Title>

> 创建时间：YYYY年M月D日 HH:MM
> 来源：dialogue-archive
> 导航：[digest](../../../digest/<COMMON_PATH>)

---

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 1）

...

<!-- DDM:TURN_SEP:v1 -->

## AI

...
```

`COMMON_PATH` = `<project>/<doc-theme>/<ts>-<slug>.md`.  
`sink=local-md`: same body is fine; omit digest nav if the script supports it, or leave as-is under `.cache` only.

---

## Workflow

### Locate + Normalize

1. Resolve `sink`.
2. If `sink=workbench`: confirm MCP ([references/archive.md](references/archive.md)). If `local-md`: skip MCP check.
3. Resolve transcript path; resolve `start_node` / `end_node` (closed, 1-based non-empty lines). Default full file when user did not narrow. **Forbid** loading entire jsonl into context — use `rg` / small windows.
4. Choose `title` / `project` / `doc-theme` / `slug` / `--out` → `{workspace}/.cache/dialogue-archive/<ts>-<slug>.md`.
5. Run:

```bash
$NORMALIZE \
  --transcript "<abs.jsonl>" \
  --start-node <N> \
  --end-node <M> \
  --out "<abs.md>" \
  --title "<title>" \
  [--project …] [--doc-theme …] [--slug …]
```

Exit ≠ 0 → stop. Paste path: skip steps 3–5 when user already has TURN_SEP md on disk.

### Sink

**`sink=workbench`**

1. Load and execute [`theme-archive`](../theme-archive/SKILL.md) **Embedded** from `[AR-1]`:

- `primary_path` = normalized `.md`
- `source_type`: `dialogue`
- `content_constraint` required when digest will run

**Forbid:** `"document": "…"`. **Do not** translate or call MCP here.

2. theme-archive `[AR-3]` writes digest when `[AD-0]` applies. Require non-empty **content_constraint**, put in digest header.

Content constraint forms:

- `源节点 71-200` / Turn-range narrative
- One-line focus: `只写 A2 库选型门禁`
- Combined

Digest header **must** include:

```markdown
> 内容约束：<content_constraint 原文>
```

**Forbid:** re-fetch full raw solely for digest; expand past the constraint. Raw coverage = script nodes; digest constraint may be narrower.

```json
{
  "id": "<archive_document id>",
  "digest": "<full digest markdown>"
}
```

Done:

```text
> ✅ dialogue-archive complete（sink=workbench）
> 📄 raw：raw/<COMMON_PATH>
> 📋 digest：digest/<COMMON_PATH>（或已跳过）
> 🧭 nodes：<start>-<end>；约束：<content_constraint 摘要>
```

**`sink=local-md`**

1. Ensure file at `{workspace}/.cache/dialogue-archive/<ts>-<slug>.md`.
2. Do **not** call `archive_document` / `archive_digest`.

Done:

```text
> ✅ dialogue-archive complete（sink=local-md，未上传 workbench）
> 📄 local：.cache/dialogue-archive/<ts>-<slug>.md
```

---

## Hard constraints

1. **Script-only raw** for session transcripts — no hand-built TURN_SEP for MCP.
2. **Path-only `archive_document`** — `source_path` only.
3. **Node indices are AI’s job** — script does not search anchors.
4. **Digest needs content_constraint** when writing digest.
5. **MCP only** for corpus writes when `sink=workbench`.

---

## Defaults

- Title: from dialogue topic or first user theme
- Project: closest topics match; else `inbox`
- Language: preserve per turn
- `ts`: archive moment (UTC+8 `YYYYMMDDHHMM`)
- `sink`: `workbench`

## References

| Doc | Purpose |
|-----|---------|
| [references/execution.md](references/execution.md) | Parent steps / macros |
| [references/archive.md](references/archive.md) | MCP paths / HARD-GATE / source_path |
| [../shared/digest-workflow.md](../shared/digest-workflow.md) | Digest `[AD-0]`–`[AD-3]` |
