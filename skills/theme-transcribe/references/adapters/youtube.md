# Adapter: youtube

## Match

`youtube.com/watch*` · `youtube.com/shorts/*` · `youtu.be/*`

## Acquire

Same as [generic-ytdlp.md](generic-ytdlp.md).

YouTube path inside yt-dlp is InnerTube (`youtubei/v1/…`) plus player JS / PO Token handling — all inside yt-dlp; do not call Data API for media bytes.

## Map

Same artifacts as generic.

## Quirks

- Age-gate / PO Token failures → update yt-dlp; optional cookies with user approval.
- Prefer `bestaudio` to avoid large muxed downloads when only STT is needed.
