# Save to Archive

> 组完 **主文件** 后加载 note-task，走 Create。需要中文稿时，先跑 note-task 的 `detect_full_english.py` / `check_zh_parity.py`，再带 `translations`。
>
> `$SKILL_DIR` = `lulu-workbench-skills` install root.

After Phase 2 Compose, execute these steps.

---

### Step 1 · Select project and staging name

Infer `project` from topics (closest match; unclear → `inbox`). Infer `doc-theme` from `bundle.meta.title` (kebab-case English).

```
slug = kebab-case summary of source title
ts   = YYYYMMDDHHMM (UTC+8)
```

Staging path only — Host assigns `common_path` on Create. Slug conflict → clarify with user before proceeding.

---

### Step 2 · Build primary file only

Header: [output-templates.md](output-templates.md). Body is the full lightly cleaned dialogue, not a theme outline.

- Write to `{workspace}/.cache/theme-line/<ts>-<slug>.md`.
- Also write the TranscriptBundle to `{workspace}/.cache/theme-line/<ts>-<slug>-bundle.json`.
- Run coverage before handoff:

```bash
python3 "$SKILL_DIR/theme-line/scripts/check_dialogue_coverage.py" "<primary.md>" "<bundle.json>"
```

- Exit 1 → stop. Do not hand off note-task.
- `bundle.meta.language` does **not** drive translation. Need `-zh.md` → run `$SKILL_DIR/note-task/scripts/detect_full_english.py` then `check_zh_parity.py` before Create.

---

### Step 3 · Handoff note-task

Load note-task and route **Create**:

- stage path = Step 2 file
- `source_type`: `theme-line`

Append `create_note` completion fields (`id`, `common_path`, `raw_path`, optional `digest_path`).
