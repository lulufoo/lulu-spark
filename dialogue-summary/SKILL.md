---
name: dialogue-summary
description: >-
  将当前或指定 Cursor 对话整理为可独立阅读的过程总结，并按需归档。
  Use when: 对话总结、总结当前对话、总结这个 session、dialogue summary、
  将讨论整理成文档。
argument-hint: '[Turn X～Y | topic filter] [--no-archive]'
---

# dialogue-summary

Produce a standalone account of how a dialogue reached its important
judgments. DONE means the user-confirmed spine has become a validated draft and
the requested local or archive handoff has succeeded.

## Boundary

This entry owns phase order, user gates, and handoffs.

1. Topic selection and writing rules live in the two local references.
2. Transcript cleaning, naming, and archiving remain with their owning modules.
3. The summary records reasoning and convergence; it is neither a transcript
   nor a replacement implementation document.

## Script Macros

| Macro | CLI |
|---|---|
| `$TRANSCRIPT_CLEAN` | `python3 "$SKILL_DIR/scripts/transcript-clean-control.py"` |

## Session state

| Variable | Meaning |
|---|---|
| `$FEEDSTOCK` | Clean dialogue input used by both authoring phases |
| `$CONFIRM` | User-facing confirmed scope and spine |
| `$CORE_TOPICS` | Internal emphasis marks |
| `$SINK` | `archive` (default) or `local` |
| `$DRAFT` | Validated summary Markdown path |

## Phase 0 — Bind input

Resolve one feedstock source.

1. **Cursor session:** load
   [`references/transcript-clean.md`](references/transcript-clean.md), run
   `$TRANSCRIPT_CLEAN from-jsonl`, and bind `$FEEDSTOCK` from its `written`
   output. A non-zero result stops the run.
2. **Supplied dialogue/Markdown:** bind it directly as `$FEEDSTOCK`; do not
   expand it from the session transcript.

## Phase 1 — Confirm topics

Establish the document spine before prose exists.

1. Load
   [`references/topic-confirmation.md`](references/topic-confirmation.md).
2. Produce `$CONFIRM` and internal `$CORE_TOPICS` from `$FEEDSTOCK` and the
   requested scope.
3. Show only `$CONFIRM`, then stop for explicit user confirmation.
4. If the user edits the scope or spine, rebuild both values and confirm again.

## Phase 2 — Write

Compose only from confirmed state.

1. Bind `$SINK=local` for `--no-archive`; otherwise bind `$SINK=archive`.
2. For archive output, write title / 创建时间 / 来源 in the draft header.
   Do not invent a notes path. Local output: H1 only.
3. Load [`references/summary-writing.md`](references/summary-writing.md).
4. Write to an absolute cache path and bind it as `$DRAFT`, using `$FEEDSTOCK`,
   `$CONFIRM`, `$CORE_TOPICS`, `$SINK`, any owning-document pointer, and the
   archive header values when applicable.
5. A failed writing gate stops the run. A structural defect returns to Phase 1.

## Phase 3 — Deliver

Finish through the selected sink.

1. **Local:** return `$DRAFT`.
2. **Archive:** load note-task and route Create with `$DRAFT`,
   `source_type=summary`, and the constraint that digest must not reconstruct
   omitted dialogue or implementation detail.
3. Archive failure reports the owning phase and stops; do not call archive MCP
   tools directly as a fallback.

## Completion

Report only observable results:

```text
> ✅ DialogueSummary 完成
> 🧭 scope：<confirmed scope>
> 🧱 topics：<count>
> 📄 draft：<absolute path>
> 🗂 archive：<note-task result | skipped>
```
