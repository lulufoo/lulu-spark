---
name: spark-auto
description: >-
  Save one knowledge unit to Lulu Spark on its own when a host-bound `$SPARK_AUTO_SIGNALS` signal appears in the conversation.
  Use when: a bound signal appears; no user command needed.
---

# spark-auto — Save on Signal

Turn each signal hit into one knowledge unit saved through `spark-task`. Done when the unit lands where the host points.

## Variables

| Variable | Binding | Role |
|----------|---------|------|
| `$SPARK_AUTO_SIGNALS` | Host-bound; required | The signal set. Unbound → no automatic save |
| `$SPARK_TARGET_WORKSPACE` | Host-bound; optional | `workspace_id` of the save. Unbound → not passed |
| `$SPARK_TARGET_PROJECT` | Host-bound; optional | `project_id` of the save. Unbound → not passed |

## Signal

1. An observable fact of the conversation; never a skill, tool, or workflow event.
2. One hit → one knowledge unit holding only what the hit established.

## Save

`spark-task` create flow, with `source_type` fitting the unit and the target ids from Variables. Then end the reply with one line: `已存入 Spark：<title>`.

## Done

The result's `workspace_id` / `project_id` match each bound variable.
