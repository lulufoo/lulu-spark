---
name: todo-task
description: >-
  Todo-task MCP workflow: create / query / update Workbench todo tasks via MCP only.
  Use when: todo-task, todo task, MCP todo, create_todo_task, list_todo_tasks, todo_md.
argument-hint: '[task title | task id]'
---

# todo-task — Todo Task MCP Workflow

## Boundary

- Todo data is written **only** by the Workbench `todo_task` service.
- Agent **MUST NOT** write `todo_tasks/` on disk, call Host HTTP directly, or invoke Rust services.
- All reads/writes go **MCP → HTTP → `todo_task` service** (I-11). MCP is a proxy only (I-3).

## Parameter SSOT

Before every call, read the live MCP tool `description` / `inputSchema`.

- This SKILL does **not** restate field contracts (they change with MCP).
- Prefer the live schema over any remembered or cached shape.
- If schema and Host behavior disagree, stop and report the mismatch; do not invent parameters.

## MCP Prerequisite

<HARD-GATE mcp="todo">
Workbench App **MUST** be running, and the `knowledge-mcp` sidecar spawned.

- **`WORKBENCH_HTTP_URL`**: sidecar proxy target; default `http://127.0.0.1:8765` (local_http).
- If Host is down or HTTP unreachable, todo tools will not appear — start Workbench App first.
</HARD-GATE>

## Intent Routing

Map the user request, then call **only** the tools that match. Do not run a fixed pipeline.

| User intent | Do | Do not |
|-------------|----|--------|
| Store / save document or notes as todo body | `create_todo_task` with `todo_md` (or update body if Host/MCP exposes an update-body tool) | Add sub-tasks without an explicit ask |
| Create an empty todo (title only) | `create_todo_task` with title; omit `todo_md` | Add sub-tasks without an explicit ask |
| Add / remove a sub-task | `add_todo_sub` / `delete_todo_sub` | Infer subs from document structure |
| Complete master or sub | `complete_todo` (`master_task_id` required; `sub_task_id` optional — omit → complete master; with sub → complete that sub) | Call removed `complete_plan_sub` or any `plan_*` tool |
| Link archive to a **completed** sub | `link_todo_archive` (after archive exists) | Link without a completed sub + archive id |
| Attachments | `add_todo_attachment` / `list_todo_attachments` / `get_todo_attachment` / `update_todo_attachment` | — |
| Inspect | `list_todo_tasks` / `get_todo_task` | — |
| Delete master | `delete_todo_task` | — |

**Body vs tree:** todo body is `todo_md`. Sub-tasks are a separate tree. Saving content into a todo means writing **body**; add sub-tasks only on explicit user instruction.

## Hard Constraints

1. **Sub-tasks require an explicit user instruction** — `add_todo_sub` (and any create-time sub-title list) is **allowed only when** the user clearly asks to add, split, or manage sub-tasks. Silence, “save to todo”, or document structure alone is **not** such an instruction — do not infer subs.
2. **Create leaves empty subs by default** — New masters start with empty `sub_tasks`. If the same turn also contains an explicit sub-task request, add them with `add_todo_sub` after create (per live schema).
3. **Verify after write** — After create/update, `get_todo_task` (or list) and confirm `todo_md` / title / subs match the request.
4. **HTTP errors** — 4xx/5xx surface as MCP tool errors (`isError: true`); do not treat error payloads as success.
5. **No `/plan-task` / `plan_*` dependency** — Agent must use `todo-task` and `todo_*` tools only; old plan names are unavailable.

## MCP Todo Tools

| MCP tool | Purpose |
|----------|---------|
| `create_todo_task` | Create master (title + optional `todo_md` body) |
| `list_todo_tasks` | List all masters |
| `get_todo_task` | Get one master by id |
| `delete_todo_task` | Delete master |
| `add_todo_sub` | Add a sub-task (only when the user explicitly instructs) |
| `delete_todo_sub` | Delete a sub-task |
| `complete_todo` | Complete a todo task. `master_task_id` required; `sub_task_id` optional. Omit `sub_task_id` → complete master; with `sub_task_id` → complete that sub. Replaces removed `complete_plan_sub` (do not call the old name). |
| `link_todo_archive` | Link archive entry id to a completed sub |
| `add_todo_attachment` | Add an attachment |
| `list_todo_attachments` | List attachments |
| `get_todo_attachment` | Get one attachment |
| `update_todo_attachment` | Update an attachment |

Field names, limits, and optionality: live MCP schema only. Old tool names (`create_plan_task`, `complete_plan`, `complete_plan_sub`, …) are unavailable; do not call them.

## Done

Observable completion for a write request:

- Tool call succeeded; and
- `get_todo_task` / `list_todo_tasks` shows the expected title, `todo_md`, and sub-task set (subs added only when the user explicitly instructed).

## References

- Workbench tech plan: MCP todo tools (FM-3 HTTP + FM-6 SKILL)
- Archive then link: [theme-archive](../theme-archive/SKILL.md) → `link_todo_archive`
