# Digest body

Load when composing `digest_body` and `digest` is `auto` or `always`.

## Whether to write

`digest` is required on `create_note`.

| Value | Do |
|-------|----|
| `never` | Do not pass `digest_body`. |
| `always` | Pass a non-empty `digest_body`. |
| `auto` | Pass `digest_body` when any rule below matches. Otherwise omit it. No `digest_path` means it was skipped. |

Write a digest when any of these is true:

- The raw has 2 or more Turn blocks (`## Turn N` or `<!-- DDM:TURN_SEP:v1 -->`).
- The raw has 2 or more theme-level `## ` headings that are not `## Turn`.
- The body after `---` is at least 600 characters.
- `source_type` is `summary` and the body is at least 200 characters.
- `source_type` is `article` and the body is at least 600 characters.

## Shape

```markdown
# [Title] — Summary

> Created: [same time as the raw]

## Overview

[One paragraph: what the note is about and where it landed. From the raw only. No chapter list.]
```

No digest nav line. No relative images. A `digest_body` that contains them is rejected.

## Overview

One paragraph, about 80–300 characters. Source is the raw only. No archive jargon.

dialogue-*: follow the given content constraint. Do not write past it. Do not reload the full raw only to draft the digest. Put the constraint in the header:

```markdown
> Content constraint: <content_constraint as given>
```

The constraint is required for dialogue-* and recommended otherwise. It is not an MCP field.

theme-line: one digest on the primary raw, not on `-zh.md`.

## Submit

Pass `digest_body` on the same `create_note` call. On success, `digest_path` is present when a digest was written and absent when `auto` skipped it.
