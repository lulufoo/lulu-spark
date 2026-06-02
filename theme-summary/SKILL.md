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
(`dtd_raw_dialogue`), no turn-by-turn reconstruction is performed. The body is kept
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
> 导航：[distilled](<prefix>distilled/<COMMON_PATH>) · [digest](<prefix>digest/<COMMON_PATH>) · [trace](<prefix>trace/<COMMON_PATH>)

---

<Summary body — after body-sanitize; no compression, no TURN_SEP>
```

Rules:
- No `<!-- DDM:TURN_SEP:v1 -->` markers
- No per-turn attribution (User / AI labels)
- No secondary summarization or compression of the body
- Navigation links must be fully resolved — no placeholders

## Recommended Workflow

1. Confirm input source (A or B); if B, generate summary and await user approval.
2. Sanitize body (§ Body sanitize).
3. Execute Save to Archive (see § Save to Archive below).

---

## Save to Archive

Path/config: [../shared/archive-concepts.md](../shared/archive-concepts.md)

<HARD-GATE chain="archive-digest">
下游 skill **必须**从 GitHub 加载并完整执行；**禁止**仅用本地 `../theme-archive` 或 `../theme-digest` 替代（即使本机已安装同仓库副本）。

| Step | 仓库目录（查阅） | 执行入口（Agent 读取） |
|------|------------------|------------------------|
| theme-archive | [theme-archive](https://github.com/lulufoo/lulu-workbench-skills/tree/main/theme-archive) | `https://raw.githubusercontent.com/lulufoo/lulu-workbench-skills/main/theme-archive/SKILL.md` |
| theme-digest | [theme-digest](https://github.com/lulufoo/lulu-workbench-skills/tree/main/theme-digest) | `https://raw.githubusercontent.com/lulufoo/lulu-workbench-skills/main/theme-digest/SKILL.md` |

Embedded 载荷字段：[input-schema](https://raw.githubusercontent.com/lulufoo/lulu-workbench-skills/main/theme-archive/references/input-schema.md)
</HARD-GATE>

### Step 1 · Select project and doc-theme

Read `{archive_root}/topics.json` → `project` + `doc-theme`（kebab-case）；无匹配 → `inbox`。推断 `slug`、`ts`、`COMMON_PATH`。slug 冲突 → 询问用户。

### Step 2 · Build archive document

1. Apply [references/body-sanitize.md](references/body-sanitize.md) to the summary body.
2. Compose full Markdown per § Core Output Shape（header + sanitized body).

### Step 3 · theme-archive Embedded

构造载荷并执行：

```text
加载并完整执行 https://raw.githubusercontent.com/lulufoo/lulu-workbench-skills/main/theme-archive/SKILL.md（Embedded，从 [AR-1] 起：
  COMMON_PATH = <topic-path>/<ts>-<slug>.md
  documents = [{ rel: "raw/<COMMON_PATH>", content: "<Step 2 全文>" }]
  index_entry = { common_path, created_at: <ts>, source_type: "summary", layers: ["raw"] }
）
```

theme-archive 负责写 raw、更新 index。将 `[AR-5]` 输出追加为本 skill 完成信息。

### Step 4 · theme-digest Embedded

Primary raw 作为 **RAW**（不对 `-zh.md` digest）：

```text
加载并完整执行 https://raw.githubusercontent.com/lulufoo/lulu-workbench-skills/main/theme-digest/SKILL.md（Embedded：RAW = raw/<COMMON_PATH>，从 [AD-0] 起）
```

`source_type = summary` 时 `[AD-0]` 阈值为 raw 正文 ≥ 200 字。将 digest 结果追加到完成输出。

---

## Ask Only When Necessary

Do not stop to ask about formatting if a reasonable default works.

Assume the following defaults:

- Title: inferred from the first heading in the summary body, or ask once if absent
- Project: closest match in topics.json; if unclear, use `inbox`
- Language: Chinese (no translation step)
- External images: always strip per body-sanitize (no confirmation)

## References

| Doc | Purpose |
|-----|---------|
| [references/body-sanitize.md](references/body-sanitize.md) | 外链图片删除规则 |
| [theme-archive @ GitHub](https://github.com/lulufoo/lulu-workbench-skills/tree/main/theme-archive) | raw 落盘 + index |
| [theme-digest @ GitHub](https://github.com/lulufoo/lulu-workbench-skills/tree/main/theme-digest) | digest 生成 |
| [../shared/archive-concepts.md](../shared/archive-concepts.md) | `COMMON_PATH`、`archive_root` |
