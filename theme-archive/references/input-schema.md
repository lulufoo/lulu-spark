# Embedded Input Schema

> Producer（theme-fetch、theme-line、dialogue-*）链式调用 theme-archive 时使用的载荷。

## Required fields

```yaml
COMMON_PATH: "<topic-path>/<ts>-<slug>.md"

documents:
  - rel: "raw/<COMMON_PATH>"           # relative to archive_root
    content: "<完整 Markdown，含 header + 正文>"

index_entry:
  common_path: "<topic-path>/<ts>-<slug>.md"
  created_at: "<ts>"                   # YYYYMMDDHHMM
  source_type: "summary | article | theme-line"
  layers: ["raw"]
```

## Optional fields

```yaml
id: "<32-char hex>"                    # 省略则由 theme-archive 生成

documents:                             # 附加 raw 文件
  - rel: "raw/<topic-path>/<ts>-<slug>-zh.md"
    content: "<中文翻译全文>"

index_entry:
  fetch:                               # theme-fetch only
    platform: "wechat"
    adapter: "wechat"
    url: "https://mp.weixin.qq.com/s/..."
  translations:                        # 英文源 + -zh.md
    zh: "<topic-path>/<ts>-<slug>-zh.md"
```

## Producer mapping

| Producer | source_type | Typical extra |
|----------|-------------|---------------|
| theme-fetch | `article` | `fetch` |
| theme-line | `theme-line` | `translations` when `language == en` |

## Call pattern

```text
Phase N 组稿完成 → 构造上述载荷 →
加载并完整执行 ../theme-archive/SKILL.md（Embedded，从 [AR-1] 起）
```

Producer 仍负责：选 project/doc-theme、组 header、Format/Compose 正文。若走 theme-archive skill，digest 由 `[AR-3]` 自动执行；若自管 MCP `archive_document`，须按 [../../shared/digest-workflow.md](../../shared/digest-workflow.md) Embedded 补 digest。

theme-archive 负责：**仅**写 raw、更新 index（`layers: ["raw"]`）。**不**触发 digest。
