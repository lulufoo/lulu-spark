# Platform adapters — resolve / acquire

Phase 0 outputs `platform`. Load the matching adapter, then run `$TRANSCRIBE_CTL acquire`.

## Routing

| Input pattern | `platform` | Adapter |
|---------------|------------|---------|
| `x.com/*`, `twitter.com/*` | `twitter` | [twitter.md](twitter.md) |
| `youtube.com/*`, `youtu.be/*` | `youtube` | [youtube.md](youtube.md) |
| Other yt-dlp URL, or local media path | `generic` | [generic-ytdlp.md](generic-ytdlp.md) |

Unknown URL → try `generic`; if acquire `ok` is false, stop.

## Adapter doc structure

Each adapter MUST contain: **Match** · **Acquire** · **Map** · **Quirks**.
