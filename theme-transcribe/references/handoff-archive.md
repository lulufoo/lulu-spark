# Handoff → theme-archive

After fluency, **load and follow** [`theme-archive`](../../theme-archive/SKILL.md) in **Embedded** mode (do not re-implement archive or translation here).

## When

- **Default:** run after `03-fluent.<lang>.md` is written.
- **Skip:** user says 不归档 / skip archive / local only.

## Documents to pass

| Role | File | Notes |
|------|------|-------|
| Primary | `03-fluent.<lang>.md` | Fluent **source** final; add archive header if missing |

**Do not** pass a zh attachment. theme-archive `[AR-1b]` decides.

`source_type`: `summary` (unless user overrides).

## Digest

After `archive_document` succeeds, theme-archive **[AR-3]** runs shared digest. Theme-transcribe does **not** call `archive_digest` itself.

## Revisions

`archive_document` cannot overwrite existing `raw/` — new archive needs a new `ts` / path.
