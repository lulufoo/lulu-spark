---
name: dialogue-summary
description: >-
  Normalize the current dialogue into raw/ and generate digest automatically via MCP.
  Use when: dialogue-summary、对话归档、对话整理、dtd_raw_dialogue、蒸馏归档、
  归档这段对话、同步对话到 raw。
---

# DialogueSummary

> **Read this file in full before executing.** This skill has 2 mandatory phases:
> 1. **Dialogue Normalization** (§ Core Input → § Recommended Workflow)
> 2. **Save to Archive** (§ Save to Archive) — MCP **archive_document** + **archive_digest**（当 `[AD-0]` 满足时不得跳过 digest）。
>
> Both phases are required. Neither may be skipped.

Normalize the current session (or user-provided dialogue) into a turn-separated `raw/`
document. Body text is kept verbatim except stripping AI折叠思考块 / 无关前缀；不得压缩或摘要化。

## Core Input

| Source | Description |
|--------|-------------|
| A. Current session | All turns in the active chat |
| B. User paste | User provides a complete dialogue document |

When ambiguous, treat as current session unless the user explicitly pasted a document.

## Core Output Shape

```markdown
# <总标题>

> 创建时间：YYYY年M月D日 HH:MM
> 来源：dialogue-summary
> 导航：[digest](<prefix>digest/<COMMON_PATH>)

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
- Body matches source dialogue verbatim (after strip rules); no compression or summarization
- Navigation link must be fully resolved — **digest only**; no placeholders
- `---` separator required before turn body (MCP archive parser)
- `COMMON_PATH` = `<project>/<doc-theme>/<ts>-<slug>.md`（Agent 组稿时生成并写入导航行）
- `prefix` = `../../../`（topic-path 固定 2 段）

## Recommended Workflow

1. Confirm Workbench MCP is available（§ Save to Archive HARD-GATE）.
2. Select project and doc-theme（§ Save to Archive Step 1）.
3. Build normalized document（§ Save to Archive Step 2）.
4. Execute Save to Archive（§ Save to Archive Step 3–4）.

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

推断 `project` + `doc-theme`（kebab-case，英文，无空格，**3–5 个单词**）；无匹配 → `inbox`。

```
project   = 语义最接近的 topics 项；不清楚 → inbox
doc-theme = 语义推断（如 agentic-coding-discipline）
slug      = 与主题一致的 kebab-case（冲突先澄清）
ts        = YYYYMMDDHHMM（东八区 UTC+8，归档时刻）
COMMON_PATH = <project>/<doc-theme>/<ts>-<slug>.md
```

创建时间：由 `ts` 换算为 `YYYY年M月D日 HH:MM`（月、日不补零，时、分两位）。

### Step 2 · Build archive document

- input  : 当前对话全部轮次（或用户粘贴文档）
- rule   : 正文与原始对话逐字一致；仅可加分隔符 / 标题 / 去格式噪音
- 剥离   : AI 推导性独白（折叠思考块 / 无关前缀句）；讲解形式的推理保留
- 禁止   : 压缩 / 改写 / 摘要化 / `{{…}}` 占位符 / 导航行占位符

Compose full Markdown per § Core Output Shape.

### Step 3 · archive_document

调用 MCP `archive_document`：

```json
{
  "document": "<Step 2 全文>",
  "source_type": "dialogue"
}
```

记录返回的 `id`、`common_path`、`raw_path`。

### Step 4 · archive_digest

当 `[AD-0]` 适用（`theme-digest` 规则：通常对话含 2+ Turn 块即满足）：

1. 依据 raw 撰写 digest 全文（`# 标题 — 摘要`、`> 创建时间：`、`## 概述`；见 [theme-digest @ GitHub](https://github.com/lulufoo/lulu-workbench-skills/tree/main/theme-digest) `[AD-1]`–`[AD-2]`）
2. 调用 MCP `archive_digest`：

```json
{
  "id": "<Step 3 返回的 id>",
  "digest": "<完整 digest Markdown>"
}
```

`digest` 已存在时需用户确认后传 `"force": true`。将 MCP 返回追加到完成输出。

完成汇总：

```
> ✅ dialogue-summary 完成
> 📄 raw：raw/<COMMON_PATH>
> 📋 digest：digest/<COMMON_PATH>（或「已跳过」）
```

---

## Ask Only When Necessary

Assume defaults:

- Title: inferred from dialogue topic or first user message theme
- Project: closest match in topics; if unclear, use `inbox`
- Language: preserve source language per turn (typically Chinese)
- `ts`: archive moment (UTC+8)

## References

| Doc | Purpose |
|-----|---------|
| [theme-digest @ GitHub](https://github.com/lulufoo/lulu-workbench-skills/tree/main/theme-digest) | digest 结构与 `[AD-0]` 阈值（撰写规则；落盘由 MCP） |
| [../shared/archive-concepts.md](../shared/archive-concepts.md) | `COMMON_PATH`、`prefix` |
