# Handoff → theme-archive

After Phase 4, **load and follow** [`theme-archive`](../../theme-archive/SKILL.md) in **Embedded** mode (do not re-implement archive rules here).

## When

- **Default:** run after Phase 4 completes successfully.
- **Skip:** user says 不归档 / skip archive / local only.

## Documents to pass

| Role | File | Notes |
|------|------|-------|
| Primary | `03-fluent.<lang>.md` | Fluent **source** final; add archive header if missing |
| Translation | `04-fluent.zh.md` | If present (English source); add archive header if missing |

Header requirements: `#` title · `> 创建时间：` · `> 导航：` digest link fully resolved. See [archive-concepts](../../shared/archive-concepts.md).

`source_type`: `summary` (unless user overrides).

## MCP shape (theme-archive [AR-2])

Prefer the contract in the current `theme-archive` SKILL:

Write primary markdown to a `.cache` path, then:

```json
{
  "source_path": "<absolute path to primary .md>",
  "source_type": "summary",
  "translations": [
    { "lang": "zh", "content": "<zh markdown with header>" }
  ]
}
```

- Omit `translations` when no `04-fluent.zh.md`.
- **Do not** invent paths; host derives `-{lang}.md`.
- If live MCP schema still only accepts legacy `extra_documents` + `index_extra.translations.zh`, use that fallback (must match expected `-zh` common_path).

## Digest

After `archive_document` succeeds, theme-archive **[AR-3]** runs shared [digest-workflow](../../shared/digest-workflow.md) automatically. Theme-transcribe does **not** call `archive_digest` itself except by executing theme-archive’s workflow.

## Revisions

`archive_document` cannot overwrite existing `raw/` — new archive needs a new `ts` / path.
