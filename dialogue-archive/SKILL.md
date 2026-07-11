---
name: dialogue-archive
description: >-
  Normalize the current dialogue into turn-separated raw/ and digest via Workbench MCP.
  Verbatim preservation — no compression. Use when: dialogue-archive、对话原文归档、
  dtd_raw_dialogue、逐轮归档、同步对话到 raw（原文）.
  Legacy alias: former workbench dialogue-summary (verbatim). Not for process
  retrospective — use dialogue-summary. If both this and an old workbench
  “dialogue-summary” exist, use this name for 原文归档.
---

# dialogue-archive

> **Read this file in full before executing.** Two mandatory phases:
> 1. **Dialogue normalization** (verbatim turns)
> 2. **Save to Archive** — MCP `archive_document` + `archive_digest` when applicable

**Renamed from** the former verbatim `dialogue-summary`. Process understanding → `dialogue-summary` (sibling skill in this repo).

Normalize the current session (or user-pasted dialogue) into a turn-separated `raw/` document. Body stays **verbatim** except stripping AI折叠思考块 / irrelevant prefixes. **No** compression or summarization.

This `SKILL.md` + `references/` are self-contained. Do **not** require files under `dialogue-summary/`.

---

## Core Input

| Source | Description |
|--------|-------------|
| A. Current session | All turns in the active chat |
| B. User paste | User provides a complete dialogue document |

When ambiguous, treat as current session unless the user explicitly pasted a document.

---

## Core Output Shape

```markdown
# <总标题>

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

Rules:

- `<!-- DDM:TURN_SEP:v1 -->` between turns; `## User（Turn N）` / `## AI` headings
- Body matches source dialogue verbatim (after strip rules)
- Navigation: digest only; fully resolved; no placeholders
- `---` required before turn body (MCP parser)
- `COMMON_PATH` = `<project>/<doc-theme>/<ts>-<slug>.md`

---

## Workflow

1. Confirm Workbench MCP available ([references/archive.md](references/archive.md)).
2. Select project and doc-theme (kebab-case; else `inbox`).
3. Build normalized document per § Core Output Shape.
4. `archive_document` with `source_type: "dialogue"`.
5. When digest applies (typically 2+ Turn blocks): write digest overview; `archive_digest`.

### Build rules

- Input: all in-scope turns (or paste)
- Keep wording; only add separators / titles / strip format noise
- Strip: AI折叠思考块 / irrelevant prefix lines; keep explanatory reasoning that is part of the reply
- Forbid: compress / rewrite / summarize / `{{…}}` placeholders / nav placeholders

### archive_document

```json
{
  "document": "<full markdown>",
  "source_type": "dialogue"
}
```

Record `id`, `common_path`, `raw_path`.

### archive_digest

When applicable, draft digest (`# 标题 — 摘要`, `> 创建时间：`, `## 概述`) per [theme-digest](https://github.com/lulufoo/lulu-workbench-skills/tree/main/theme-digest) `[AD-1]`–`[AD-2]`, then:

```json
{
  "id": "<archive_document id>",
  "digest": "<full digest markdown>"
}
```

Existing digest → confirm then `"force": true`.

Done:

```text
> ✅ dialogue-archive complete
> 📄 raw：raw/<COMMON_PATH>
> 📋 digest：digest/<COMMON_PATH>（或已跳过）
```

---

## Defaults

- Title: from dialogue topic or first user theme
- Project: closest topics match; else `inbox`
- Language: preserve per turn
- `ts`: archive moment (UTC+8)

## References

| Doc | Purpose |
|-----|---------|
| [references/archive.md](references/archive.md) | MCP paths / HARD-GATE |
| [theme-digest @ GitHub](https://github.com/lulufoo/lulu-workbench-skills/tree/main/theme-digest) | digest structure / `[AD-0]` |
