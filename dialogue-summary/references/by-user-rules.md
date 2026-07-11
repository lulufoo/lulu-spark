# User stance rules

Tag only user inputs that **materially shaped this dialogue’s process shape**.

## Include

- Changed mainline direction; rejected a whole class of approaches; chose a scheme family
- Forced new steps or withdrew a branch (spine would differ without it)
- Hard constraints that lock later path
- Explicit delete/stop of a design that narrows the process

## Exclude

- `继续` / `执行` / `commit` / `可以` / `ok` style proceed commands
- Pure confirmation of assistant restatement
- Clarifying questions with no stance (unless the user then states a stance)
- Operational nits that do not change process shape

## Test

> If this user line were removed, would the **spine or load-bearing cores** clearly differ? Yes → may tag. No → do not tag.

## Mounting (in the finished document)

User and assistant content are **folded into one process narrative**. Do **not** write as dialogue (who asked / who answered).

When a shape-making **stance or decision** must be attributed:

1. State the proposition in process prose (what was required / rejected / chosen).
2. Append the fixed mark **`〔User〕`** immediately after that proposition (or after the short clause that carries it).
3. Continue with the process consequence in the same paragraph or next sentence — **no** `因此：` banner line.

**Canonical mark:** `〔User〕` only (ASCII letters User). Do not use `By User`, `用户`, `〔By User〕`.

The mark applies **inside table cells too** — do not drop it just because a decision is compressed into a table row / cell.

**Optional longer quote** (rare; when wording itself is load-bearing):

```markdown
> <stance text condensed>
>
> — User
```

Prefer the inline `〔User〕` mark for most cases.

**Do not:**

- Stage the user as narrative subject（「用户追问…」「追问抬到…」）
- Use couplets: `〔By User〕…` + following `因此：`
- Collect a separate “user quotes” chapter

Internal working notes may list shape-making lines for Gate checks; delivery uses `〔User〕` / optional blockquote only.
