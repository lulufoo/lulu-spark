---
name: theme-line
description: Restructure a video transcript, interview, podcast, talk, or speech into a theme-first timeline outline with speaker-separated dialogue blocks. Use when the user wants content from YouTube or another video source organized as title -> theme sections -> speaker turns, especially for requests like "按时间线分类", "按主题整理 transcript", "Host / guest 对话展开", "弱化时间突出主题", or "根据视频标题整理对话内容". Automatically prefer the source title as the document title when available.
---

# ThemeLine

> **Read this file in full before executing.** This skill has 2 mandatory phases:
> 1. **ThemeLine Generation** (§ Core Output Shape → § Recommended Workflow)
> 2. **Save to Archive Steps 1–9** (§ Save to Archive) — Step 9 (Archive digest) is required and must not be skipped.
>
> Both phases are required. Neither may be skipped.

Produce a readable transcript-derived document that emphasizes themes first and time second.

## Core Output Shape

Use this default structure unless the user asks for another format:

1. Document title
2. Archive metadata lines (创建时间, 时长, 发布)
3. Theme-first sections in timeline order
4. Speaker-separated blocks inside each section

Prefer:

`Original Video Title`

`Theme Name`
`Time: 00:47 - 02:23`

`Host:` ...

`Guest:` ...

Do not lead with timestamps in the section header unless the user explicitly asks for time-first formatting.

## Title Handling

When the source is a video URL, fetch or infer the original video title from the source page or a reliable mirror and use that as the top-level title.

If the exact title cannot be confirmed, use the best available source title and say that it was inferred.

If the user provides a custom title, prefer the user's title.

For the Chinese translation file (`-zh.md`), prefer a concise Chinese title (speaker + event + outlet), e.g. `Anthropic CEO Dario Amodei：世界经济论坛 | 华尔街日报`.

## Video Metadata

When the source is a video URL (YouTube, etc.), fetch duration and publish date before writing the archive header:

```bash
python3 -m yt_dlp --print "%(duration)s" --print "%(upload_date)s" --no-download "{url}"
```

| Field | Rule |
|-------|------|
| **时长** | `duration` in seconds → round to nearest minute → `约 {N} 分钟` |
| **发布** | `upload_date` (`YYYYMMDD`) → `YYYY-MM-DD`; if unavailable, omit or note `(inferred)` |

Place on its own line immediately after `创建时间`:

```markdown
> 时长：约 36 分钟 · 发布：2025-01-21
```

Omit the 时长 line only for non-video sources (plain text transcript, local file) where duration is unknown.

## Sectioning Rules

Build sections in chronological order.

Choose section titles that describe the topic, not the mechanics of the transcript. Good examples:

- `Feeling Behind as a Programmer`
- `Software 3.0 Explained`
- `Human Oversight Still Matters`

Avoid weak section titles like:

- `Part 1`
- `Discussion`
- `More Talk About AI`

Use the timestamp as supporting metadata under the theme heading, not as the main label.

## Speaker Handling

Within each section, expand content by speaker turn.

Use the clearest available speaker labels, such as:

- `Host`
- `Andrej Karpathy`
- `Interviewer`
- `Speaker 1`

If the source does not include reliable speaker labels, infer speakers conservatively from context and mark uncertain turns with `(uncertain)` only when needed.

## Transcript Fidelity

Match the user's requested fidelity level:

- If the user wants a summary, paraphrase aggressively.
- If the user wants transcript-like structure, use dialogue-style paraphrase with short, faithful turns.
- If the user wants exact wording from copyrighted material that was not fully provided by them, do not reproduce a full or near-complete transcript. Instead, provide a structured paraphrase with short quoted snippets only when necessary.

Remove obvious ASR duplication and silently fix trivial recognition errors when the intended wording is clear.

## Recommended Workflow

1. Identify the source and available title.
2. For video URLs, fetch duration and `upload_date` (see § Video Metadata).
3. Determine whether the transcript is speaker-labeled, partially labeled, or unlabeled.
4. Break the timeline into coherent topic sections.
5. Name each section by theme.
6. Reconstruct the conversation flow inside each section by speaker.
7. Present time as a secondary line under each theme.
8. Execute Save to Archive (Steps 1–9, see § Save to Archive below).

## Save to Archive

After generating the ThemeLine document, save it to the local archive and update `index.json`.

**Configuration**: Read `{skill_dir}/../config.json` (repository root) to get `archive_root`.

---

### Step 1 · Select project and doc-theme, determine file names

Read `{archive_root}/topics.json` and select a project:

- Take each item's `dir` field (if present), otherwise take the last segment of `repo` (after `/`)
- Infer `doc-theme` from the video title or source content (kebab-case, English, no spaces, describes the semantic topic)

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

Examine the generated document's main body language:

- **Chinese** → source file only, no language suffix, no `translations` field
- **English** → source file is the English original; a Chinese translation will be generated in Step 4

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

`时长` / `发布` come from § Video Metadata. For non-video sources, omit the 时长 line.

Where:

```
COMMON_PATH = <topic-path>/<ts>-<slug>.md
prefix      = "../../../"   (topic-path is always 2 segments: project/doc-theme)
```

Navigation paths must be fully resolved — no placeholders.

---

### Step 4 · Build Chinese translation file (English source only)

Translate the full ThemeLine body into Chinese. Keep all structural elements (section headings, time lines, speaker labels) in their original form; translate only the dialogue and narrative content.

Compose the translation `.md`:

```markdown
# {中文标题}

> 创建时间：{YYYY年M月D日 HH:MM}

> 时长：约 {duration_min} 分钟 · 发布：{YYYY-MM-DD}

> 导航：[distilled]({prefix}distilled/<topic-path>/{ts}-{slug}.md) · [digest]({prefix}digest/<topic-path>/{ts}-{slug}.md) · [trace]({prefix}trace/<topic-path>/{ts}-{slug}.md)

> 原文：[Video]({source_url})

{Translated ThemeLine body}
```

Use the same `时长` / `发布` as the English source file.

Translation file path: `raw/<topic-path>/<ts>-<slug>-zh.md`

---

### Step 5 · Generate entry ID

Generate a 32-character lowercase hex ID using `secrets.token_hex(16)` (Python) or equivalent.

---

### Step 6 · Prepare index.json entry

**Chinese source:**

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

Write in this order to avoid index pointing to non-existent files:

1. `{archive_root}/raw/<topic-path>/<ts>-<slug>.md` (source)
2. `{archive_root}/raw/<topic-path>/<ts>-<slug>-zh.md` (translation, English source only)
3. `{archive_root}/index.json` (append new entry to `entries`)

Create the directory if it does not exist.

---

### Step 8 · Completion output

```
✅ Save to Archive 完成
📄 raw：raw/<topic-path>/<ts>-<slug>.md
📄 zh： raw/<topic-path>/<ts>-<slug>-zh.md   (英文源时输出)
🗂 index.json 已更新（新增条目 <id>）
```

### Step 9 · Archive digest (auto)

Use the primary raw file from Step 7 — `{archive_root}/raw/<topic-path>/<ts>-<slug>.md` — as **RAW**. Do not run digest separately on the `-zh.md` file.

Load and execute [../theme-digest/SKILL.md](../theme-digest/SKILL.md) (**Embedded**: primary `RAW` only, not `-zh.md`; from `[AD-0]` onward). Skip if [AD-0] is not met.

Append to the completion output:

```
📋 digest: digest/<topic-path>/<ts>-<slug>.md  (or "skipped")
```

---

## Ask Only When Necessary

Do not stop to ask about formatting if a reasonable default works.

Assume the following defaults:

- Title: source title
- Ordering: chronological
- Section style: theme-first
- Time display: secondary
- Speaker style: `Host` plus named guest when identifiable

## References

For output patterns and wording conventions, see [references/output-templates.md](references/output-templates.md).
