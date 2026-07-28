---
name: dialogue-archive
description: >-
  Normalize the current dialogue into turn-separated markdown. Default sink is
  Workbench MCP (raw/ + digest). Intent not to persist to Workbench → sink=local-md
  (write markdown under workspace .cache only). Verbatim — no compression.
  Use when: dialogue-archive、对话原文归档、dtd_raw_dialogue、逐轮归档、同步对话到 raw（原文）.
  Legacy alias: former workbench dialogue-summary (verbatim). Not for process
  retrospective — use dialogue-summary. If both this and an old workbench
  “dialogue-summary” exist, use this name for 原文归档.
---

# dialogue-archive

> **Read this file in full before executing.** Two phases:
> 1. **Phase A — Normalize** (mechanical clean + sub-agent metadata / render)
> 2. **Phase B — Sink** (`workbench` or `local-md`)

**Not** process summary (`dialogue-summary`). Body stays **verbatim** after chrome strip — no compression.

Orchestration SSOT: [`../shared/dialogue-execution.md`](../shared/dialogue-execution.md).  
Clean / render SSOT: [`../shared/transcript-clean.md`](../shared/transcript-clean.md).  
Skill-local steps: [`references/execution.md`](references/execution.md).

---

## Script Macros

`$SKILL_DIR` = `lulu-workbench-skills` install root (Cursor: `~/.cursor/skills/lulu-workbench-skills`).

| Macro | Command |
|-------|---------|
| `$TRANSCRIPT_CLEAN` | `python3 "$SKILL_DIR/scripts/transcript-clean-control.py"` |

---

## Core Input

| Source | Description |
|--------|-------------|
| A. Current session | Resolve jsonl → `$TRANSCRIPT_CLEAN from-jsonl` → clean-raw |
| B. User paste | Finished TURN_SEP markdown; skip clean + worker |

---

## Sink

Resolve **before** Phase A MCP checks.

| `sink` | When | Phase B |
|--------|------|---------|
| `workbench` | Default | MCP `archive_document` + digest when applicable |
| `local-md` | User intent is not to save / upload / persist into Workbench | Write archive markdown under workspace `.cache`; no MCP; no digest |

Understand intent — do **not** maintain a phrase list. If unclear whether Workbench persist is refused, default `workbench`.

---

## Core Output Shape

`sink=workbench`:

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

`sink=local-md`: same body; **omit** the digest navigation line; optionally add `> 落点：local-md`.

`COMMON_PATH` = `<project>/<doc-theme>/<ts>-<slug>.md`.  
Render **only** via `$TRANSCRIPT_CLEAN to-archive-md` (verbatim `u`/`a`).  
`local-md`: pass `--omit-digest-nav`; `--out` → `{workspace}/.cache/dialogue-archive/<ts>-<slug>.md`.

---

## Workflow

### Phase A — Normalize

1. Resolve `sink` (see Sink).
2. If `sink=workbench`: confirm Workbench MCP ([references/archive.md](references/archive.md)). If `sink=local-md`: **skip** MCP prerequisite.
3. Resolve session jsonl; run `$TRANSCRIPT_CLEAN from-jsonl` → `{workspace}/.cache/dialogue-archive-<sid>-clean-raw.json`. Fail closed on chrome residue / zero turns.
4. Dispatch **Grok** worker per [`references/execution.md`](references/execution.md) (default; do not re-ask). Worker infers title / project / doc-theme / slug / ts; runs `to-archive-md --omit-empty-ai` (and `--omit-digest-nav` when `sink=local-md`); does **not** rewrite turn text; does **not** MCP-archive.
5. Parent takes archive markdown path from worker receipt.

Paste path: skip steps 3–4 when user pasted a complete document. For pasted docs under `local-md`, strip digest nav (or ensure header matches local-md shape) before Done.

### Phase B — Sink

**`sink=workbench`**

1. `archive_document` with `source_type: "dialogue"`.
2. When digest applies (≥2 Turn blocks): overview per [`../shared/digest-workflow.md`](../shared/digest-workflow.md); `archive_digest`.

```json
{ "document": "<full markdown>", "source_type": "dialogue" }
```

Done:

```text
> ✅ dialogue-archive complete（sink=workbench）
> 📄 raw：raw/<COMMON_PATH>
> 📋 digest：digest/<COMMON_PATH>（或已跳过）
```

**`sink=local-md`**

1. Ensure archive markdown is at `{workspace}/.cache/dialogue-archive/<ts>-<slug>.md`.
2. Do **not** call `archive_document` / `archive_digest`.

Done:

```text
> ✅ dialogue-archive complete（sink=local-md，未上传 workbench）
> 📄 local：.cache/dialogue-archive/<ts>-<slug>.md
```

---

## Hard constraints

1. **No hand extract** — Agent MUST NOT parse jsonl or assemble TURN_SEP by hand.
2. **Verbatim** — `to-archive-md` must not summarize or paraphrase `u`/`a`.
3. **MCP only** for corpus writes when `sink=workbench` ([`../shared/archive-concepts.md`](../shared/archive-concepts.md)). `local-md` writes workspace `.cache` only — never hand-write corpus `raw/` / `digest/` / `index.json`.

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
| [references/execution.md](references/execution.md) | Parent / worker / macros |
| [references/archive.md](references/archive.md) | MCP paths / HARD-GATE |
| [../shared/transcript-clean.md](../shared/transcript-clean.md) | Clean + to-archive-md |
| [../shared/dialogue-execution.md](../shared/dialogue-execution.md) | Shared parent/worker spine |
| [../shared/digest-workflow.md](../shared/digest-workflow.md) | Digest `[AD-0]`–`[AD-3]` |
