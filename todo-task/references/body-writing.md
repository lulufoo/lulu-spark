# Body Writing (todo_md)

SSOT for `todo_md` body structure under Todo Norms — primarily for WorkType
`问题描述与分析` / `效果分析`.

## Weight

Problem and Analysis carry the bulk of the content. Optimization direction
stays a short outline — the full contract belongs in the archive design doc,
not the todo body.

| Section | Weight | Content |
|---------|--------|---------|
| **1. Problem** | Heavy | Cause and effect: scenario, trigger, consequence, current gap |
| **2. Analysis** | Heavy | Root-cause mechanism, what was checked (code read, commands run, facts compared), why it breaks |
| **3. Optimization direction** | Brief | A short bulleted outline of direction/principle only; point to the design doc for the real contract |

## Template

```text
# {Title-echoing heading}

> Created: {date}
> Related design (existing or planned): {path or "TBD"}
> Related todo(s): {id(s), if any}

## 1. Problem (cause and effect)

### 1.1 Scenario
### 1.2 Consequence
### 1.3 Current gap (facts)

## 2. Analysis

### 2.x  (mechanism table / root-cause breakdown / checks performed)

## 3. Optimization direction (brief)

- 3–5 bullets max; detail goes to the design doc once written
```

## Rules

1. **Problem + Analysis dominate** — Optimization direction should read as an
   outline (typically under ~10 lines), never a restated design.
2. **No design contract in the body** — do not write the full technical
   contract (schema, algorithm, acceptance criteria) here; that lives in the
   archive design doc, linked once it exists.
3. **Pending-analysis placeholder** — when the todo is created before analysis
   is done, mark it explicitly (e.g. "state: pending analysis") and write §3
   as open questions to investigate, not a solution outline.
4. **Show the checks, not just the conclusion** — Analysis should reference
   what was actually inspected (files read, commands run, facts compared),
   so the depth of the analysis is verifiable later.
5. **Cross-references at the top only** — link related todo ids / design
   filenames in the header block; do not scatter ad-hoc references through
   the prose.

## When this does not apply

| WorkType | Body shape |
|----------|------------|
| `方案` | Minimal — point to the design doc; do not duplicate problem/analysis prose already covered by a linked `问题描述与分析` todo |
| `缺陷修复` / `流程优化` / `执行待办` / `待优化计划` | Own shape (repro steps / checklist / action list) — not covered by this reference |

## Self-check

Without opening the design doc, can a reader answer:

1. What happened, and what does it break?
2. What was actually checked to reach that conclusion?
3. Roughly which direction is proposed — without needing the full contract here?

Any "no" → rewrite before create.

## User override

If the user gives explicit body content or structure, use it as given and
briefly note that this template was skipped.
