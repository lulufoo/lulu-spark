# Adapter: generic yt-dlp

## Match

Any URL `yt-dlp` can extract, or a local audio/video file path.

## Acquire

```bash
bash "$SKILL_DIR/theme-transcribe/scripts/transcribe.sh" "<url-or-file>" "<work_dir>" [model]
```

Script responsibilities:

1. Resolve `yt-dlp` / `ffmpeg` / Whisper venv (see SKILL Prerequisites).
2. Download best audio (or copy local file) → `media/` under work_dir.
3. `ffmpeg` → 16 kHz mono WAV.
4. Whisper → timestamped `01-transcript.<lang>.txt` + `.srt`.
5. All tool stdout/stderr → `logs/`; script stdout prints result paths only.

## Map

| Artifact | Path |
|----------|------|
| Timestamped text | `01-transcript.<lang>.txt` |
| SRT | `01-transcript.<lang>.srt` |
| Meta | `meta.json` (`language`, `model`, `source_url`) |

## Quirks

- Client private APIs (not official download APIs); extractors break when sites change.
- Login / geo walls → try `yt-dlp --cookies-from-browser chrome` only if user approves.
- Prefer audio-only formats to save time/bandwidth.
