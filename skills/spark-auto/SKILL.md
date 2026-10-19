---
name: spark-auto
description: >-
  Save to Lulu Spark when a `$SPARK_AUTO_SIGNALS` signal appears in the conversation; the host rules define the signals.
  Use when: a bound `$SPARK_AUTO_SIGNALS` signal appears; no user command needed.
---

# spark-auto — Save on Signal

Watch the conversation for the host-bound signals; on a hit, shape one knowledge unit and write it to Lulu Spark through `spark-task`. Done when the write returns `ok` / `id` and the reply reports it.

## Signals

`$SPARK_AUTO_SIGNALS` is the signal set. This SKILL defines none.

1. The host rules bind `$SPARK_AUTO_SIGNALS`; unbound → no automatic save.
2. A signal is an observable fact of the conversation; no skill, tool, or workflow event is a signal.
3. One hit → one knowledge unit. Several hits in one turn → one unit each.

## Shape

The signal decides whether to save; the knowledge form decides what to save. Pick the form that fits what the hit established and render its template.

| Form | Headings |
|------|----------|
| 决策 Decision | 选项 / 选择 / 理由 |
| 纠偏 Correction | 原以为 / 实际 / 依据 |
| 结论 Conclusion | 目标 / 结果 |
| 排错教训 Debugging lesson | Problem / What we learned / Takeaways |
| 口头信号 Verbatim note | 原话, plus one sentence of context |

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
| `$SPARK_AUTO_SIGNALS` | The signal set that triggers a save | Host-bound; required |
| `$SPARK_TARGET_WORKSPACE` | Target workspace for automatic saves | Declared only; not consumed |
| `$SPARK_TARGET_PROJECT` | Target project for automatic saves | Declared only; not consumed |

## Done

- `create_doc` returned `ok` and `id`; and
- the reply ends with the `已存入 Spark：<title>` line, or the `建议存入 Spark：<title>` line when downgraded.
