# Archive conventions (dialogue-archive)

## Sink scope

| `sink` | MCP |
|--------|-----|
| `spark` | This document applies in full |
| `local-md` | Skip MCP; write workspace `.cache` only — do **not** write the notes store by hand |

## MCP prerequisite (`sink=spark` only)

Lulu Spark App **must** be running.  
**Do not** write notes `raw/` / `digest/` / `index.json` directly. Use MCP only. Load note-task and route Create.

| Op | MCP tool |
|----|----------|
| Write raw (+ digest per `digest` field) | `create_note` |

First step when archiving to Lulu Spark: confirm MCP available.

## Staging path

```text
{workspace}/.cache/dialogue-archive/<ts>-<slug>.md
```

Host assigns `common_path` on Create. Do not invent a notes write identity.

- `project`: closest topics match; else `inbox`
- `doc-theme` / `slug`: kebab-case (staging filename only)
- `ts`: `YYYYMMDDHHMM` (UTC+8) for the staging filename

## Raw header (script)

```markdown
# <Title>

> 创建时间：YYYY年M月D日 HH:MM
> 来源：dialogue-archive

---

<!-- DDM:TURN_SEP:v1 -->
…turn body…
```

Host rewrites the raw shell.

`sink=local-md`: script adds `> 落点：local-md` when `--local-md` is passed.

## create_note

Hard-cut: body comes from Host reading a file. **Do not** send `document`. Fields: live MCP schema (`title` + `source_path` + `digest` on desktop).

If the tool/API still rejects `source_path` or requires `document`, stop and report Host/MCP not yet upgraded — do **not** paste the full markdown into the tool call.

`digest_body` writing contract: non-empty **内容约束** must appear in the digest header:

```markdown
> 内容约束：<content_constraint>
```

Do not re-load full raw solely to draft the digest.
