# Summary Writing

> Turn confirmed topics into a concise, standalone document without redesigning
> the spine.

## Input

Compose from the bound writing state.

| Value | Requirement |
|---|---|
| `$FEEDSTOCK` | Same evidence used for topic confirmation |
| `$CONFIRM` | Confirmed scope, reader, mode, and spine |
| `$CORE_TOPICS` | Internal emphasis only |
| `$DRAFT` | Absolute cache output path |
| Header values | Title, 创建时间, 来源 |
| Owning document | Optional; used by `design-pointer` |

The confirmed spine fixes topic selection and order. A structural defect stops
writing and returns to topic confirmation.

## Document frame

Set the reader-facing title, header, and opening.

1. Title: user-given as given. Otherwise name the process subject and its
   distinctive focus — not the chat, the source, or a chapter list.
2. Write the header below.
3. Start with the first substantive topic. Supply only the context later topics
   need; do not add a mandatory `0. 读前说明`.

```markdown
# <title>

> 创建时间：<UTC+8 datetime>
> 来源：<dialogue/session label>

---
```

## Body contract

Write natural article prose in confirmed spine order.

1. Make each substantive topic's judgment, decisive reasoning or evidence, and
   relevant boundary reconstructable.
2. Preserve distinctions, meaningful rejects, and defeaters supplied by the
   dialogue; never invent missing support or outcomes.
3. Give each conclusion one owning place and avoid turn-by-turn replay.
4. Use `$CORE_TOPICS` to vary depth: expand main-line and selected paths;
   compress non-core and rejected paths without losing their rationale.
5. Under `design-pointer`, explain only the mechanism needed for the judgment
   and point to the owning document for implementation detail.
6. Attach links to the facts they support, state material uncertainty in place,
   and use `〔User〕` only for a stance that changed the spine, conclusion, or
   hard boundary.

## Output

Write the complete Markdown to `$DRAFT` and return:

```text
title: <H1>
body_path: <absolute .md path>
gate: pass | fail
unresolved:
  - <item, only when present>
```

Return the path, not the full body, to the parent.

## Acceptance

The writing gate passes only when:

1. title and header are present;
2. body structure and emphasis match the confirmed state;
3. important judgments, reasons, rejects, and boundaries remain reconstructable
   without the original dialogue; and
4. the document contains no duplicated conclusion, transcript replay, invented
   support, or replacement implementation document.
