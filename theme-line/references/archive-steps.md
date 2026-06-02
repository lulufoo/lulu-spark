# Save to Archive — Steps 1–6

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

### Step 5 · theme-archive Embedded

构造载荷（见 [../../theme-archive/references/input-schema.md](../../theme-archive/references/input-schema.md)）：

```text
加载并完整执行 ../../theme-archive/SKILL.md（Embedded，从 [AR-1] 起：
  COMMON_PATH = <topic-path>/<ts>-<slug>.md
  documents = [
    { rel: "raw/<COMMON_PATH>", content: "<Step 3 全文>" },
    { rel: "raw/<topic-path>/<ts>-<slug>-zh.md", content: "..." }   # en 时
  ]
  index_entry = {
    common_path, created_at: <ts>, source_type: "theme-line", layers: ["raw"],
    translations: { zh: "..." }   # en 时
  }
）
```

将 theme-archive `[AR-5]` 输出追加为中间结果。

### Step 6 · theme-digest Embedded（theme-line 触发）

Primary raw 作为 **RAW**（不对 `-zh.md` digest）：

```text
加载并完整执行 ../../theme-digest/SKILL.md（Embedded：RAW = raw/<COMMON_PATH>，从 [AD-0] 起）
```

Skip if `[AD-0]` is not met。将 digest 结果追加为完成输出：

```
📋 digest: digest/<topic-path>/<ts>-<slug>.md  (or "skipped")
```
