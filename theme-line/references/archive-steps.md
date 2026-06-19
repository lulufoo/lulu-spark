# Save to Archive — Steps 1–6

> **路径约定：** [archive-concepts.md](../../shared/archive-concepts.md)（`COMMON_PATH`、`prefix`、`layers`）

After Phase 2 Compose produces the ThemeLine body, execute these steps to save via Workbench MCP.

---

### Step 1 · Select project and doc-theme, determine file names

Infer `project` from topics (closest match; unclear → `inbox`). Infer `doc-theme` from `bundle.meta.title` (kebab-case English).

```
topic-path  = <project>/<doc-theme>
slug        = kebab-case summary of source title
ts          = YYYYMMDDHHMM (UTC+8)
COMMON_PATH = <topic-path>/<ts>-<slug>.md
```

Slug conflict → clarify with user before proceeding.

---

### Step 2 · Detect source language

Read `bundle.meta.language`:

| `language` | Action |
|------------|--------|
| `en` | English source; generate `-zh.md` in Step 4 |
| `zh` | Chinese source only; no language suffix |
| `mixed` / `unknown` | Source file only; no `-zh.md` |

---

### Step 3 · Build source file content

```markdown
# {Document Title}

> 创建时间：{YYYY年M月D日 HH:MM}

> 时长：约 {duration_min} 分钟 · 发布：{YYYY-MM-DD}

> 导航：[digest]({prefix}digest/{COMMON_PATH})

> 原文：[Video]({source_url})

---

{ThemeLine body}
```

- Omit 时长 line when `duration_sec` is null.
- `prefix` = `../../../` per [archive-concepts.md](../../shared/archive-concepts.md).

---

### Step 4 · Build Chinese translation file (English source only)

When `bundle.meta.language == "en"`, translate the full ThemeLine body. Same metadata lines; Chinese title.

Path: `raw/<topic-path>/<ts>-<slug>-zh.md`

---

### Step 5 · archive_document (MCP)

<HARD-GATE>
Workbench App **must be running** (`workbench-knowledge` MCP). On failure → stop; **do not** write `archive_root` directly.
</HARD-GATE>

```json
{
  "document": "<Step 3 full markdown>",
  "source_type": "theme-line",
  "extra_documents": [
    {
      "rel": "raw/<topic-path>/<ts>-<slug>-zh.md",
      "content": "<Step 4 full markdown>"
    }
  ],
  "index_extra": {
    "translations": { "zh": "<topic-path>/<ts>-<slug>-zh.md" }
  }
}
```

- Omit `extra_documents` and `index_extra` when language is not `en`.
- Record returned `id`, `common_path`, `raw_path`, `extra_paths`.

---

### Step 6 · archive_digest (MCP)

When `[AD-0]` applies (theme-line raw usually qualifies):

1. Write digest per [theme-digest](../../theme-digest/SKILL.md) `[AD-1]`–`[AD-2]` from **primary raw only**.
2. Call MCP `archive_digest`:

```json
{
  "id": "<Step 5 id>",
  "digest": "<full digest markdown>"
}
```

Use `"force": true` only when user confirms overwrite.

```
📋 digest: digest/<COMMON_PATH>  (or skipped)
```
