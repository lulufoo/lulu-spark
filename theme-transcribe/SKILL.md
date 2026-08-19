---
name: theme-transcribe
description: >-
  Download video/audio via yt-dlp, local Whisper STT with timestamps, then
  subtopic-split, fluency pass; then hand off to theme-archive for
  Workbench raw + digest (theme-archive translates if the body is full English).
  Independent of theme-line. Use for 视频转写、语音转文本、theme-transcribe、
  X/Twitter/YouTube/Bilibili 视频转文字、带时间戳转写.
---

# ThemeTranscribe

> **Pipeline (fixed order):**
> 1. Transcribe — source language + timestamps  
> 2. Subtopic split — themes + time ranges  
> 3. Fluency pass — source-language final  
> 4. Archive — load [`theme-archive`](../theme-archive/SKILL.md) (full-English translate + upload + digest)  
>
> Read this file in full before executing. Details: [references/pipeline.md](references/pipeline.md).

Local STT pipeline for video/audio URLs. **Does not** call `theme-line`.  
Workbench upload and 摘要/digest are **not** reimplemented here — Phase 4 **calls `theme-archive`**.

## Boundary

| Skill | Role |
|-------|------|
| **theme-transcribe** | URL → timestamped STT → subtopics → fluency → handoff `theme-archive` |
| `theme-line` | Existing transcript → its own ThemeLine compose (separate) |
| `theme-fetch` | Web articles |
| `theme-archive` | Persist finished Markdown + digest (invoked by Phase 4) |

## Triggers

`theme-transcribe` · 视频转写 · 语音转文本 · 带时间戳转写 · X/Twitter/YouTube/Bilibili URL + transcribe intent

## Prerequisites

| Tool | Expected |
|------|----------|
| `yt-dlp` | on `PATH` (or `~/Library/Python/3.9/bin/yt-dlp`) |
| `ffmpeg` | on `PATH` |
| Whisper venv | `~/.local/share/theme-transcribe/venv` with `openai-whisper` |

Missing tool → stop; print install hint. **Do not** improvise alternate downloaders.

Bootstrap venv once if missing:

```bash
uv venv ~/.local/share/theme-transcribe/venv --python 3.12
uv pip install --python ~/.local/share/theme-transcribe/venv/bin/python openai-whisper
```

## Work directory

```text
<workspace>/.cache/theme-transcribe/<ts>-<slug>/
  01-transcript.<lang>.txt     # timestamped STT (source)
  01-transcript.<lang>.srt     # optional sidecar from Whisper
  02-subtopics.<lang>.md       # subtopic sections + time ranges
  03-fluent.<lang>.md          # fluency-optimized source final
  logs/                        # yt-dlp / whisper stdout (never paste into chat)
```

`<workspace>` = Cursor workspace root of the open project (or `git rev-parse --show-toplevel` when cwd is inside that repo). Example for lulu-workbench: `.cache/theme-transcribe/<ts>-<slug>/`.

`ts` = `YYYYMMDDHHMM` (UTC+8). `slug` = kebab-case from title or URL id.

Do **not** use `$HOME/.cache/theme-transcribe/`. Whisper venv stays at `~/.local/share/theme-transcribe/venv` (tool install, not run cache).

## Phase 0 · Resolve

| Input | `platform` | Doc |
|-------|------------|-----|
| `x.com/*`, `twitter.com/*` | `twitter` | [adapters/twitter.md](references/adapters/twitter.md) |
| `youtube.com/*`, `youtu.be/*` | `youtube` | [adapters/youtube.md](references/adapters/youtube.md) |
| Other yt-dlp URL / local media file | `generic` | [adapters/generic-ytdlp.md](references/adapters/generic-ytdlp.md) |

Read matching adapter before Acquire. Full table: [adapters/README.md](references/adapters/README.md).

## Phase 1 · Transcribe (HARD)

1. Create work directory.
2. Run pack script (logs → `logs/` only):

```bash
bash "$SKILL_DIR/theme-transcribe/scripts/transcribe.sh" "<url-or-file>" "<work_dir>" [model]
```

Default model: `small`. Optional: `medium`.

3. Require timestamped transcript (`[HH:MM:SS --> HH:MM:SS]` lines or `.srt`). Bare prose without times → **fail Phase 1**.
4. Detect source language (`en` / other) from Whisper output / `--language`.
5. Chat: report work_dir + duration + language. **Do not** paste full transcript.

## Phase 2 · Subtopic split

Using **only** timestamped source transcript:

- Infer 3–12 subtopics (topic titles, not mechanics).
- Assign contiguous time ranges; each section header includes time span.
- Body under each section: paraphrase / compress from assigned spans; keep key quotes short.
- Write `02-subtopics.<lang>.md`.
- Chat: list subtopic titles + time ranges only.

## Phase 3 · Fluency pass

- Input: `02-subtopics.<lang>.md`
- Smooth spoken rhythm; shorter sentences; preserve facts, names, and time spans.
- **Do not** drop or reorder subtopics; **do not** invent claims.
- Write `03-fluent.<lang>.md`.

## Phase 4 · Archive via theme-archive (default ON)

**Do not** translate here. theme-archive `[AR-1b]` translates only when the fluent body is full English.

1. Read [handoff-archive.md](references/handoff-archive.md).
2. **Load and execute** [`theme-archive`](../theme-archive/SKILL.md) **Embedded**:
   - primary = `03-fluent.<lang>.md` (+ archive header if needed)
   - `source_type`: `summary`
   - **Do not** pass `translations` or `04-fluent.zh.md`
3. Append theme-archive completion lines to the chat reply.

**Skip Phase 4** only when user says 不归档 / skip archive / local only.

Standalone: user may also run `/theme-archive` later on the fluent files under `.cache/theme-transcribe/`.

## Token hygiene

- Redirect yt-dlp / Whisper / ffmpeg noise to `logs/`.
- Chat returns paths + short confirmations only (plus theme-archive result lines in Phase 4).
- STT runs locally — does not consume model API tokens; tool stdout and full-text pastes do.

## Ask only when necessary

Defaults: model `small` · workdir under `<workspace>/.cache/theme-transcribe/` · **Phase 4 theme-archive ON** (it owns en→zh).

Ask when: tool missing · download blocked (login/geo) · language ambiguous · archive slug conflict.

## References

| Doc | Purpose |
|-----|---------|
| [pipeline.md](references/pipeline.md) | Phase rules and artifacts |
| [adapters/](references/adapters/) | Platform Acquire |
| [handoff-archive.md](references/handoff-archive.md) | Embedded payload into theme-archive |
| [../theme-archive/SKILL.md](../theme-archive/SKILL.md) | Upload + digest |
