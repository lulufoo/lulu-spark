# WeChat Official Account Adapter

> `platform`: `wechat` · `adapter`: `wechat@v1`

---

## Match

- URL 匹配 `https://mp.weixin.qq.com/s/{id}` 或带 query 的同一 path
- 不匹配 `mp.weixin.qq.com/mp/` 等非文章页

---

## Acquire

**依赖**（一次性）：

```bash
pip3 install -r scripts/requirements.txt
```

**命令**（MUST 使用，勿即兴重写解析逻辑）：

```bash
cd {skill_dir}
python3 scripts/fetch_wechat.py "https://mp.weixin.qq.com/s/{id}" -o /tmp/wechat-bundle.json
```

- 成功：stdout 打印 `OK`；`-o` 写入 ArticleBundle JSON
- 失败：非零 exit；stderr 含原因（验证页、超时、无 `#js_content`）

可选调试保留 HTML：

```bash
python3 scripts/fetch_wechat.py "{url}" -o bundle.json --save-html /tmp/wechat.html
```

---

## Map

脚本输出 conform [bundle-schema.md](../bundle-schema.md)：

| Field | Source |
|-------|--------|
| `source.platform` | `wechat` |
| `source.adapter` | `wechat@v1` |
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
| 页面含「环境异常」/ 验证 | 中止；提示用户在浏览器打开后「另存 HTML」→ `plain-html` adapter |
| curl 超时 | 重试一次；仍失败则中止 |
| 标题/章节粘连 | 由 `format_article.py` 后处理；必要时人工微调 raw |
| 表格在 HTML 中为 `<table>` | 脚本 Map 为 `type: table` |
| 推广引流句「关注xxx公众号」 | 脚本过滤常见 patterns |

---

## Phase 2

```bash
python3 scripts/format_article.py /tmp/wechat-bundle.json
```

输出 Markdown **正文**（不含 Archive header）。
