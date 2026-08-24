# Title Naming (shared)

> Shared SSOT for human-facing titles produced by `dialogue-summary` and
> `todo-task`. File slugs and archive paths remain owned by their respective
> path contracts.

## Core shape

```text
{Domain} {Topic}：{Focus}
```

| Segment | Rule |
|---------|------|
| **Domain** | Scannable system, product, or subject area. |
| **Topic** | Stable problem area; use a noun phrase, not a date, version, or work action. |
| **Separator** | Use one full-width `：`. |
| **Focus** | The distinctive cut, tension, conclusion, or object that makes this title different from neighboring documents. |

The title names the **subject and its distinctive focus**, not the act of
writing about it.

## Global gates

Every generated or proposed title must pass all of these:

1. **Mainline-first** — name the document's central line, not its entry point,
   chapter list, or every model/example mentioned.
2. **Context-free** — a reader without the originating chat can identify the
   domain, stable topic, and concrete focus.
3. **Natural reading** — the whole title reads as normal language, not four
   template fragments glued together.
4. **No semantic repetition** — do not repeat generic heads across segments
   (`能力分析：使用分析`, `问题分析：缺口分析`).
5. **No TOC packing** — do not join unrelated chapter labels with commas merely
   to achieve coverage.
6. **Specific over generic** — avoid bare titles such as `问题分析`,
   `方案讨论`, `模型总结`, or slogan-only conclusions.
7. **Concise after complete** — first make the title self-contained, then remove
   words that do not change identification.

## Document profile

Use for dialogue summaries, articles, archive documents, and other
reader-facing Markdown.

```text
{Domain} {Topic}：{Focus}
```

- `Focus` should express the document's central distinction or settled scope.
- Do not append `分析`, `总结`, or `研究` merely to announce the document genre.
  Keep such a word only when it distinguishes this artifact from another
  artifact with the same subject.
- A source post, incident, or person belongs in the title only when that source
  remains the document's subject. If it merely opened the discussion, omit it.

```text
Good: AI 模型协作：判断缺口与使用分工
Bad:  Theo 模型榜、5 系乱顶、Sol 膨胀与四个工位
Bad:  AI 能力分析：模型缺口与使用分析
```

## Todo profile

Todo titles need an explicit work type for queue scanning. Keep it visibly
separate from the semantic title:

```text
{Domain} {Topic}：{Focus}（{WorkType}）
```

All four semantic segments plus the separator are required unless the user
explicitly supplies the title.

Closed `WorkType` set:

```text
效果分析 | 问题描述与分析 | 选型分析 | 对比分析 | 方案 |
缺陷修复 | 流程优化 | 执行待办 | 待优化计划
```

```text
Good: Compose 写入节奏控制：Init 逐章写入（效果分析）
Good: AI 模型协作：判断缺口与使用分工（选型分析）
Bad:  AI 能力分析：模型缺口与使用分析
```

`Domain` must not be only a bare gate code (`G3`, `RS`) or internal nickname.
Necessary jargon is allowed in `Focus` only when the preceding segments make it
readable.

## Self-check

Without chat context, can a reader answer:

1. What system, product, or subject is this about?
2. What stable problem area does it belong to?
3. What concrete distinction makes this document worth opening?
4. Does the title read naturally without repeated or glued-on words?
5. For Todo only: is the work type explicit and from the closed set?

Any “no” means rewrite before writing or creating the artifact.

## Length and user override

- Todo field limits come from the live MCP/Host schema. If over budget, compress
  in order: `Focus` → `Topic` → `Domain`; do not drop `WorkType`.
- Document titles follow the owning artifact's length constraints.
- A user-explicit title overrides the generated-title profile. Use it as given
  unless the user asks for naming review or optimization; then evaluate it
  against the applicable profile.
