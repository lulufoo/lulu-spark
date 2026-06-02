# Plain HTML Adapter

> `platform`: `plain-html` · `adapter`: `plain-html@v1`

---

## Match

- 用户提供的本地 `.html` / `.htm` 文件路径
- 用户粘贴的 HTML 片段（先写入临时文件）
- Phase 0 无法识别的 URL（降级）
- WeChat 验证页拦截后的「浏览器另存为」HTML

---

## Acquire

**命令**：

```bash
cd {skill_dir}
python3 scripts/fetch_html.py /path/to/article.html -o /tmp/html-bundle.json
# 或
python3 scripts/fetch_html.py --url "https://example.com/article" -o bundle.json
```

`fetch_html.py` 启发式选取正文容器：`article`, `main`, `[role=main]`, `.article-content`, `#js_content`, 否则 `body`。

---

## Map

| Field | Source |
|-------|--------|
| `source.platform` | `plain-html` |
| `source.adapter` | `plain-html@v1` |
| `source.url` | URL 或 `null`（本地文件） |
| `meta.title` | `<title>` 或首个 `<h1>` |
| `meta.author` | `<meta name="author">` 或 null |
| `meta.publisher` | null |
| `meta.published_at` | `<meta property="article:published_time">` 或 null |
| `meta.language` | 检测或 `unknown` |
| `content.blocks` | 正文区解析 |

---

## Quirks

| 场景 | 处理 |
|------|------|
| 需登录 / JS 渲染 | 无法抓取；请用户另存 HTML 或粘贴正文 |
| 正文容器识别错误 | 用户指定 CSS 选择器：`--selector "#content"` |
| 纯 Markdown 粘贴 | 跳过 Acquire；直接 `markdown_raw` 进 bundle，Phase 2 用 `markdown-pass-through` |

---

## Phase 2

同 wechat：`python3 scripts/format_article.py bundle.json`
