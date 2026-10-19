---
name: spark-auto
description: >-
  Save a knowledge unit to Lulu Spark on the Agent's own initiative when the conversation shows a clear signal; no user command needed.
  Use when: a choice is locked with a reason, an earlier judgment is overturned with evidence, a stated goal is confirmed achieved, a fix passes for a reason other than the first hypothesis, or the user says 记一下 / 原来如此 / 下次注意 / 学到了 / note this / lesson learned / good to know.
---

# spark-auto — Save on Signal

Watch the conversation for the signals below; on a hit, shape one knowledge unit and write it to Lulu Spark through `spark-task`. Done when the write returns `ok` / `id` and the reply reports it.

## Signals

Signal table SSOT. A signal is an observable fact of the conversation; a hit triggers one save.

| # | Signal |
|---|--------|
| 1 | One option is locked out of several, with a stated reason |
| 2 | An earlier judgment that held is overturned with evidence |
| 3 | A goal that was explicitly declared is judged achieved |
| 4 | Debugging: reproduction failed → a change was made → rerun passed, and the cause differs from the first hypothesis |
| 5 | The user says: 记一下 / 原来如此 / 下次注意 / 学到了 (or an equivalent in the conversation's language) |

1. Only conversation facts count; no skill, tool, or workflow event is a signal.
2. One hit → one knowledge unit. Several hits in one turn → one unit each.

## Shape

The signal decides whether to save; the knowledge form decides what to save. Render the matching template.

| Form | Typical signal | Headings |
|------|----------------|----------|
| 决策 Decision | 1 | 选项 / 选择 / 理由 |
| 纠偏 Correction | 2 | 原以为 / 实际 / 依据 |
| 结论 Conclusion | 3 | 目标 / 结果 |
| 排错教训 Debugging lesson | 4 | Problem / What we learned / Takeaways |
| 口头信号 Verbatim note | 5 | 原话, plus one sentence of context |

1. Headings as listed, each an H2; body in the conversation's language, limited to what the signal established.
2. The H1 is the document title; choose it as `spark-task` prescribes.

## Write contract

1. Stage the rendered Markdown as a temporary `.md` outside the user's tracked files.
2. Hand the path to the `spark-task` create flow; it owns the MCP call and document norms.
3. `source_type`: pick a fitting category from `list_doc_categories`.
4. `digest`: `auto`, as `spark-task` prescribes.
5. Omit `workspace_id` and `project_id`: the document lands in the default workspace's default project.
6. Write silently; end the reply with one line: `已存入 Spark：<title>`.
7. When host rules forbid writes without permission, downgrade to `建议存入 Spark：<title>` and wait for the user's confirmation.
8. No `search` before writing in this version.

## Variables

| Variable | Meaning | This version |
|----------|---------|--------------|
| `$SPARK_TARGET_WORKSPACE` | Target workspace for automatic saves | Declared only; not consumed |
| `$SPARK_TARGET_PROJECT` | Target project for automatic saves | Declared only; not consumed |

## Done

- `create_doc` returned `ok` and `id`; and
- the reply ends with the `已存入 Spark：<title>` line, or the `建议存入 Spark：<title>` line when downgraded.
