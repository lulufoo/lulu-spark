# Output Templates

## Archive header (Phase 3)

```markdown
# {title}

> 创建时间：{YYYY年MM月DD日 HH:MM}
> 
> 来源：theme-fetch / {platform}
> 
> 原文：{url}
> 
> 刊载：{publisher} · 作者 {author} · 发布 {published_at}
> 
> 导航：[digest]({prefix}digest/{COMMON_PATH})

---

{author_line}

{body}
```

- `{author_line}`：`作者 | {author}`；无 author 则省略
- `{publisher}` / `{author}` / `{published_at}` 缺失时省略对应片段
- `{url}` 使用 Markdown 链接 `[url](url)`

## Body (Phase 2 output)

- 以 Phase 2 Format 输出粘贴，不再压缩
- 不含 `来源：` 重复行（已在 header）
