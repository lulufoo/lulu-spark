# WeChat Official Account Adapter

> `platform`: `wechat` · `adapter`: `wechat@v1`

---

## Match

- URL 匹配 `https://mp.weixin.qq.com/s/{id}` 或带 query 的同一 path
- 不匹配 `mp.weixin.qq.com/mp/` 等非文章页

---

## Acquire

<HARD-GATE>
**微信文章 MUST 走 Semi-auto A。** 禁止作为第一步运行 `fetch_wechat.py`（无 `--allow-direct-curl` 时会 exit 2）。
禁止 curl 直抓后「重试一次」——超时/验证页即改用 Semi-auto A，不要重试 B。
Agent 第一步 MUST 是 `acquire_wechat_browser.py`（会 `open` 系统默认浏览器）。
</HARD-GATE>

**依赖**（一次性）：

```bash
pip3 install -r scripts/requirements.txt
```

### A · Semi-auto（Default — MUST use first）

用**系统默认浏览器**打开（保留登录/验证态），自动监听下载目录；可选 Playwright 有头自动存页。

```bash
cd {skill_dir}
python3 scripts/acquire_wechat_browser.py "https://mp.weixin.qq.com/s/{id}" -o /tmp/wechat.html
python3 scripts/fetch_html.py /tmp/wechat.html -o /tmp/wechat-bundle.json --selector "#js_content"
```

| 步骤 | 行为 |
|------|------|
| 1 | 脚本执行 `open`（macOS）/ `xdg-open`（Linux）→ **用户可见浏览器窗口** |
| 2a | 加 `--try-playwright`：额外有头 Chromium，出现 `#js_content` 后自动写入 `-o` |
| 2b | 默认：提示用户 ⌘S 另存「网页，仅 HTML」到 **Downloads** |
| 3 | 脚本监听 `~/Downloads`（或 `--downloads`），检测到稳定的新 `.html` 后复制到 `-o` |
| 4 | `fetch_html.py` → ArticleBundle → Phase 2 `format_article.py` |

**Agent 必须告知用户**：「已在浏览器打开文章；加载完成后请 ⌘S 另存 HTML 到下载文件夹，脚本会自动拾取。」

已手动保存时可跳过等待：

```bash
python3 scripts/acquire_wechat_browser.py --html ~/Downloads/文章.html -o /tmp/wechat.html
python3 scripts/fetch_html.py /tmp/wechat.html -o /tmp/wechat-bundle.json --selector "#js_content"
```

### B · Direct curl（Disabled by default）

仅调试或用户明确要求 headless 时使用；**不得**作为首次 Acquire。

```bash
python3 scripts/fetch_wechat.py "https://mp.weixin.qq.com/s/{id}" -o bundle.json --allow-direct-curl
```

- 易超时（exit 28）或验证页 → **立即**改 Semi-auto A，**不要**重试 B

---

## Map

脚本输出 conform [bundle-schema.md](../bundle-schema.md)：

| Field | Source |
|-------|--------|
| `source.platform` | Semi-auto 经 `fetch_html` 为 `plain-html`；Phase 3 index 仍记 `fetch.platform=wechat` |
| `source.adapter` | `plain-html@v1` / 调试 curl 时为 `wechat@v1` |
| `source.url` | 输入 URL |
| `source.fetched_at` | 采集时刻 ISO8601 (UTC+8) |
| `meta.title` | `og:title` 或页面 title |
| `meta.author` | 文中 `author` 字段或 `js_name` |
| `meta.publisher` | 公众号名 `nickname` / `#js_name` |
| `meta.published_at` | `create_time` 或 `#publish_time` |
| `meta.language` | 正文检测 → 默认 `zh` |
| `content.blocks` | `#js_content` 解析：段落、heading、table、image |

---

## Quirks

| 场景 | 处理 |
|------|------|
| 验证页 / 环境异常 | Semi-auto A；在已打开浏览器内完成验证后再另存 |
| 用户说没看到浏览器 | 检查是否误跑 `fetch_wechat.py`；改跑 `acquire_wechat_browser.py` |
| curl 超时 | **禁止重试 curl**；改 Semi-auto A |
| 下载目录非默认 | `--downloads /path/to/dir` |
| 标题/章节粘连 | `format_article.py` 后处理 |

---

## Phase 2

```bash
python3 scripts/format_article.py /tmp/wechat-bundle.json
```

输出 Markdown **正文**（不含 Archive header）。
