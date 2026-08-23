# Adapter: twitter / X

## Match

`https://x.com/*/*/status/*` · `https://twitter.com/*/*/status/*`

## Acquire

Same as [generic-ytdlp.md](generic-ytdlp.md). yt-dlp Twitter extractor typically:

1. Guest token + GraphQL (or legacy / syndication fallback)
2. HLS / progressive media URL
3. Fragment download

Use `$TRANSCRIBE_CTL acquire` — do not hand-roll GraphQL calls.

## Map

Same artifacts as generic.

## Quirks

- Relies on undocumented client endpoints; guest GraphQL often works for public posts, then fails after site changes.
- Spaces / auth-gated media may need cookies (user approval).
- Long videos = many HLS fragments; expect multi-minute downloads; keep progress in `logs/` only.
