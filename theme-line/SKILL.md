---
name: theme-line
description: >-
  Restructure video/interview transcripts (YouTube, InfoQ, plain text) into theme-first
  timeline outlines with speaker-separated dialogue blocks. Multi-platform Acquire via
  TranscriptBundle; saves to raw/ and auto digest. Use for 按主题整理 transcript,
  Host/guest 对话展开, InfoQ/YouTube 视频归档.
---

# ThemeLine

> **4 mandatory phases:**
> 0. Resolve Source
> 1. Acquire → TranscriptBundle
> 2. Compose ThemeLine
> 3. Save to Archive
>
> Read this file in full before executing. Phase 3 chains **theme-archive** (Steps 1–5) then **theme-digest** (Step 6).

Produce a readable transcript-derived document: themes first, time second.

## Phase 0 · Resolve Source

| Input | `platform` | Doc |
|-------|-----------|-----|
| `youtube.com/*`, `youtu.be/*` | `youtube` | [adapters/youtube.md](references/adapters/youtube.md) |
| `infoq.cn/video/*`, `infoq.cn/article/*` | `infoq` | [adapters/infoq.md](references/adapters/infoq.md) |
| Local file, paste, unknown URL | `plain` | [adapters/plain-text.md](references/adapters/plain-text.md) |

Full routing: [adapters/README.md](references/adapters/README.md)

## Phase 1 · Acquire

<HARD-GATE>
MUST read the matching adapter doc before any fetch. Do NOT improvise platform-specific commands.
</HARD-GATE>

- Output MUST conform to [references/bundle-schema.md](references/bundle-schema.md)
- Read `references/adapters/{platform}.md` → Match / Acquire / Map / Quirks
- Bundle is ephemeral (memory only); optional debug: `{archive_root}/.cache/{topic-path}/{ts}-{slug}-bundle.json`
- **禁止** persist bundle to `trace/`

## Phase 2 · Compose

Read [references/compose-strategies.md](references/compose-strategies.md) — steps C0–C6.

| Strategy | Condition |
|----------|-----------|
| `segment-seeded` | segments>0 AND utterances>0 |
| `flat-caption` | segments==0 AND utterances>0 |
| `summary-only` | segments>0 AND utterances==0 |
| fail-fast | both==0 → stop, do not Archive |

### Sectioning

Build sections in chronological order. Theme titles describe topics, not mechanics. Timestamp is secondary metadata under each theme heading.

### Speaker Handling

Expand by speaker turn: `Host`, named guest, or `(uncertain)` when needed. flat-caption is **not** diarization — cap at Host/Guest + `(uncertain)`.

### Transcript Fidelity

Dialogue-style paraphrase. **No** near-complete verbatim SRT copy. Short quotes ≤2 sentences/section OK. Apply `fidelity.corrections` per compose C1.

Output patterns: [references/output-templates.md](references/output-templates.md)

## Phase 3 · Save to Archive

Load and execute [references/archive-steps.md](references/archive-steps.md) from Step 1.

Path/config: [../shared/archive-concepts.md](../shared/archive-concepts.md)

## Title Handling

- Video URL → use source title from `bundle.meta.title`
- User custom title → prefer user's title
- `-zh.md` title → concise Chinese (speaker + event + outlet)

## Ask Only When Necessary

Defaults: source title · chronological · theme-first sections · secondary time · Host + named guest

## References

| Doc | Purpose |
|-----|---------|
| [bundle-schema.md](references/bundle-schema.md) | TranscriptBundle contract |
| [adapters/](references/adapters/) | Platform Acquire docs |
| [compose-strategies.md](references/compose-strategies.md) | Compose rules |
| [archive-steps.md](references/archive-steps.md) | Save to Archive Steps 1–9 |
| [output-templates.md](references/output-templates.md) | Output patterns |
