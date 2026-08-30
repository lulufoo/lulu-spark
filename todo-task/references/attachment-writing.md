# Attachment Writing

SSOT for Todo attachments under Todo Norms.

## Explicit request required

Add, update, or replace an attachment only when the user explicitly asks to do so.
Do not infer an attachment from a long analysis, a diagnosis, or a document-shaped
body.

An explicit request identifies either the source content or the intended attachment
type. If its scope is unclear, resolve the scope before writing.

## Attachment types

| Type | Purpose | Required content |
|------|---------|------------------|
| Analysis | Preserve the detailed investigation or decision record behind a Todo | Scenario, evidence checked, reasoning, decision or open question, and verification target |
| Dialogue | Preserve a user-requested raw decision trail | A confirmed transcript range, normalized by `dialogue-archive` with `sink=local-md` |
| Evidence | Preserve material needed to reproduce or verify the work | Minimal reproduction, command output, logs, screenshots, or an external primary source |

For a dialogue attachment:

1. The user must explicitly request the attachment.
2. Resolve a proposed node range from the user's stated topic or anchor.
3. Ask the user to confirm that range before normalization.
4. Use `dialogue-archive` with `sink=local-md`.
5. Attach the resulting local Markdown through the Todo attachment tool.

`local-md` means do not call `create_note`; the dialogue
must not enter Workbench notes. Copying the local file into Todo attachments
is still required when the user requested an attachment.

## Todo body manifest

When an attachment is added or replaced, add a concise `## Attachments` list in the
Todo header/reference area:

```markdown
## Attachments

- `202608091051-test-sandbox-race-analysis.md` — Analysis; root cause and acceptance targets.
- `202608091051-test-sandbox-race-dialogue.md` — Dialogue; confirmed nodes 239–307; local-md source.
```

Use a descriptive filename and state the type, purpose, and source scope. Keep the
body list concise; the attachment holds the full record.

## Safety and verification

- Never attach credentials, tokens, private keys, or unredacted personal configuration.
- Read the live MCP schema before each attachment operation.
- Use only path-based Todo attachment tools; never send an attachment body as a tool
  argument.
- After each add or update, list the Todo attachments and verify the expected file is
  present.
