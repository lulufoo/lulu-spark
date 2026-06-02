---
name: theme-summary
description: >-
  Archive a conversation summary directly to raw/ and run digest automatically.
  Use when the user wants to sync a summary note, written conclusion, or
  session recap from a dialogue into the archive. Triggers on: 总结归档、归档总结、
  summary sync、theme-summary、dtd_raw_summary、归档这段总结、同步到 raw。
---

# ThemeSummary

> **Read this file in full before executing.** This skill has 2 mandatory phases:
> 1. **ThemeSummary Generation** (§ Core Input → § Recommended Workflow)
> 2. **Save to Archive** (§ Save to Archive) — 链式 **theme-archive**（raw/index）+ **theme-digest**（digest，不得跳过）。
>
> Both phases are required. Neither may be skipped.

Archive a conversation summary — a written conclusion, recap, or distilled note
produced from a dialogue — directly as a `raw/` document. Unlike `dialogue-summary`
(`dtd_raw_dialogue`), no turn-by-turn reconstruction is performed; the summary
body is stored as-is.

## Core Input

Two accepted input sources (confirm before proceeding):

| Source | Description |
|--------|-------------|
| A. User paste | The user's message contains the complete summary Markdown |
| B. Agent-generated | Agent writes the summary from the current session; **user must confirm before archiving** |

When source is ambiguous, ask the user which applies before continuing.

## Core Output Shape

```markdown
# <Title>

> 创建时间：YYYY年M月D日 HH:MM
> 来源：theme-summary
> 导航：[distilled](<prefix>distilled/<COMMON_PATH>) · [digest](<prefix>digest/<COMMON_PATH>) · [trace](<prefix>trace/<COMMON_PATH>)

---

<Summary body — verbatim from input, no compression, no TURN_SEP>
```

Rules:
- No `<!-- DDM:TURN_SEP:v1 -->` markers
- No per-turn attribution (User / AI labels)
- No secondary summarization or compression of the body
- Navigation links must be fully resolved — no placeholders

## Recommended Workflow

1. Confirm input source (A or B); if B, generate summary and await user approval.
2. Execute Save to Archive (see § Save to Archive below).

---

## Save to Archive

Path/config: [../shared/archive-concepts.md](../shared/archive-concepts.md)

### Step 1 · Select project and doc-theme

Read `{archive_root}/topics.json` → `project` + `doc-theme`（kebab-case）；无匹配 → `inbox`。推断 `slug`、`ts`、`COMMON_PATH`。slug 冲突 → 询问用户。

### Step 2 · Build archive document

按 § Core Output Shape 组完整 Markdown（header + 正文 verbatim）。

### Step 3 · theme-archive Embedded

构造载荷（见 [../theme-archive/references/input-schema.md](../theme-archive/references/input-schema.md)）并执行：

```text
加载并完整执行 ../theme-archive/SKILL.md（Embedded，从 [AR-1] 起：
  COMMON_PATH = <topic-path>/<ts>-<slug>.md
  documents = [{ rel: "raw/<COMMON_PATH>", content: "<Step 2 全文>" }]
  index_entry = { common_path, created_at: <ts>, source_type: "summary", layers: ["raw"] }
）
```

theme-archive 负责写 raw、更新 index。将 `[AR-5]` 输出追加为本 skill 完成信息。

### Step 4 · theme-digest Embedded

Primary raw 作为 **RAW**（不对 `-zh.md` digest）：

```text
加载并完整执行 ../theme-digest/SKILL.md（Embedded：RAW = raw/<COMMON_PATH>，从 [AD-0] 起）
```

`source_type = summary` 时 `[AD-0]` 阈值为 raw 正文 ≥ 200 字。将 digest 结果追加到完成输出。

---

## Ask Only When Necessary

Do not stop to ask about formatting if a reasonable default works.

Assume the following defaults:

- Title: inferred from the first heading in the summary body, or ask once if absent
- Project: closest match in topics.json; if unclear, use `inbox`
- Language: Chinese (no translation step)
