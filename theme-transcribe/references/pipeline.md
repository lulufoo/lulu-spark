# ThemeTranscribe artifacts

Work root: `<workspace>/.cache/theme-transcribe/<ts>-<slug>/`.  
Scripts own reads and writes. SKILL branches on `$TRANSCRIBE_CTL` JSON only.

## Layout

```text
media/source.*
media/audio.wav
logs/
01-transcript.<lang>.txt
01-transcript.<lang>.srt
02-verbatim.<lang>.md
03-dialogue-timed.<lang>.md
03-time-segmented.<lang>.md
meta.json
corrections.json
diarization.json
```

`03-dialogue-timed` and `03-time-segmented` are alternatives. Install venvs stay under `~/.local/share/theme-transcribe/`.

## Invariants

1. `acquire` is the only Whisper call.
2. `01-transcript` is read-only after acquire.
3. `verbatim` keeps full coverage: no summary, no reorder, no drop.
4. `route` labels existing time windows or falls back to time ranges.
5. Chinese drafts are not written here.

## Control stdout

`acquire` / `verbatim` / `route` each print one JSON object. Required on success: `ok`, `phase`. `route` also prints `mode`, `source_type`, `primary`, `speaker_count`, `turn_count`.
