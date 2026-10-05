# Adapter: generic yt-dlp

## Match

Any URL `yt-dlp` can extract, or a local audio/video file path.

## Acquire

`$TRANSCRIBE_CTL acquire --src "<url-or-file>" --work-dir "$WORK_DIR"`

See `$TRANSCRIBE_CTL --help` for tool bootstrap. Do not invent another downloader.

## Map

Artifacts and JSON fields: `$TRANSCRIBE_CTL acquire` stdout.

## Quirks

- Extractors break when sites change.
- Login / geo walls → `yt-dlp --cookies-from-browser chrome` only with user approval.
- Prefer audio-only formats.
