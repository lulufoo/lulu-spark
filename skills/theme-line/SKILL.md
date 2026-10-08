---
name: theme-line
description: >-
  Acquire existing captions or transcripts (YouTube, InfoQ, plain text) and
  compose a complete chronological dialogue under workspace .cache.
  No media download, no Whisper.
  Use for theme-line, 完整对话整理, YouTube/InfoQ 字幕采集, Host/guest 对话展开.
---

# ThemeLine

> **4 mandatory phases:**
> 0. Resolve Source
> 1. Acquire → TranscriptBundle
> 2. Compose complete dialogue
> 3. Write `.cache` markdown
>
> Read this file in full before executing.

Produce a complete chronological dialogue from captions or an existing transcript. Time and source chapters are navigation only. DONE when the primary markdown path exists.

## Boundary

| Skill | Role |
|-------|------|
| **theme-line** | Direct captions / API / local or pasted transcript → complete dialogue. **No** media download. **No** Whisper. |
| `theme-transcribe` | Download media → Whisper → verbatim draft |

The written markdown must keep the full dialogue.

## Phase 0 · Resolve Source

| Input | `platform` | Doc |
|-------|-----------|-----|
| `youtube.com/*`, `youtu.be/*` | `youtube` | [adapters/youtube.md](references/adapters/youtube.md) |
| `infoq.cn/video/*`, `infoq.cn/article/*` | `infoq` | [adapters/infoq.md](references/adapters/infoq.md) |
| Local file, paste, unknown URL | `plain` | [adapters/plain-text.md](references/adapters/plain-text.md) |

Full routing: [adapters/README.md](references/adapters/README.md)

## Phase 1 · Acquire

<HARD-GATE>
MUST read the matching adapter doc before any fetch. Do NOT improvise platform-specific commands. Do NOT download audio/video. Do NOT run Whisper.
</HARD-GATE>

- Output MUST conform to [references/bundle-schema.md](references/bundle-schema.md)
- Read `references/adapters/{platform}.md` → Match / Acquire / Map / Quirks
- Bundle is ephemeral (memory only); optional debug: `{workspace}/.cache/theme-line/{ts}-{slug}-bundle.json`

## Phase 2 · Compose

Read [references/compose-strategies.md](references/compose-strategies.md) — steps C0–C6.

| Strategy | Condition |
|----------|-----------|
| `complete-dialogue` | `utterances.length > 0` |
| fail-fast | `utterances.length == 0` → stop, do not write |

`segments` never substitute for missing utterances. Do not emit `summary-only` or paraphrase outlines.

### Sectioning

Keep source order. Source chapter titles (if present) are navigation markers only. If no chapters, use time-range markers. Do not regroup by invented themes.

### Speaker Handling

Acquire must fill `meta.speakers` with proper names when title or in-text labels give them. Compose must put those names on turns. `(uncertain)` is fallback only. Rules: [speaker-roster.md](references/speaker-roster.md). Caption-only sources are **not** diarization.

### Transcript Fidelity

Light clean only: merge caption fragments, drop consecutive duplicates, apply `fidelity.corrections`. Keep source wording and coverage. **No** paraphrase, summary, or dropped turns. Persist the bundle and run `scripts/check_dialogue_coverage.py` before write (see [archive-steps.md](references/archive-steps.md)).

Output patterns: [references/output-templates.md](references/output-templates.md)

## Phase 3 · Write

Load and execute [references/archive-steps.md](references/archive-steps.md) from Step 1.

## Title Handling

- Video URL → use source title from `bundle.meta.title`
- User custom title → prefer user's title

## Ask Only When Necessary

Defaults: source title · chronological complete dialogue · source-chapter nav if present else time-range nav · named host/guest from title or labels when evidenced

## References

| Doc | Purpose |
|-----|---------|
| [bundle-schema.md](references/bundle-schema.md) | TranscriptBundle contract |
| [adapters/](references/adapters/) | Platform Acquire docs |
| [compose-strategies.md](references/compose-strategies.md) | Compose rules |
| [speaker-roster.md](references/speaker-roster.md) | Name roster + turn labels |
| [archive-steps.md](references/archive-steps.md) | Build primary markdown |
| [output-templates.md](references/output-templates.md) | Output patterns |
