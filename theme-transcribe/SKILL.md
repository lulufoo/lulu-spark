---
name: theme-transcribe
description: >-
  Produce a complete verbatim transcript from a video or audio URL, then archive
  it. Multi-speaker audio becomes timed dialogue; otherwise the draft is
  time-segmented. Use for 视频转写、完整逐字稿、theme-transcribe、
  X/Twitter/YouTube/Bilibili 转文字.
---

# ThemeTranscribe

Turn a media URL or local file into a complete source-language verbatim draft and hand it to `theme-archive`. Done when archive returns paths, or when the user opted out of archive after the English draft exists.

## Boundary

| Skill | Role |
|-------|------|
| **theme-transcribe** | Acquire once → verbatim → route → hand off `theme-archive` |
| `theme-line` | Direct captions / API / paste → complete chronological dialogue. No media download, no Whisper. |
| `theme-fetch` | Web articles |
| `theme-archive` | Translate full English, persist raw, digest |

## Triggers

`theme-transcribe` · 视频转写 · 完整逐字稿 · 带时间戳转写 · X/Twitter/YouTube/Bilibili URL + transcribe intent

## Script Macros

| Macro | Command |
|-------|---------|
| `$TRANSCRIBE_CTL` | `python3 "$SKILL_DIR/theme-transcribe/scripts/theme-transcribe-control.py"` |

Subcommands: `--help` · `acquire` · `verbatim` · `route`.

## Execution

1. Resolve `platform`. Load [adapters/README.md](references/adapters/README.md) and the matching adapter. Bind `WORK_DIR` under `<workspace>/.cache/theme-transcribe/<ts>-<slug>/`. Announce: `准备开始：目标为完整逐字稿，不做摘要。`
2. Announce `阶段 1–2/6：正在下载并做 Whisper 时间戳转写……`. Run `$TRANSCRIBE_CTL acquire --src … --work-dir "$WORK_DIR"`. Fail if `ok` is false (missing tools, login/geo, no timestamps). Announce segment count from stdout.
3. Announce `阶段 3/6：正在生成完整逐字稿……`. Run `$TRANSCRIBE_CTL verbatim --work-dir "$WORK_DIR"`.
4. Announce `阶段 4/6：正在判断是否存在多人对话……`. Run `$TRANSCRIBE_CTL route --work-dir "$WORK_DIR"`. Do not run Whisper again.
5. Branch on stdout `mode` only:
   - `dialogue-timed` → announce speaker count; Agent may replace `Speaker A/B` with names evidenced in the draft; `source_type` is `dialogue`.
   - `time-segmented` → announce time-range segmentation, no speaker labels; `source_type` is `transcript`.
6. Load [handoff-archive.md](references/handoff-archive.md). Compose archive header on stdout `primary`. Load and execute [`theme-archive`](../theme-archive/SKILL.md) Embedded. Skip only when the user says 不归档 / skip archive / local only. `skip_translate: true` only when the user says 停止中文翻译.

Ask only when acquire cannot proceed, language is ambiguous, or archive slug conflicts. Default model: `small`.

## Chat

Return phase lines, stdout counts, and theme-archive paths. Do not paste the transcript. Tool noise stays in `logs/`.

## References

| Doc | When |
|-----|------|
| [pipeline.md](references/pipeline.md) | Artifact names after a script error |
| [adapters/](references/adapters/) | Phase 0 / acquire |
| [handoff-archive.md](references/handoff-archive.md) | Archive step |
| [../theme-archive/SKILL.md](../theme-archive/SKILL.md) | Translate + persist |
