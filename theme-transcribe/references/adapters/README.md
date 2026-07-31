# Platform adapters — Phase 0 / Phase 1

> Phase 0 outputs `platform`; Phase 1 reads the matching adapter, then runs `scripts/transcribe.sh`.

## Routing

| Input pattern | `platform` | Adapter |
|---------------|------------|---------|
| `x.com/*`, `twitter.com/*` | `twitter` | [twitter.md](twitter.md) |
| `youtube.com/*`, `youtu.be/*` | `youtube` | [youtube.md](youtube.md) |
| Other URL supported by yt-dlp, or local media path | `generic` | [generic-ytdlp.md](generic-ytdlp.md) |

Unknown URL → try `generic`; if yt-dlp cannot extract, stop and ask user for a file or another URL.

## Adapter doc structure

Each adapter MUST contain: **Match** · **Acquire** · **Map** · **Quirks**.
