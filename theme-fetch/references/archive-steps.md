# Save to Archive — Steps 1–6

> **路径约定：** [archive-concepts.md](../../shared/archive-concepts.md)（`COMMON_PATH`、`prefix`、`layers`）

After Phase 2 Format produces the Markdown body, execute these steps to save via Workbench MCP.

---

### Step 1 · Select project and doc-theme

Infer `project` from topics (closest match; unclear → `inbox`). Infer `doc-theme` from `bundle.meta.title` (kebab-case English). Infer `slug`, `ts`, `COMMON_PATH` from title.

```
topic-path  = <project>/<doc-theme>
slug        = kebab-case summary of source title
ts          = YYYYMMDDHHMM (UTC+8)
COMMON_PATH = <topic-path>/<ts>-<slug>.md
```

Slug conflict → clarify with user before proceeding.

---

### Step 2 · Detect source language

Read `bundle.meta.language`：`en` → source + `-zh.md`；`zh` / `mixed` / `unknown` → source only.

---

### Step 3 · Build documents

Primary： [output-templates.md](output-templates.md) header + Phase 2 body。

英文源 → 翻译 body → `-zh.md`（同 header 模板）。

---

### Step 4 · archive_document (MCP)

<HARD-GATE>
Workbench App **must be running** (`workbench-knowledge` MCP). On failure → stop; **do not** write corpus files directly.
</HARD-GATE>

```json
{
  "document": "<Step 3 primary full markdown>",
  "source_type": "article",
  "extra_documents": [
    {
      "rel": "raw/<topic-path>/<ts>-<slug>-zh.md",
      "content": "<Step 3 -zh.md full markdown>"
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

### Step 5 · archive_digest (MCP)

When `[AD-0]` applies (`source_type = article` uses normal thresholds):

1. Write digest per [digest-workflow](../../shared/digest-workflow.md) `[AD-1]`–`[AD-2]` from **primary raw only** (not `-zh.md`).
2. Call MCP `archive_digest`:

```json
{
  "id": "<Step 4 id>",
  "digest": "<full digest markdown>"
}
```

Use `"force": true` only when user confirms overwrite.

```
📋 digest: digest/<COMMON_PATH>  (or skipped)
```
