---
name: theme-fetch
description: >-
  Fetch web articles from multiple platforms (WeChat, plain HTML, …) via ArticleBundle
  adapters, format as readable Markdown, save to raw/ and auto digest. Use for 采集文章、
  抓取链接、归档公众号、theme-fetch、mp.weixin.qq.com、网页归档.
---

# ThemeFetch

> **4 mandatory phases:**
> 0. Resolve Source
> 1. Acquire → ArticleBundle
> 2. Format Article
> 3. Save to Archive
>
> Read this file in full before executing. Phase 3 saves via Workbench MCP (`archive_document` + `archive_digest`).

Fetch external web articles, normalize structure, archive to `raw/` — **no summarization**, **no ThemeLine**.

## Phase 0 · Resolve Source

| Input | `platform` | Doc |
|-------|-----------|-----|
| `mp.weixin.qq.com/s/*` | `wechat` | [adapters/wechat.md](references/adapters/wechat.md) |
| Local `.html` path, pasted HTML, unknown URL | `plain-html` | [adapters/plain-html.md](references/adapters/plain-html.md) |

Full routing: [adapters/README.md](references/adapters/README.md)

**Boundary with theme-line:** `infoq.cn/video/*` → theme-line (transcript). `infoq.cn/article/*` full text → future `infoq-article` adapter; until then use `plain-html` or paste.

## Phase 1 · Acquire

<HARD-GATE>
MUST read the matching adapter doc before any fetch. Do NOT improvise platform-specific commands.
</HARD-GATE>

<HARD-GATE platform="wechat">
`mp.weixin.qq.com/s/*` 第一步 MUST 运行：

```bash
python3 scripts/acquire_wechat_browser.py "<url>" -o /tmp/wechat.html
python3 scripts/fetch_html.py /tmp/wechat.html -o /tmp/wechat-bundle.json --selector "#js_content"
```

**禁止**先运行 `fetch_wechat.py`（默认已禁用 curl）。执行后浏览器 MUST 被打开；若未打开则说明用错了脚本。
</HARD-GATE>

- Output MUST conform to [references/bundle-schema.md](references/bundle-schema.md)
- Read `references/adapters/{platform}.md` → Match / Acquire / Map / Quirks
- Prefer adapter scripts under `scripts/` when documented
- Bundle is ephemeral (memory only); optional debug: `{archive_root}/.cache/{topic-path}/{ts}-{slug}-bundle.json`
- **禁止** persist bundle to `trace/`

## Phase 2 · Format

Read [references/format-rules.md](references/format-rules.md) — steps F0–F4.

| Strategy | Condition |
|----------|-----------|
| `structured-blocks` | `content.blocks.length > 0` |
| `markdown-pass-through` | `content.markdown_raw` non-empty |
| fail-fast | no title AND no body → stop, do not Archive |

Run:

```bash
python3 scripts/format_article.py bundle.json
```

unless adapter doc specifies otherwise.

**Fidelity:** preserve original wording; fix layout only (headings, tables, flow diagrams). Do not summarize or rewrite.

Output patterns: [references/output-templates.md](references/output-templates.md)

## Phase 3 · Save to Archive

Load [references/archive-steps.md](references/archive-steps.md) — Step 4 **archive_document** · Step 5 **archive_digest**（MCP）。

Path/config: [../shared/archive-concepts.md](../shared/archive-concepts.md)

## Title Handling

- URL source → use `bundle.meta.title` unless user overrides
- User custom title → prefer user's title
- slug → kebab-case English from title

## Ask Only When Necessary

Defaults: source title · infer project semantically · `language` from bundle · auto digest

Ask only when: slug conflict · fetch blocked (verification page) · project ambiguous

## References

| Doc | Purpose |
|-----|---------|
| [bundle-schema.md](references/bundle-schema.md) | ArticleBundle contract |
| [adapters/](references/adapters/) | Platform Acquire docs |
| [format-rules.md](references/format-rules.md) | Format rules F0–F4 |
| [archive-steps.md](references/archive-steps.md) | Save to Archive Steps 1–9 |
| [output-templates.md](references/output-templates.md) | Header / body patterns |
