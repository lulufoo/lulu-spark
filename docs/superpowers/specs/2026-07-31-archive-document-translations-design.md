# archive_document 译稿字段简化

> 状态：已批准  
> 日期：2026-07-31  
> 决策：调用面采用 `translations[{lang,content}]`；旧字段硬切

---

## 问题

✅ Verified（`packages/knowledge-mcp/index.mjs`、`src-tauri/src/services/archive_write.rs`）

当前 `archive_document` 要求调用方同时提供：

1. `extra_documents[].rel` + `content`
2. `index_extra.translations.zh`（与 `rel` 去 `raw/` 后路径一致）

而 RS 已能从主文档 `common_path` 推出 `-zh.md` 路径，并强制两边一致。路径被算三遍，SKILL / MCP 面臃肿。

---

## 目标

- SKILL → MCP → HTTP 请求同形、只表达意图（主文 + 译稿 list）
- MCP 薄代理，透传 body
- 路径派生与 `index.translations` 写入由 RS 消化
- 旧字段硬切（不兼容）

非目标：前端多语言 UI、非译稿附件、改动 `archive_digest` 契约。

---

## 请求契约

SKILL / MCP / HTTP 同形：

```json
{
  "document": "<主 raw 全文 Markdown>",
  "source_type": "theme-line",
  "translations": [
    { "lang": "zh", "content": "<译稿全文>" },
    { "lang": "fr", "content": "<译稿全文>" }
  ]
}
```

| 字段 | 必填 | 说明 |
|------|------|------|
| `document` | 是 | 主体 |
| `source_type` | 否 | 默认 `summary` |
| `translations` | 否 | 缺省或 `[]` = 无译稿 |
| `translations[].lang` | 项内必填 | ISO 639-1 两字母小写（`zh` / `fr`） |
| `translations[].content` | 项内必填 | 非空 Markdown |

已有任务关联字段（`master_task_id` / `sub_task_id`）保持不变。✅ Verified（`archive_write.rs` `parse_task_ref`）

---

## RS 消化规则

主文档解析出 `common_path` 后，对每个 translation：

```
common_path = topic/doc/202606191700-slug.md
→ file       = raw/topic/doc/202606191700-slug-{lang}.md
→ index      = translations.{lang} = topic/doc/202606191700-slug-{lang}.md
```

规则：`stem + "-" + lang + ".md"`。  
现有 `expected_zh_common_path`（`archive_parse.rs`）是 `lang=zh` 特例，本设计泛化为 `expected_lang_common_path(primary, lang)`。✅ Verified（现有 zh 规则）/ ⚠️ Inferred（泛化命名与多 lang）

index 落盘仍为 map（调用面 list，存储不变）：

```json
"translations": {
  "zh": "topic/doc/202606191700-slug-zh.md",
  "fr": "topic/doc/202606191700-slug-fr.md"
}
```

---

## 校验（RS → 400 / 409）

| 条件 | 状态 |
|------|------|
| `lang` 不匹配 `^[a-z]{2}$` | 400 |
| 同一请求 `lang` 重复 | 400 |
| `content` trim 后为空 | 400 |
| 主 `common_path` 无法派生译稿路径 | 400 |
| 请求含 `extra_documents` 或 `index_extra` | 400（硬切，提示改用 `translations`） |
| 译稿文件已存在 | 409 |

写失败时回滚已写文件，保持现有 dual-store 行为。✅ Verified（`archive_write.rs` rollback）

---

## 返回值

```json
{
  "ok": true,
  "id": "<32 hex>",
  "common_path": "topic/doc/202606191700-slug.md",
  "raw_path": "raw/topic/doc/202606191700-slug.md",
  "created_at": "...",
  "extra_paths": [
    "raw/topic/doc/202606191700-slug-zh.md",
    "raw/topic/doc/202606191700-slug-fr.md"
  ]
}
```

无译稿时省略 `extra_paths`。✅ Verified（现有响应形状）

---

## 链路与改造点

```
theme-line SKILL
  → knowledge-mcp archive_document（schema + 透传）
  → POST /api/archive-document（路由透传）
  → archive_write.rs（消化 translations）
  → raw/ + index.json
```

| # | 位置 | 改动 |
|---|------|------|
| 1 | `packages/knowledge-mcp/index.mjs` | schema 换 `translations`；删除 `extra_documents` / `index_extra` |
| 2 | `src-tauri/src/services/archive_write.rs` | 解析 `translations[]`；派生路径；写 index；拒收旧字段 |
| 3 | theme-line `references/archive-steps.md` | Step 5 改新结构 |
| 4 | `src-tauri/.../unit-tests/services/archive_write.rs` | theme-line / mismatch 用例跟新；补多 lang |
| 5 | `packages/knowledge-mcp/scripts/verify.mjs` | 跟新 schema |
| 6 | `archive_parse.rs` | `expected_lang_common_path` |
| 7 | `index_build/common.rs` `should_skip_md` | 从仅 `-zh` 扩到主 stem 的 `-{lang}.md` 变体，避免译稿被扫成独立 entry |

一般不改：`local_http` 路由、`writeApiInvokeMap`、`frontend/js/api.js` 的 payload 透传。

---

## 旧字段（硬切）

- MCP：只暴露新字段  
- RS：收到 `extra_documents` 或 `index_extra` → 400  
- theme-line SKILL 同步迁移，无过渡期

---

## 验收要点

1. 仅 `document` 仍可归档（无 translations）
2. 单条 `zh`：写出 `-zh.md`，`index.translations.zh` 正确
3. 多条 `zh`+`fr`：两文件 + index map 两键
4. 旧 payload → 400
5. 非法 / 重复 `lang` → 400
6. MCP verify 与 RS 单测通过
