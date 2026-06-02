# Format Rules — Phase 2 (F0–F4)

> 输入：ArticleBundle · 输出：Markdown body（**不含** Archive header）

---

## F0 · Select strategy

| Strategy | Condition | Action |
|----------|-----------|--------|
| `structured-blocks` | `len(content.blocks) > 0` | F1 |
| `markdown-pass-through` | `content.markdown_raw` 非空 | 直接使用，仅 F4 空白规范化 |
| fail-fast | 无 title 且无 body | 中止 |

---

## F1 · Render blocks

| `type` | Markdown |
|--------|----------|
| `heading` | `{"#"*level} {text}` |
| `paragraph` | 空行 + 段落 + 空行 |
| `table` | GFM table；`rows[0]` 为表头 |
| `list` | `- item` 或 `1. item` |
| `code` | fenced code；默认 `text` |
| `image` | `![image](url)` |
| `quote` | `> text` |

---

## F2 · Normalize headings (WeChat / CN articles)

当 `source.platform == wechat` 或 blocks 含章节编号模式：

- 独立行 `01`–`11` + 下一标题 → `## 01 {title}`
- 文本 `1.1` / `1.1 xxx` → `### 1.1 xxx`
- 节点小标题「需求分析节点」→ `#### 需求分析节点`

由 `format_article.py` 在 render 前预处理 blocks（见脚本 `--normalize-headings`）。

---

## F3 · Flow diagrams

段落内同时含 `→` 与 `↓` 且长度 > 40 → 转为 ` ```text ` 块，逐步骤换行。

---

## F4 · Whitespace

- 连续空行压缩为最多 1 行
- 段内错误换行（行首 `——` 续接上一行）合并
- **不**改正文措辞

---

## Script entry

```bash
python3 scripts/format_article.py bundle.json [--normalize-headings]
```

stdout = Markdown body only.
