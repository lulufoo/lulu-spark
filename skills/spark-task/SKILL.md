---
name: spark-task
description: >-
  Spark MCP workflow: work in Lulu Spark through its five MCP groups — workspaces, projects, documents, search and Read Later — via MCP only.
  Use when: spark-task, workspace, project, create_doc, save Markdown to a workspace, search my notes, read later.
argument-hint: '[workspace | project | title | path | pasted Markdown | search query | read-later link]'
---

# spark-task — Spark MCP Workflow

## Boundary

Operate Lulu Spark only through its MCP tools. A **workspace** holds **projects**; a project holds **documents**. **Read Later** is a separate saved reading list. Any other method is forbidden.

## Parameter SSOT

Before every call, read the live MCP tool `description` / `inputSchema`.

- This SKILL does **not** restate field contracts (they change with MCP).
- Prefer the live schema over any remembered or cached shape.
- If schema and Host behavior disagree, stop and report the mismatch; do not invent parameters.

## MCP Prerequisite

<HARD-GATE mcp="spark">
Lulu Spark App **MUST** be running, with its Host MCP server reachable.

- If Host is down or HTTP unreachable, the tools will not appear — start Lulu Spark App first.
- A person chooses, per MCP channel, which tools are on (Settings → MCP). A tool missing from the live tool list is off or not offered on this channel: say so; do not look for another way.
</HARD-GATE>

## The five groups

| Group | Tools | What it is for |
|-------|-------|----------------|
| Global | `search` | Find documents across every workspace, or in one |
| Workspace | `list_workspaces`, `create_workspace`, `rename_workspace` | The containers; `Default` always exists |
| Project | `list_projects`, `create_project`, `update_project` | The folders inside a workspace; every workspace starts with `inbox` |
| Document | `get_doc_digest`, `get_doc_file`, `get_doc_content`, `create_doc`, `update_doc`, `delete_doc` | The documents themselves |
| Read Later | `get_read_later` | One saved reading-list entry |

Channel exposure (the live tool list wins):

- `get_doc_file` is for the Spark, Cursor, Codex and Claude channels; `get_doc_content` is its **mobile-only** counterpart.
- `delete_doc` and `get_read_later` are offered on `/mcp/spark` only.
- There is no tool to delete, disable or enable a workspace or project: a person does that in Settings.

## Intent Routing

Map the user request, then call **only** the tools that match. Do not run a fixed pipeline.

| User intent | Do | Do not |
|-------------|----|--------|
| Create a workspace | `create_workspace`; the result carries its generated id | Do not guess an id before the result |
| Rename a workspace | `list_workspaces` to find the id, then `rename_workspace` | Do not rename by name alone |
| Create a project | Confirm the workspace with `list_workspaces`, check `list_projects` for a near-duplicate, then `create_project` | Do not create a project to move documents: it does not move any |
| Edit a project's title or description | `list_projects` for its `folder`, then `update_project` | Do not change the `folder` |
| List workspaces / projects | `list_workspaces`; `list_projects` with the workspace | — |
| Create / save / archive a document | Confirm the target with `list_workspaces` and `list_projects`. When the user names none, use workspace `Default` and project `inbox`. Infer `folder` and `title` when the user did not give them — [path-or-paste.md](references/path-or-paste.md) for a lone path or paste. Load [chinese-companion.md](references/chinese-companion.md). Call `create_doc` with the live schema parameters. | Do not invent a workspace or project id |
| Search documents | `search`, optionally limited to one workspace | Do not list a catalog's full id set |
| Read one digest | `get_doc_digest` with the `workspace` of the search hit | — |
| Read one document body | Spark / Cursor / Codex / Claude: `get_doc_file` with a `dest_dir`, then read the returned copy. Mobile: `get_doc_content` | Do not read the data directory directly |
| Edit an existing document | `get_doc_file`, edit the copy, then `update_doc` with `source_path` set to the copy | Do not edit the data directory directly |
| Delete a document | Only when the user explicitly asks, and only when `delete_doc` is in the tool list: confirm the document, then `delete_doc` | Irreversible — never delete as a side step of another task |
| Open a Read Later link `[title](read-later:<id>)` | `get_read_later` with the 32-hex id | Do not look the entry up by title |

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

1. **Listed tools only** — Use `spark-task` and the MCP tools listed in this SKILL only.
2. **Verify after write** — After a create or update, confirm `ok` and `id`. Treat `digest_written`, `zh_written`, and `assets_copied` as present only when that write happened. Do not require `common_path`, `raw_path`, `digest_path`, `extra_paths`, or `asset_paths` on the result.
3. **HTTP errors** — 4xx/5xx surface as MCP tool errors (`isError: true`); do not treat error payloads as success.
4. **Path or paste** — a lone markdown path or pasted body uses [path-or-paste.md](references/path-or-paste.md). Do not call Host HTTP.
5. **Ids come from results** — Use workspace ids, project folders and document ids as the list / search / create results return them; never build one by hand.

## MCP Tools

| MCP tool | Group | Purpose |
|----------|-------|---------|
| `search` | Global | Search Lulu Spark's own library across workspaces or in one; returns id, title, snippet, workspace, project |
| `list_workspaces` | Workspace | List workspaces; the `Default` workspace always exists |
| `create_workspace` | Workspace | Create a workspace; returns its generated id |
| `rename_workspace` | Workspace | Rename a workspace (including `Default`); the id never changes |
| `list_projects` | Project | List the projects of one workspace; does not list documents |
| `create_project` | Project | Add a project to a workspace; does not move existing documents |
| `update_project` | Project | Change a project's title and description; the folder is kept |
| `get_doc_digest` | Document | Digest Markdown for one document id |
| `get_doc_file` | Document | Copy one document's file into `dest_dir` and return `{ok, id, path}`; the body is not returned and an existing copy is never overwritten |
| `get_doc_content` | Document | Mobile channel only: resolve a document id to its local file path |
| `create_doc` | Document | Create a document in a workspace and project |
| `update_doc` | Document | Replace an existing document body from the Markdown file at `source_path` |
| `delete_doc` | Document | Hard-delete one document; irreversible; `/mcp/spark` only |
| `get_read_later` | Read Later | One Read Later entry by id: id, url, read; `/mcp/spark` only |

Field names, limits, and optionality: live MCP schema only.

## Done

Observable completion for a write request:

- Tool call succeeded; and
- Response has `ok` and `id` (for a workspace, its generated id); and
- When a digest was written: `digest_written` is present; when skipped: it is absent; and
- When a zh companion was written: `zh_written` is present; when skipped: it is absent; and
- When companion images were sent: `assets_copied` is present; when skipped: it is absent.

## References

- Choosing a title: [choosing-a-title](references/choosing-a-title.md)
- Digest body: [digest-body](references/digest-body.md)
- Chinese companion: [chinese-companion](references/chinese-companion.md)
