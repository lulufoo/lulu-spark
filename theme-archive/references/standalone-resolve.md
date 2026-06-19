# Standalone Path Resolution

> Standalone 模式下 theme-archive 自行解析 `COMMON_PATH`，经 MCP `archive_document` 落盘。

## Step 1 · Infer project and doc-theme

语义推断 `project`（最接近 topics；无合适项 → `inbox`）。

从文档标题或正文语义推断 `doc-theme`（kebab-case 英文，3–5 词）。

## Step 2 · Slug and timestamp

```
slug = kebab-case(English summary of # title)
ts   = YYYYMMDDHHMM (UTC+8)
COMMON_PATH = <project>/<doc-theme>/<ts>-<slug>.md
```

slug 冲突 → 询问用户后再继续。

## Step 3 · Header defaults

若用户文档缺少 header 字段，补全：

```markdown
# {Title}

> 创建时间：{YYYY年M月D日 HH:MM}
> 来源：theme-archive
> 导航：[digest](../../../digest/{COMMON_PATH})

---

{body}
```

`prefix = "../../../"`（topic-path 始终 2 段）。MCP 产出者 raw header **仅含 digest 链接**。

## Step 4 · source_type

| 用户声明 | source_type |
|----------|-------------|
| `--source-type article` | `article` |
| `--source-type theme-line` | `theme-line` |
| 未声明 | `summary` |

## Step 5 · Hand off

解析完成后从 `[AR-1]` 继续 Archive Workflow，经 MCP `archive_document` 落盘。
