---
name: note-task
description: >-
  Note-task MCP workflow: create and read Lulu Spark notes via MCP only.
  Use when: note-task, create_note, archive a note, save Markdown to notes.
argument-hint: '[title | path | pasted Markdown]'
---

# note-task — Note MCP Workflow

## Boundary

Operate notes only through MCP. Any other method is forbidden.

## Parameter SSOT

Before every call, read the live MCP tool `description` / `inputSchema`.

- This SKILL does **not** restate field contracts (they change with MCP).
- Prefer the live schema over any remembered or cached shape.
- If schema and Host behavior disagree, stop and report the mismatch; do not invent parameters.

## MCP Prerequisite

<HARD-GATE mcp="notes">
Lulu Spark App **MUST** be running, and the `knowledge-mcp` sidecar spawned.

- If Host is down or HTTP unreachable, note tools will not appear — start Lulu Spark App first.
</HARD-GATE>

## Intent Routing

Map the user request, then call **only** the tools that match. Do not run a fixed pipeline.

| User intent | Do | Do not |
|-------------|----|--------|
| Create / save / archive a note | Infer `project`, `theme`, and `title` when the user did not give them — [path-or-paste.md](references/path-or-paste.md) for a lone path or paste. Load [chinese-companion.md](references/chinese-companion.md). Call `create_note` with the live schema parameters. | — |
| List catalogs with newest note pointer | `get_all_notes_catalog` | — |
| Search notes and knowledge | `search_document` | Do not list a catalog's full id set |
| Search notes by text | `search_notes` | Do not loop catalog-by-catalog |
| Read one digest | `get_note_digest_by_id` | — |
| Read one raw body | `get_note_content_by_id` | — |

**Create is one tool.** `digest` is a required `create_note` field. When a digest will be written, pass `digest_body` on the same call. Host AD-0 decides whether `auto` writes. Companion-image when-to-pass is the live MCP description; collecting sibling files is [path-or-paste.md](references/path-or-paste.md).

## Note Norms

Before writing note fields, load and follow the matching reference.
Do not invent norms not listed here.

| Field / concern | Reference | When |
|-----------------|-----------|------|
| `title` | [choosing-a-title.md](references/choosing-a-title.md) | Before choosing or proposing a create title |
| Standalone path / paste | [path-or-paste.md](references/path-or-paste.md) | Path or paste with no producer (includes summary image strip and sibling-file collection) |
| Sibling image files | [path-or-paste.md](references/path-or-paste.md) | How to collect files next to a staged Markdown |
| Full-English detect | [chinese-companion.md](references/chinese-companion.md) | Before every `create_note` |
| `digest_body` shape | [digest-body.md](references/digest-body.md) | Before composing `digest_body` (`auto` / `always`) |

- User-explicit values override the corresponding norm; say so briefly when skipping.
- Field limits still come from live MCP schema (Parameter SSOT).

## Hard Constraints

1. **Listed tools only** — Agent must use `note-task` and the note tools listed in this SKILL only.
2. **Verify after write** — After create, confirm `id` / `common_path` / `raw_path`; `digest_path` when a digest was written; `extra_paths` when a zh companion was written; `asset_paths` when companion images were sent.
3. **HTTP errors** — 4xx/5xx surface as MCP tool errors (`isError: true`); do not treat error payloads as success.
4. **Path or paste** — a lone markdown path or pasted body uses [path-or-paste.md](references/path-or-paste.md). Do not call Host HTTP.

## MCP Note Tools

| MCP tool | Purpose |
|----------|---------|
| `create_note` | Create a note |
| `get_all_notes_catalog` | Every project catalog with newest `note_id` + `created_at` (no bodies) |
| `search_document` | Search notes and knowledge; returns id, title, snippet, category |
| `search_notes` | Search raw note bodies; returns note ids and match snippets (no digest) |
| `get_note_digest_by_id` | Digest Markdown for one note id |
| `get_note_content_by_id` | Raw Markdown for one note id (Host truncates over 10KB) |

Field names, limits, and optionality: live MCP schema only.

## Done

Observable completion for a write request:

- Tool call succeeded; and
- Response has `id`, `common_path`, `raw_path`; and
- When a digest was written: `digest_path` is present; when skipped: it is absent; and
- When a zh companion was written: `extra_paths` includes the `-zh.md` path; and
- When companion images were sent: `asset_paths` lists the copied `raw/…` files.

## References

- Choosing a title: [choosing-a-title](references/choosing-a-title.md)
- Digest body: [digest-body](references/digest-body.md)
- Chinese companion: [chinese-companion](references/chinese-companion.md)
