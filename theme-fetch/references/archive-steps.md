# Save to Archive

> 组完 **主文件** 后加载 note-task，走 Create。

After Phase 2 Format produces the Markdown body, execute these steps.

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

Primary： [output-templates.md](output-templates.md) header + Phase 2 body。

Write to `{workspace}/.cache/theme-fetch/<ts>-<slug>.md`.

`bundle.meta.language` does **not** drive translation.

---

### Step 3 · Handoff note-task

Load note-task and route **Create**:

- stage path = Step 2 file
- `source_type`: `article`

Append `create_note` completion fields (`id`, `common_path`, `raw_path`, optional `digest_path`).
