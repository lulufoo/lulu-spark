---
name: theme-summary
description: >-
  Legacy: archive an already-written summary note to raw/ + digest (no process
  rewrite). Prefer dialogue-summary for 过程回顾 / dtd_raw_summary / 总结归档
  (process form). Use theme-summary only when the body is already final and
  should be saved verbatim. Triggers: theme-summary、已写好的总结落盘.
---

# ThemeSummary

> **Routing:** 过程回顾 / 决策回顾 / `dtd_raw_summary` → use sibling [`dialogue-summary`](../dialogue-summary/). 本 skill 仅保留「已定稿总结原文落盘」。

> **Read this file in full before executing.** This skill has 2 mandatory phases:
> 1. **ThemeSummary Generation** (§ Core Input → § Recommended Workflow)
> 2. **Save to Archive** (§ Save to Archive) — MCP **archive_document** + **archive_digest**（不得跳过 digest 步骤当 `[AD-0]` 满足时）。
>
> Both phases are required. Neither may be skipped.

Archive a conversation summary — a written conclusion, recap, or distilled note
produced from a dialogue — directly as a `raw/` document. Unlike `dialogue-archive` (verbatim turns) and `dialogue-summary` (process retrospective), this skill does **not** rewrite the body — paste/confirm then archive. The body is kept
verbatim except **external-linked images are stripped** (see § Body sanitize).

## Core Input

Two accepted input sources (confirm before proceeding):

| Source | Description |
|--------|-------------|
| A. User paste | The user's message contains the complete summary Markdown |
| B. Agent-generated | Agent writes the summary from the current session; **user must confirm before archiving** |

When source is ambiguous, ask the user which applies before continuing.

## Body sanitize

<HARD-GATE>
Before Step 2 (Build archive document), MUST apply [references/body-sanitize.md](references/body-sanitize.md) to the summary body. Do not archive raw bodies that still contain `![…](http…)` / `![…](https…)` or external `<img src="http…">`.
</HARD-GATE>

## Core Output Shape

```markdown
# <Title>

> 创建时间：YYYY年M月D日 HH:MM
> 来源：theme-summary
> 导航：[digest](<prefix>digest/<COMMON_PATH>)

---

<Summary body — after body-sanitize; no compression, no TURN_SEP>
```

Rules:
- No `<!-- DDM:TURN_SEP:v1 -->` markers
- No per-turn attribution (User / AI labels)
- No secondary summarization or compression of the body
- Navigation link must be fully resolved — no placeholders
- `COMMON_PATH` = `<project>/<doc-theme>/<ts>-<slug>.md`（Agent 组稿时生成并写入导航行）

## Recommended Workflow

1. Confirm input source (A or B); if B, generate summary and await user approval.
2. Sanitize body (§ Body sanitize).
3. Execute Save to Archive (see § Save to Archive below).

---

## Save to Archive

Path conventions: [../shared/archive-concepts.md](../shared/archive-concepts.md)（`COMMON_PATH`、`prefix`、`slug`、`ts`）

<HARD-GATE mcp="archive">
Workbench App **必须运行**（MCP `workbench-knowledge` 可用）。**禁止**直写 `archive_root` 或链式加载 theme-archive / theme-digest SKILL 落盘。

| Step | MCP tool | 说明 |
|------|----------|------|
| 3 | `archive_document` | 写 raw + index；返回 `id` |
| 4 | `archive_digest` | 传 `id` + digest 全文；写 digest + layers |

MCP 由 Workbench App spawn（`http://127.0.0.1:9876/mcp`）。不可用 → **明确报错并停止**。
</HARD-GATE>

### Step 1 · Select project and doc-theme

推断 `project` + `doc-theme`（kebab-case）；无匹配 → `inbox`。推断 `slug`、`ts`、`COMMON_PATH`。slug 冲突 → 询问用户后重组 document。

**`ts`（仅 GitHub URL）**：若用户输入为 `github.com/{owner}/{repo}/blob/{ref}/{path}` 或 `raw.githubusercontent.com/...`，MUST 在确定 `COMMON_PATH` 前按 [references/ts-inference.md](references/ts-inference.md) 从 Git 历史推断 `ts`（与 `> 创建时间：` 一致）。**非 GitHub 输入**：`ts` = 归档时刻（UTC+8）。

### Step 2 · Build archive document

1. Apply [references/body-sanitize.md](references/body-sanitize.md) to the summary body.
2. Compose full Markdown per § Core Output Shape（header + sanitized body；导航行 **仅含 digest 链接**）。

### Step 3 · archive_document

调用 MCP `archive_document`：

```json
{
  "document": "<Step 2 全文>",
  "source_type": "summary"
}
```

`source_type` 可省略（默认 `summary`）。记录返回的 `id`、`common_path`、`raw_path`。

### Step 4 · archive_digest

当 `[AD-0]` 适用（`theme-digest` 规则：`source_type = summary` 且 raw 正文 ≥ 200 字）：

1. 依据 raw 撰写 digest 全文（`# 标题 — 摘要`、`> 创建时间：`、`## 概述`；见 [theme-digest @ GitHub](https://github.com/lulufoo/lulu-workbench-skills/tree/main/theme-digest) `[AD-1]`–`[AD-2]`）
2. 调用 MCP `archive_digest`：

```json
{
  "id": "<Step 3 返回的 id>",
  "digest": "<完整 digest Markdown>"
}
```

`digest` 已存在时需用户确认后传 `"force": true`。将 MCP 返回追加到完成输出。

---

## Ask Only When Necessary

Do not stop to ask about formatting if a reasonable default works.

Assume the following defaults:

- Title: inferred from the first heading in the summary body, or ask once if absent
- Project: closest match in topics; if unclear, use `inbox`
- Language: Chinese (no translation step)
- External images: always strip per body-sanitize (no confirmation)
- GitHub blob/raw URL: infer `ts` per [references/ts-inference.md](references/ts-inference.md); Git 失败时回退归档时刻

## References

| Doc | Purpose |
|-----|---------|
| [references/body-sanitize.md](references/body-sanitize.md) | 外链图片删除规则 |
| [references/ts-inference.md](references/ts-inference.md) | **仅 GitHub URL**：从 Git 历史推断 `ts` |
| [theme-digest @ GitHub](https://github.com/lulufoo/lulu-workbench-skills/tree/main/theme-digest) | digest 结构与 `[AD-0]` 阈值（撰写规则；落盘由 MCP） |
| [../shared/archive-concepts.md](../shared/archive-concepts.md) | `COMMON_PATH`、`prefix` |
