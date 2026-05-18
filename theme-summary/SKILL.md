---
name: theme-summary
description: >-
  Archive a conversation summary directly to raw/ and run digest automatically.
  Use when the user wants to sync a summary note, written conclusion, or
  session recap from a dialogue into the archive. Triggers on: 总结归档、归档总结、
  summary sync、theme-summary、dtd_raw_summary、归档这段总结、同步到 raw。
---

# ThemeSummary

> **Read this file in full before executing.** This skill has 2 mandatory phases:
> 1. **ThemeSummary Generation** (§ Core Input → § Recommended Workflow)
> 2. **Save to Archive Steps 1–8** (§ Save to Archive) — Step 8 (Archive digest) is required and must not be skipped.
>
> Both phases are required. Neither may be skipped.

Archive a conversation summary — a written conclusion, recap, or distilled note
produced from a dialogue — directly as a `raw/` document. Unlike `ddm`
(`dtd_raw_dialogue`), no turn-by-turn reconstruction is performed; the summary
body is stored as-is.

## Core Input

Two accepted input sources (confirm before proceeding):

| Source | Description |
|--------|-------------|
| A. User paste | The user's message contains the complete summary Markdown |
| B. Agent-generated | Agent writes the summary from the current session; **user must confirm before archiving** |

When source is ambiguous, ask the user which applies before continuing.

## Core Output Shape

```markdown
# <Title>

> 创建时间：YYYY年M月D日 HH:MM
> 来源：theme-summary
> 导航：[distilled](<prefix>distilled/<COMMON_PATH>) · [digest](<prefix>digest/<COMMON_PATH>) · [trace](<prefix>trace/<COMMON_PATH>)

---

<Summary body — verbatim from input, no compression, no TURN_SEP>
```

Rules:
- No `<!-- DDM:TURN_SEP:v1 -->` markers
- No per-turn attribution (User / AI labels)
- No secondary summarization or compression of the body
- Navigation links must be fully resolved — no placeholders

## Recommended Workflow

1. Confirm input source (A or B); if B, generate summary and await user approval.
2. Execute Save to Archive (Steps 1–8, see § Save to Archive below).

---

## Save to Archive

**Configuration**: Read `{skill_dir}/../config.json` (repository root) to get `archive_root`.

---

### Step 1 · Select project and doc-theme, determine file names

Read `{archive_root}/topics.json` and select a project:

- Take each item's `dir` field (if present), otherwise take the last segment of `repo` (after `/`)
- If no suitable project exists, set `project = inbox`
- Infer `doc-theme` from the summary title or content (kebab-case, English, no spaces, 3–5 words)

```
select : project   = dir field or repo short name (e.g. "ai-software-dev")
         doc-theme = semantic inference from summary title (e.g. "agentic-coding-session-recap")
output : topic-path  = <project>/<doc-theme>
         slug         = kebab-case matching the topic (English, no spaces)
         ts            = YYYYMMDDHHMM (UTC+8)
         source-file  = raw/<topic-path>/<ts>-<slug>.md
```

If a slug conflict exists, clarify with the user before proceeding.

---

### Step 2 · Build archive document

Compose the `.md` with this header, then the summary body verbatim:

```markdown
# {Document Title}

> 创建时间：{YYYY年M月D日 HH:MM}
> 来源：theme-summary
> 导航：[distilled]({prefix}distilled/{COMMON_PATH}) · [digest]({prefix}digest/{COMMON_PATH}) · [trace]({prefix}trace/{COMMON_PATH})

---

{Summary body}
```

Where:

```
COMMON_PATH = <topic-path>/<ts>-<slug>.md
prefix      = "../../../"   (topic-path is always 2 segments: project/doc-theme)
```

Navigation paths must be fully resolved — no placeholders.

---

### Step 3 · Generate entry ID

Generate a 32-character lowercase hex ID using `secrets.token_hex(16)` (Python) or equivalent.

---

### Step 4 · Prepare index.json entry

```json
"<id>": {
  "common_path": "<topic-path>/<ts>-<slug>.md",
  "created_at": "<ts>",
  "layers": ["raw"],
  "entry_kind": "summary"
}
```

---

### Step 5 · Write files

Write in this order to avoid index pointing to non-existent files:

1. `{archive_root}/raw/<topic-path>/<ts>-<slug>.md`
2. `{archive_root}/index.json` (append new entry to `entries`)

Create the directory if it does not exist.

---

### Step 6 · Completion output

```
✅ ThemeSummary 归档完成
📄 raw：raw/<topic-path>/<ts>-<slug>.md
🗂 index.json 已更新（entry_kind: summary）
```

---

### Step 7 · Archive digest (auto)

Use the raw file from Step 5 as **RAW**.

Load [../shared/archive-digest.md](../shared/archive-digest.md) and execute [AD-0]–[AD-4].

> Note: [AD-0] condition `entry_kind = summary` lowers the threshold to raw body ≥ 200 chars.
> Skip only if that condition is not met.

Append to the completion output:

```
📋 digest：digest/<topic-path>/<ts>-<slug>.md  (or "skipped")
```

---

## Ask Only When Necessary

Do not stop to ask about formatting if a reasonable default works.

Assume the following defaults:

- Title: inferred from the first heading in the summary body, or ask once if absent
- Project: closest match in topics.json; if unclear, use `inbox`
- Language: Chinese (no translation step)
