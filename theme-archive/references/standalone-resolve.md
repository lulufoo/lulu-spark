# Standalone Path Resolution

> Standalone 模式下 theme-archive 自行解析 `COMMON_PATH`。

## Step 1 · Read topics.json

`{archive_root}/topics.json` → `topics[]`：

- `project` = item `dir` 或 `repo` 最后一段
- 无合适项 → `project = inbox`

从文档标题或正文语义推断 `doc-theme`（kebab-case 英文，3–5 词）。

## Step 2 · Slug and timestamp

```
slug = kebab-case(English summary of # title)
ts   = YYYYMMDDHHMM (UTC+8)
```

检查 `{archive_root}/raw/<topic-path>/` 是否已有同名 `<ts>-<slug>.md` 或 slug 冲突 → 询问用户。

## Step 3 · Header defaults

若用户文档缺少 header 字段，补全：

```markdown
# {Title}

> 创建时间：{YYYY年M月D日 HH:MM}
> 来源：theme-archive
> 导航：[distilled](../../../distilled/{COMMON_PATH}) · [digest](../../../digest/{COMMON_PATH}) · [trace](../../../trace/{COMMON_PATH})

---

{body}
```

`prefix = "../../../"`（topic-path 始终 2 段）。

## Step 4 · source_type

| 用户声明 | source_type |
|----------|-------------|
| `--source-type article` | `article` |
| `--source-type theme-line` | `theme-line` |
| 未声明 | `summary` |

## Step 5 · Hand off

解析完成后构造 Embedded 等价载荷，从 `[AR-1]` 继续 Archive Workflow。
