# Digest writing shape

Load when composing `digest_body` for `create_note` (`digest` is `auto` or `always`). Not a public skill.

Path symbols: [archive-concepts](archive-concepts.md).

## When Host writes

`digest` is required on `create_note`: `auto` | `always` | `never`.

| Value | Host |
|-------|------|
| `never` | No digest |
| `always` | Require non-empty `digest_body`; write `digest/<COMMON_PATH>` |
| `auto` | Require `digest_body` if any AD-0 rule matches; otherwise skip |

### AD-0

```text
· raw has 2+ Turn blocks (`## Turn N` or `<!-- DDM:TURN_SEP:v1 -->`)
· raw has 2+ theme-level `## ` headings that are not `## Turn`
· body after `---` is ≥ 600 characters
· source_type is summary and body ≥ 200
· source_type is article and body ≥ 600
```

`auto` and none match → no digest; response has no `digest_path`.

## Shape (AD-1)

```markdown
# [Title] — 摘要

> 创建时间：[same ts as raw]

## 概述

[One paragraph: what the note is about and where it landed. From raw only. No chapter list.]
```

No digest nav line. Digest and raw share `common_path` and `layers`.

## Content constraint (AD-1b)

Required for dialogue-* ; recommended otherwise. Not an MCP field.

Put this in the digest header:

```markdown
> 内容约束：<content_constraint as given>
```

Do not reload full raw only to draft the digest. Do not write past the constraint.

## Writing (AD-2)

```text
· Source: raw only (dialogue-*: dialogue context + constraint, not a full raw reload)
· Overview: one paragraph, about 80–300 characters
· No DDM / archive jargon
· theme-line: one digest on the primary raw, not on `-zh.md`
· No relative images (`![…](./…)` / `<img src="local">`). Host rejects. Do not copy raw figures into digest.
```

## Submit (AD-3)

Put AD-1–AD-2 in the same `create_note` call as `digest_body`. Check `digest_path` on success; absent means `auto` skipped. Relative images in `digest_body` → Host 400; rewrite without them.
