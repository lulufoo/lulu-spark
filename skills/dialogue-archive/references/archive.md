# Write conventions (dialogue-archive)

## Path

```text
{workspace}/.cache/dialogue-archive/<ts>-<slug>.md
```

- `slug`: kebab-case from title
- `ts`: `YYYYMMDDHHMM` (UTC+8)

## Raw header (script)

```markdown
# <Title>

> 创建时间：YYYY年M月D日 HH:MM
> 来源：dialogue-archive
> 源节点：<start>-<end>（<transcript name>）
> 落点：local-md

---

<!-- DDM:TURN_SEP:v1 -->
…turn body…
```

`$NORMALIZE --local-md` writes the 落点 line.
