# WeChat Official Account Adapter

> `platform`: `wechat` · `adapter`: `wechat@v1`

---

## Match

- URL 匹配 `https://mp.weixin.qq.com/s/{id}` 或带 query 的同一 path
- 不匹配 `mp.weixin.qq.com/mp/` 等非文章页

---

## Acquire（推荐顺序）

**依赖**（一次性）：

```bash
pip3 install -r scripts/requirements.txt
```

### A · Semi-auto（Recommended）

用**系统默认浏览器**打开（保留登录/验证态），自动监听下载目录；可选 Playwright 有头自动存页。

```bash
cd {skill_dir}
python3 scripts/acquire_wechat_browser.py "https://mp.weixin.qq.com/s/{id}" -o /tmp/wechat.html
python3 scripts/fetch_html.py /tmp/wechat.html -o /tmp/wechat-bundle.json --selector "#js_content"
```

| 步骤 | 行为 |
|------|------|
| 1 | Agent 执行脚本 → macOS `open` / Linux `xdg-open` 打开 URL |
| 2a | `--try-playwright`：有头浏览器，出现 `#js_content` 后自动写入 `-o`（需 `pip install playwright`） |
| 2b | 默认：提示用户 ⌘S 另存「网页，仅 HTML」到 **Downloads** |
| 3 | 脚本监听 `~/Downloads`（或 `--downloads`），检测到稳定的新 `.html` 后复制到 `-o` |
| 4 | `fetch_html.py` → ArticleBundle → Phase 2 `format_article.py` |

已手动保存时可跳过等待：

```bash
python3 scripts/acquire_wechat_browser.py --html ~/Downloads/文章.html -o /tmp/wechat.html
```

### B · Direct curl（Best-effort）

网络快且无验证页时可用；易超时或被「环境异常」拦截。

```bash
python3 scripts/fetch_wechat.py "https://mp.weixin.qq.com/s/{id}" -o /tmp/wechat-bundle.json
```

- 成功：stderr `OK`；`-o` 写入 ArticleBundle JSON
- 失败：改用 **A** 或浏览器另存 → `plain-html`

调试保留 HTML：

```bash
python3 scripts/fetch_wechat.py "{url}" -o bundle.json --save-html /tmp/wechat.html
```

---

## Map

脚本输出 conform [bundle-schema.md](../bundle-schema.md)：

| Field | Source |
|-------|--------|
| `source.platform` | `wechat`（Semi-auto 经 fetch_html 时为 `plain-html`，meta 仍可用） |
| `source.adapter` | `wechat@v1` / `plain-html@v1` |
| `source.url` | 输入 URL |
| `source.fetched_at` | 采集时刻 ISO8601 (UTC+8) |
| `meta.title` | `og:title` 或页面 title |
| `meta.author` | 文中 `author` 字段或 `js_name` |
| `meta.publisher` | 公众号名 `nickname` / `#js_name` |
| `meta.published_at` | `create_time` 或 `#publish_time` |
| `meta.language` | 正文检测 → 默认 `zh` |
| `content.blocks` | `#js_content` 解析：段落、heading、table、image |

Semi-auto 完成后可在 bundle `source` 上追加注释 URL 为原始微信链接（index `fetch.url` 用 Phase 3 传入）。

---

## Quirks

| 场景 | 处理 |
|------|------|
| 验证页 / 环境异常 | 用 **Semi-auto A**；在已打开浏览器内完成验证后再另存 |
| curl 超时 | 不要用 B；改用 **Semi-auto A** |
| 下载目录非默认 | `--downloads /path/to/dir` |
| 标题/章节粘连 | `format_article.py` 后处理；必要时人工微调 raw |
| 推广引流句 | `fetch_wechat` / `fetch_html` 共用过滤 |

---

## Phase 2

```bash
python3 scripts/format_article.py /tmp/wechat-bundle.json
```

输出 Markdown **正文**（不含 Archive header）。
