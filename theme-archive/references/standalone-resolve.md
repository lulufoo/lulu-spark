# Standalone Path Resolution

> Standalone 模式下 theme-archive 推断 `project` / `theme` / `title` / `created_at`，经 MCP `create_note` 落盘。`common_path` 由 Host 返回。

## Step 1 · Infer project and theme

语义推断 `project`（最接近 topics；无合适项 → `inbox`）。

从文档标题或正文语义推断 `theme`（kebab-case 英文，3–5 词）。

## Step 2 · Title and timestamp

```
title = # heading or first line
created_at = YYYYMMDDHHMM (UTC+8); omit to let Host use now
```

不要再拼 `{ts}-{slug}` 作为写入身份。

## Step 3 · Header defaults

若用户文档缺少标题或正文分隔，补全：

```markdown
# {Title}

> 创建时间：{YYYY年M月D日 HH:MM}

---

{body}
```

不要写入 digest 导航行。Host 会重写 raw 壳。

## Step 4 · source_type

| 用户声明 | source_type |
|----------|-------------|
| `--source-type article` | `article` |
| `--source-type theme-line` | `theme-line` |
| 未声明 | `summary` |

## Step 5 · Hand off

解析完成后从 `[AR-1]` 继续 Archive Workflow（含 `[AR-1b]` 全文英文中译），经 MCP 落盘。
