# ThemeTranscribe pipeline

> Artifacts live under `<workspace>/.cache/theme-transcribe/<ts>-<slug>/`  
> (`<workspace>` = Cursor workspace root / git top-level of the open project).

---

## Phase 1 · Transcribe

**Goal:** source-language text with timestamps.

**Acquire:** adapter + `scripts/transcribe.sh`.

**Output format** (`01-transcript.<lang>.txt`), preferred:

```text
[00:00:00 --> 00:00:12] First utterance…
[00:00:12 --> 00:00:28] Next…
```

Also accept Whisper `.srt` as source of truth; derive the `.txt` view if needed.

**Hard fail:** no timestamps.

**Language:** write detected / forced code into a one-line `meta.json`:

```json
{ "language": "en", "model": "small", "source_url": "…" }
```

---

## Phase 2 · Subtopic split

**Input:** `01-transcript.<lang>.txt` (and `.srt` if present).

**Output:** `02-subtopics.<lang>.md`

```markdown
# <Title>

## <Subtopic title>
> 时间：MM:SS – MM:SS

<body>

## <Next subtopic>
> 时间：MM:SS – MM:SS

<body>
```

Rules:

- Chronological section order.
- Theme titles describe topics, not “intro / outro / Q&A mechanics” unless that is the topic.
- Every body sentence must map to some timestamp span in Phase 1; no orphan claims.
- Target roughly 3–12 sections for a ~10–20 min talk; scale with duration.

---

## Phase 3 · Fluency

**Input:** `02-subtopics.<lang>.md`  
**Output:** `03-fluent.<lang>.md`

Keep the same `##` sections and time ranges. Improve readability only.

---

## Phase 4 · Archive / digest

**Default ON.** Execute [`theme-archive`](../../theme-archive/SKILL.md) Embedded per [handoff-archive.md](handoff-archive.md).  
Primary = fluent source only. **Do not** pre-translate. theme-archive `[AR-1b]` emits `-zh.md` when the body is full English.  
Skip only if user opts out.
