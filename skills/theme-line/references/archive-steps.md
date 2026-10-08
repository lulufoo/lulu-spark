# Write primary markdown

> `$SKILL_DIR` = `lulu-spark-skills` install root.

After Phase 2 Compose, execute these steps.

---

### Step 1 · Name the write

```
slug = kebab-case summary of source title
ts   = YYYYMMDDHHMM (UTC+8)
```

Slug conflict → clarify with user before proceeding.

---

### Step 2 · Build the primary file

Header: [output-templates.md](output-templates.md). Body is the full lightly cleaned dialogue, not a theme outline.

- Write to `{workspace}/.cache/theme-line/<ts>-<slug>.md`.
- Also write the TranscriptBundle to `{workspace}/.cache/theme-line/<ts>-<slug>-bundle.json`.
- Run coverage:

```bash
python3 "$SKILL_DIR/theme-line/scripts/check_dialogue_coverage.py" "<primary.md>" "<bundle.json>"
```

- Exit 1 → stop.
- `bundle.meta.language` does **not** drive translation.

DONE when the primary markdown path exists.

```text
> ✅ theme-line complete
> 📄 local：<absolute .cache path>
```
