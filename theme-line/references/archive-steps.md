# Save to Archive — Steps 1–9

> **路径与配置：** [archive-concepts.md](../../shared/archive-concepts.md)（`COMMON_PATH`、`prefix`、`layers`、读 `config.json`）

After Phase 2 Compose produces the ThemeLine body, execute these steps to save to the local archive and update `index.json`.

**Configuration**: Read `{skill_dir}/../config.json` (repository root) to get `archive_root`. If missing or unreadable → **stop immediately**.

---

### Step 1 · Select project and doc-theme, determine file names

Read `{archive_root}/topics.json` and select a project:

- Take each item's `dir` field (if present), otherwise take the last segment of `repo` (after `/`)
- Infer `doc-theme` from `bundle.meta.title` or source content (kebab-case, English, no spaces)

```
select : project   = dir field or repo short name (e.g. "learning-ai-agent")
         doc-theme = semantic inference from source title (e.g. "waymo-20m-rides-interview")
output : topic-path  = <project>/<doc-theme>
         slug         = kebab-case summary of the source title (English, no spaces)
         ts            = YYYYMMDDHHMM (UTC+8)
         source-file  = raw/<topic-path>/<ts>-<slug>.md
```

If a slug conflict exists, clarify with the user before proceeding.

---

### Step 2 · Detect source language

Read `bundle.meta.language` (do **not** examine body text):

| `language` | Action |
|------------|--------|
| `en` | English source; generate `-zh.md` in Step 4 |
| `zh` | Chinese source only; no language suffix |
| `mixed` / `unknown` | Source file only; no `-zh.md` |

---

### Step 3 · Build source file content

Compose the source `.md` with this header, then the ThemeLine body:

```markdown
# {Document Title}

> 创建时间：{YYYY年M月D日 HH:MM}

> 时长：约 {duration_min} 分钟 · 发布：{YYYY-MM-DD}

> 导航：[distilled]({prefix}distilled/{COMMON_PATH}) · [digest]({prefix}digest/{COMMON_PATH}) · [trace]({prefix}trace/{COMMON_PATH})

> 原文：[Video]({source_url})

{ThemeLine body (no Source: line)}
```

`时长` / `发布` from `bundle.meta.duration_sec` / `bundle.meta.published_at`. Omit 时长 when `duration_sec` is null.

Where (see [archive-concepts.md](../../shared/archive-concepts.md)):

```
COMMON_PATH = <topic-path>/<ts>-<slug>.md
prefix      = "../../../"
```

Navigation paths must be fully resolved — no placeholders.

---

### Step 4 · Build Chinese translation file (English source only)

When `bundle.meta.language == "en"`, translate the full ThemeLine body into Chinese. Keep structural elements (section headings, time lines, speaker labels); translate dialogue only.

```markdown
# {中文标题}

> 创建时间：{YYYY年M月D日 HH:MM}

> 时长：约 {duration_min} 分钟 · 发布：{YYYY-MM-DD}

> 导航：[distilled]({prefix}distilled/<topic-path>/{ts}-{slug}.md) · [digest]({prefix}digest/<topic-path>/{ts}-{slug}.md) · [trace]({prefix}trace/<topic-path>/{ts}-{slug}.md)

> 原文：[Video]({source_url})

{Translated ThemeLine body}
```

Translation path: `raw/<topic-path>/<ts>-<slug>-zh.md`

---

### Step 5 · Generate entry ID

32-character lowercase hex: `secrets.token_hex(16)` (Python) or equivalent.

---

### Step 6 · Prepare index.json entry

**Chinese / mixed / unknown source:**

```json
"<id>": {
  "common_path": "<topic-path>/<ts>-<slug>.md",
  "created_at": "<ts>",
  "layers": ["raw"],
  "source_type": "theme-line"
}
```

**English source:**

```json
"<id>": {
  "common_path": "<topic-path>/<ts>-<slug>.md",
  "created_at": "<ts>",
  "layers": ["raw"],
  "source_type": "theme-line",
  "translations": {
    "zh": "<topic-path>/<ts>-<slug>-zh.md"
  }
}
```

---

### Step 7 · Write files

Write in this order:

1. `{archive_root}/raw/<topic-path>/<ts>-<slug>.md` (source)
2. `{archive_root}/raw/<topic-path>/<ts>-<slug>-zh.md` (English source only)
3. `{archive_root}/index.json` (append new entry)

Create directories if needed.

---

### Step 8 · Completion output

```
✅ Save to Archive 完成
📄 raw：raw/<topic-path>/<ts>-<slug>.md
📄 zh： raw/<topic-path>/<ts>-<slug>-zh.md   (英文源时输出)
🗂 index.json 已更新（新增条目 <id>）
```

### Step 9 · Archive digest (auto)

Use primary raw file from Step 7 — `{archive_root}/raw/<topic-path>/<ts>-<slug>.md` — as **RAW**. Do not digest `-zh.md`.

Load and execute [../theme-digest/SKILL.md](../theme-digest/SKILL.md) (**Embedded**: primary `RAW` only; from `[AD-0]` onward). Skip if `[AD-0]` is not met.

Append:

```
📋 digest: digest/<topic-path>/<ts>-<slug>.md  (or "skipped")
```

Update `index.json` layers: `["raw"]` → append `"digest"`. Do **not** append `"trace"`.
