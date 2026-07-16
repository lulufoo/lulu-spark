---
name: plan-task
description: >-
  Plan-task MCP workflow: create / query / update Workbench plan tasks via MCP only.
  Use when: plan-task, plan task, MCP plan, create_plan_task, list_plan_tasks, plan_md.
argument-hint: '[task title | task id]'
---

# plan-task — Plan Task MCP Workflow

## Boundary

- Plan data is written **only** by the Workbench `plan_task` service.
- Agent **MUST NOT** write `plan_tasks/` on disk, call Host HTTP directly, or invoke Rust services.
- All reads/writes go **MCP → HTTP → `plan_task` service** (I-11). MCP is a proxy only (I-3).

## Parameter SSOT

Before every call, read the live MCP tool `description` / `inputSchema`.

- This SKILL does **not** restate field contracts (they change with MCP).
- Prefer the live schema over any remembered or cached shape.
- If schema and Host behavior disagree, stop and report the mismatch; do not invent parameters.

## MCP Prerequisite

<HARD-GATE mcp="plan">
Workbench App **MUST** be running, and the `knowledge-mcp` sidecar spawned.

- **`WORKBENCH_HTTP_URL`**: sidecar proxy target; default `http://127.0.0.1:8765` (local_http).
- If Host is down or HTTP unreachable, plan tools will not appear — start Workbench App first.
</HARD-GATE>

## Intent Routing

Map the user request, then call **only** the tools that match. Do not run a fixed pipeline.

| User intent | Do | Do not |
|-------------|----|--------|
| Store / save document or notes as plan body | `create_plan_task` with `plan_md` (or update body if Host/MCP exposes an update-body tool) | Add sub-tasks without an explicit ask |
| Create an empty plan (title only) | `create_plan_task` with title; omit `plan_md` | Add sub-tasks without an explicit ask |
| Add / remove / complete a sub-task | `add_plan_sub` / `delete_plan_sub` / `complete_plan_sub` | Infer subs from document structure |
| Link archive to a **completed** sub | `link_plan_archive` (after archive exists) | Link without a completed sub + archive id |
| Inspect | `list_plan_tasks` / `get_plan_task` | — |
| Delete master | `delete_plan_task` | — |

**Body vs tree:** plan body is `plan_md`. Sub-tasks are a separate tree. Saving content into a plan means writing **body**; add sub-tasks only on explicit user instruction.

## Hard Constraints

1. **Sub-tasks require an explicit user instruction** — `add_plan_sub` (and any create-time sub-title list) is **allowed only when** the user clearly asks to add, split, or manage sub-tasks. Silence, “save to plan”, or document structure alone is **not** such an instruction — do not infer subs.
2. **Create leaves empty subs by default** — New masters start with empty `sub_tasks`. If the same turn also contains an explicit sub-task request, add them with `add_plan_sub` after create (per live schema).
3. **Verify after write** — After create/update, `get_plan_task` (or list) and confirm `plan_md` / title / subs match the request.
4. **HTTP errors** — 4xx/5xx surface as MCP tool errors (`isError: true`); do not treat error payloads as success.

## MCP Plan Tools

| MCP tool | Purpose |
|----------|---------|
| `create_plan_task` | Create master (title + optional `plan_md` body) |
| `list_plan_tasks` | List all masters |
| `get_plan_task` | Get one master by id |
| `delete_plan_task` | Delete master |
| `add_plan_sub` | Add a sub-task (only when the user explicitly instructs) |
| `delete_plan_sub` | Delete a sub-task |
| `complete_plan_sub` | Mark a sub-task complete |
| `link_plan_archive` | Link archive entry id to a completed sub |

Field names, limits, and optionality: live MCP schema only.

## Done

Observable completion for a write request:

- Tool call succeeded; and
- `get_plan_task` / `list_plan_tasks` shows the expected title, `plan_md`, and sub-task set (subs added only when the user explicitly instructed).

## References

- Workbench tech plan: MCP plan tools (FM-3 HTTP + FM-6 SKILL)
- Archive then link: [theme-archive](../theme-archive/SKILL.md) → `link_plan_archive`
