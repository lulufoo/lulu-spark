# Save to Archive

> **路径约定：** [archive-concepts.md](../../shared/archive-concepts.md)
>
> 组完 **主文件** 后 Embedded 执行 [theme-archive](../../theme-archive/SKILL.md)。**禁止**在本 skill 翻译或自管 MCP。

After Phase 2 Compose, execute these steps.

---

### Step 1 · Select project and filenames

Infer `project` from topics (closest match; unclear → `inbox`). Infer `doc-theme` from `bundle.meta.title` (kebab-case English).

```
topic-path  = <project>/<doc-theme>
slug        = kebab-case summary of source title
ts          = YYYYMMDDHHMM (UTC+8)
COMMON_PATH = <topic-path>/<ts>-<slug>.md
```

Slug conflict → clarify with user before proceeding.

---

### Step 2 · Build primary file only

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
- `prefix` = `../../../`.
- Write to `{workspace}/.cache/theme-line/<ts>-<slug>.md`.
- **Do not** build `-zh.md`. `bundle.meta.language` does **not** drive translation.

---

### Step 3 · Handoff theme-archive

Load and execute [`theme-archive`](../../theme-archive/SKILL.md) **Embedded** from `[AR-1]`:

- `primary_path` = Step 2 file
- `source_type`: `theme-line`
- `COMMON_PATH` from Step 1

theme-archive runs `[AR-1b]` (full-English → `-zh.md`) then MCP + digest. Append its completion lines.
