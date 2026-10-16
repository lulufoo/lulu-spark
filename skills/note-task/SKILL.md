---
name: note-task
description: >-
  Note-task MCP workflow: create and read Lulu Spark documents by workspace and project via MCP only.
  Use when: note-task, create_doc, archive a note, save Markdown to a workspace.
argument-hint: '[title | path | pasted Markdown]'
---

# note-task — Document MCP Workflow

## Boundary

Operate documents only through MCP. Documents live in a `workspace` and a `project` inside it. Any other method is forbidden.

## Parameter SSOT

Before every call, read the live MCP tool `description` / `inputSchema`.

- This SKILL does **not** restate field contracts (they change with MCP).
- Prefer the live schema over any remembered or cached shape.
- If schema and Host behavior disagree, stop and report the mismatch; do not invent parameters.

## MCP Prerequisite

<HARD-GATE mcp="notes">
Lulu Spark App **MUST** be running, with its Host MCP server reachable.

- If Host is down or HTTP unreachable, document tools will not appear — start Lulu Spark App first.
</HARD-GATE>

## Intent Routing

Map the user request, then call **only** the tools that match. Do not run a fixed pipeline.

| User intent | Do | Do not |
|-------------|----|--------|
| Create / save / archive a document | Confirm the target with `list_workspaces` and `list_projects`. When the user names none, use workspace `Default` and project `inbox`. Infer `folder` and `title` when the user did not give them — [path-or-paste.md](references/path-or-paste.md) for a lone path or paste. Load [chinese-companion.md](references/chinese-companion.md). Call `create_doc` with the live schema parameters. | Do not invent a workspace or project id |
| List workspaces | `list_workspaces` | — |
| List projects of one workspace | `list_projects` | — |
| Search documents | `search`, optionally limited to one workspace | Do not list a catalog's full id set |
| Read one digest | `get_doc_digest` with the `workspace` of the search hit | — |
| Read one raw body | `get_doc_file` with a `dest_dir`, then read the returned copy | Do not read the data directory directly |
| Edit an existing document | `get_doc_file`, edit the copy, then `update_doc` with `source_path` set to the copy | Do not edit the data directory directly |

**Create is one tool.** `digest` is a required `create_doc` field. When a digest will be written, pass `digest_body` on the same call. Host AD-0 decides whether `auto` writes. Companion-image when-to-pass is the live MCP description; collecting sibling files is [path-or-paste.md](references/path-or-paste.md).

## Document Norms

Before writing document fields, load and follow the matching reference.
Do not invent norms not listed here.

| Field / concern | Reference | When |
|-----------------|-----------|------|
| `title` | [choosing-a-title.md](references/choosing-a-title.md) | Before choosing or proposing a create title |
| Standalone path / paste | [path-or-paste.md](references/path-or-paste.md) | Path or paste with no producer (includes summary image strip and sibling-file collection) |
| Sibling image files | [path-or-paste.md](references/path-or-paste.md) | How to collect files next to a staged Markdown |
| Full-English detect | [chinese-companion.md](references/chinese-companion.md) | Before every `create_doc` |
| `digest_body` shape | [digest-body.md](references/digest-body.md) | Before composing `digest_body` (`auto` / `always`) |

- User-explicit values override the corresponding norm; say so briefly when skipping.
- Field limits still come from live MCP schema (Parameter SSOT).

## Hard Constraints

1. **Listed tools only** — Agent must use `note-task` and the document tools listed in this SKILL only.
2. **Verify after write** — After create or update, confirm `ok` and `id`. Treat `digest_written`, `zh_written`, and `assets_copied` as present only when that write happened. Do not require `common_path`, `raw_path`, `digest_path`, `extra_paths`, or `asset_paths` on the result.
3. **HTTP errors** — 4xx/5xx surface as MCP tool errors (`isError: true`); do not treat error payloads as success.
4. **Path or paste** — a lone markdown path or pasted body uses [path-or-paste.md](references/path-or-paste.md). Do not call Host HTTP.

## MCP Document Tools

| MCP tool | Purpose |
|----------|---------|
| `list_workspaces` | List workspaces; the `Default` workspace always exists |
| `list_projects` | List the projects of one workspace; does not list documents |
| `create_doc` | Create a document in a workspace and project |
| `search` | Search Lulu Spark's own library across workspaces or in one; returns id, title, snippet, workspace, project |
| `get_doc_digest` | Digest Markdown for one document id |
| `get_doc_file` | Copy one document's file into `dest_dir` and return `{ok, id, path}`; the body is not returned and an existing copy is never overwritten |
| `update_doc` | Replace an existing document body from the Markdown file at `source_path` |

Field names, limits, and optionality: live MCP schema only.

## Done

Observable completion for a write request:

- Tool call succeeded; and
- Response has `ok` and `id`; and
- When a digest was written: `digest_written` is present; when skipped: it is absent; and
- When a zh companion was written: `zh_written` is present; when skipped: it is absent; and
- When companion images were sent: `assets_copied` is present; when skipped: it is absent.

## References

- Choosing a title: [choosing-a-title](references/choosing-a-title.md)
- Digest body: [digest-body](references/digest-body.md)
- Chinese companion: [chinese-companion](references/chinese-companion.md)
