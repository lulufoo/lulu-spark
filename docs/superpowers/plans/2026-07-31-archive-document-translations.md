# archive_document translations 简化 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 `archive_document` 调用面改为 `translations[{lang,content}]`，由 RS 派生路径与 `index.translations`；旧字段硬切。

**Architecture:** SKILL / MCP / HTTP 同形透传；`archive_write` 解析 `translations[]`，用 `expected_lang_common_path` 写 `raw/…-{lang}.md` 并填充 index map；`extra_documents` / `index_extra` 一律 400。

**Tech Stack:** Rust (`lulu_workbench_lib`)、Node MCP (`packages/knowledge-mcp`)、theme-line skill（独立仓 `lulu-workbench-skills`）

**Spec:** [`docs/superpowers/specs/2026-07-31-archive-document-translations-design.md`](../specs/2026-07-31-archive-document-translations-design.md)

## Global Constraints

- 请求字段：`document`（必填）、`source_type`（可选）、`translations`（可选 list）
- `lang`：`^[a-z]{2}$`；同请求不重复；`content` trim 非空
- 路径：`{primary_stem}-{lang}.md`；index 落盘仍为 `translations: { lang: common_path }`
- 硬切：payload 含 `extra_documents` 或 `index_extra` → 400
- MCP 只透传，不衍生路径
- 不改 `archive_digest`；不做前端多语言 UI

---

## File map

| File | Role |
|------|------|
| `src-tauri/src/services/archive_parse.rs` | `expected_lang_common_path`；`expected_zh_common_path` 薄封装 |
| `src-tauri/src/unit-tests/services/archive_parse.rs` | 路径派生单测 |
| `src-tauri/src/services/archive_write.rs` | 拒旧字段；解析/写入 translations |
| `src-tauri/src/unit-tests/services/archive_write.rs` | 集成行为单测 |
| `src-tauri/src/services/index_build/common.rs` | `should_skip_md` 扩到 `-{lang}` |
| `src-tauri/src/unit-tests/services/index_build/common.rs` | skip 规则单测 |
| `packages/knowledge-mcp/index.mjs` | MCP schema + 透传 |
| `packages/knowledge-mcp/scripts/verify.mjs` | 若断言旧字段则更新（当前仅无 translations 的 happy path） |
| `~/.cursor/skills/lulu-workbench-skills/theme-line/references/archive-steps.md` | Step 5 新 payload（独立仓） |
| `docs/superpowers/specs/2026-07-31-archive-document-translations-design.md` | 状态改为已批准 |

---

### Task 1: `expected_lang_common_path`

**Files:**
- Modify: `src-tauri/src/services/archive_parse.rs:98-104`
- Modify: `src-tauri/src/unit-tests/services/archive_parse.rs:54-60`

**Interfaces:**
- Produces: `pub fn expected_lang_common_path(primary_common_path: &str, lang: &str) -> Option<String>`
- Produces: `pub fn expected_zh_common_path(primary: &str) -> Option<String>` — 调用 `expected_lang_common_path(primary, "zh")`

- [ ] **Step 1: Write the failing test**

在 `src-tauri/src/unit-tests/services/archive_parse.rs` 追加（并 `use` 新函数）：

```rust
#[test]
fn expected_lang_common_path_appends_suffix() {
    assert_eq!(
        expected_lang_common_path("inbox/t/202606191430-foo.md", "zh").as_deref(),
        Some("inbox/t/202606191430-foo-zh.md")
    );
    assert_eq!(
        expected_lang_common_path("inbox/t/202606191430-foo.md", "fr").as_deref(),
        Some("inbox/t/202606191430-foo-fr.md")
    );
}

#[test]
fn expected_lang_common_path_rejects_bad_primary() {
    assert_eq!(expected_lang_common_path("inbox/t/foo", "zh"), None);
}
```

保留现有 `expected_zh_common_path_appends_suffix` 不变。

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cd src-tauri && cargo test --lib expected_lang_common_path -- --nocapture
```

Expected: FAIL（函数未定义）

- [ ] **Step 3: Write minimal implementation**

在 `archive_parse.rs` 将 `expected_zh_common_path` 改为：

```rust
pub fn expected_lang_common_path(primary_common_path: &str, lang: &str) -> Option<String> {
    if !primary_common_path.ends_with(".md") {
        return None;
    }
    let stem = &primary_common_path[..primary_common_path.len() - 3];
    Some(format!("{stem}-{lang}.md"))
}

pub fn expected_zh_common_path(primary_common_path: &str) -> Option<String> {
    expected_lang_common_path(primary_common_path, "zh")
}
```

本任务不在此函数内校验 `lang` 格式（校验放在 `archive_write`）。

- [ ] **Step 4: Run test to verify it passes**

Run:

```bash
cd src-tauri && cargo test --lib expected_lang_common_path expected_zh_common_path -- --nocapture
```

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/services/archive_parse.rs src-tauri/src/unit-tests/services/archive_parse.rs
git commit -m "$(cat <<'EOF'
feat(archive): add expected_lang_common_path for translation stems

EOF
)"
```

---

### Task 2: RS `archive_write` — translations 消化与硬切

**Files:**
- Modify: `src-tauri/src/services/archive_write.rs`（`parse_extra_documents` / `parse_translations_zh` / `archive_document` 译稿分支）
- Modify: `src-tauri/src/unit-tests/services/archive_write.rs:146-192`

**Interfaces:**
- Consumes: `expected_lang_common_path(primary, lang)`
- Produces: payload 形状见 Global Constraints；成功时 `extra_paths: ["raw/…-{lang}.md", …]`；`index.entries[id].translations` 为 map

- [ ] **Step 1: Rewrite failing / replacement tests**

替换 `archive_document_theme_line_with_zh_extra` 与 `archive_document_rejects_mismatched_zh_path`，并新增用例：

```rust
#[test]
fn archive_document_theme_line_with_zh_translation() {
    let (_sandbox, repo_root) = setup_corpus();
    let zh_path = "learning-ai-agent/waymo-interview/202606191700-waymo-interview-zh.md";
    let v = archive_document(
        &repo_root,
        &json!({
            "document": THEME_LINE_DOC,
            "source_type": "theme-line",
            "translations": [{ "lang": "zh", "content": THEME_LINE_ZH }]
        }),
    );
    assert_eq!(v.get("ok"), Some(&json!(true)), "failed: {v}");
    let corpus = crate::config::meili_env::workbench_knowledge_root_path(&repo_root);
    assert!(corpus.join(format!("raw/{zh_path}")).is_file());
    assert_eq!(
        v["extra_paths"],
        json!([format!("raw/{zh_path}")])
    );
    let id = v["id"].as_str().unwrap();
    let index: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(corpus.join("index.json")).unwrap()).unwrap();
    assert_eq!(index["entries"][id]["translations"]["zh"], json!(zh_path));
}

#[test]
fn archive_document_multi_lang_translations() {
    let (_sandbox, repo_root) = setup_corpus();
    let v = archive_document(
        &repo_root,
        &json!({
            "document": THEME_LINE_DOC,
            "source_type": "theme-line",
            "translations": [
                { "lang": "zh", "content": THEME_LINE_ZH },
                { "lang": "fr", "content": "# Titre\n\n> 创建时间：2026年6月19日 17:00\n\n---\n\nbonjour\n" }
            ]
        }),
    );
    assert_eq!(v.get("ok"), Some(&json!(true)), "failed: {v}");
    let id = v["id"].as_str().unwrap();
    let corpus = crate::config::meili_env::workbench_knowledge_root_path(&repo_root);
    let index: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(corpus.join("index.json")).unwrap()).unwrap();
    assert_eq!(
        index["entries"][id]["translations"]["zh"],
        json!("learning-ai-agent/waymo-interview/202606191700-waymo-interview-zh.md")
    );
    assert_eq!(
        index["entries"][id]["translations"]["fr"],
        json!("learning-ai-agent/waymo-interview/202606191700-waymo-interview-fr.md")
    );
}

#[test]
fn archive_document_rejects_legacy_extra_documents() {
    let (_sandbox, repo_root) = setup_corpus();
    let v = archive_document(
        &repo_root,
        &json!({
            "document": THEME_LINE_DOC,
            "source_type": "theme-line",
            "extra_documents": [{
                "rel": "raw/learning-ai-agent/waymo-interview/202606191700-waymo-interview-zh.md",
                "content": THEME_LINE_ZH
            }]
        }),
    );
    assert_eq!(v.get("_status"), Some(&json!(400)));
    let err = v["error"].as_str().unwrap_or("");
    assert!(err.contains("translations"), "error should mention translations: {v}");
}

#[test]
fn archive_document_rejects_legacy_index_extra() {
    let (_sandbox, repo_root) = setup_corpus();
    let v = archive_document(
        &repo_root,
        &json!({
            "document": THEME_LINE_DOC,
            "index_extra": { "translations": { "zh": "x" } }
        }),
    );
    assert_eq!(v.get("_status"), Some(&json!(400)));
}

#[test]
fn archive_document_rejects_bad_or_duplicate_lang() {
    let (_sandbox, repo_root) = setup_corpus();
    let bad = archive_document(
        &repo_root,
        &json!({
            "document": THEME_LINE_DOC,
            "translations": [{ "lang": "ZH", "content": THEME_LINE_ZH }]
        }),
    );
    assert_eq!(bad.get("_status"), Some(&json!(400)));

    let dup = archive_document(
        &repo_root,
        &json!({
            "document": THEME_LINE_DOC,
            "translations": [
                { "lang": "zh", "content": THEME_LINE_ZH },
                { "lang": "zh", "content": THEME_LINE_ZH }
            ]
        }),
    );
    assert_eq!(dup.get("_status"), Some(&json!(400)));
}
```

删除旧的 `archive_document_rejects_mismatched_zh_path`（路径不再由调用方提供）。

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cd src-tauri && cargo test --lib archive_document_theme_line_with_zh_translation archive_document_multi_lang_translations archive_document_rejects_legacy_extra_documents archive_document_rejects_legacy_index_extra archive_document_rejects_bad_or_duplicate_lang -- --nocapture
```

Expected: FAIL（仍认旧字段 / 不认 `translations`）

- [ ] **Step 3: Implement RS digestion**

在 `archive_write.rs`：

1. `use` 改为 `expected_lang_common_path`（可保留 `expected_zh_common_path` 若别处仍用）。
2. 删除 `parse_extra_documents` / `parse_translations_zh`。
3. 新增：

```rust
struct TranslationDoc {
    lang: String,
    common_path: String,
    rel: String,
    content: String,
}

fn reject_legacy_translation_fields(payload: &Value) -> Option<Value> {
    if payload.get("extra_documents").is_some() || payload.get("index_extra").is_some() {
        return Some(json!({
            "error": "extra_documents/index_extra removed; use translations: [{lang, content}]",
            "_status": 400
        }));
    }
    None
}

fn is_valid_lang(lang: &str) -> bool {
    lang.len() == 2 && lang.chars().all(|c| c.is_ascii_lowercase())
}

fn parse_translations(
    payload: &Value,
    primary_common_path: &str,
) -> Result<Vec<TranslationDoc>, Value> {
    let Some(arr) = payload.get("translations").and_then(|v| v.as_array()) else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for item in arr {
        let lang = item
            .get("lang")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        let content = item
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if !is_valid_lang(&lang) {
            return Err(json!({ "error": format!("invalid translations.lang: {lang}"), "_status": 400 }));
        }
        if content.trim().is_empty() {
            return Err(json!({ "error": "translations.content must be non-empty", "_status": 400 }));
        }
        if !seen.insert(lang.clone()) {
            return Err(json!({ "error": format!("duplicate translations.lang: {lang}"), "_status": 400 }));
        }
        let common_path = expected_lang_common_path(primary_common_path, &lang).ok_or_else(|| {
            json!({ "error": "invalid primary common_path for translation", "_status": 400 })
        })?;
        let rel = format!("raw/{common_path}");
        out.push(TranslationDoc {
            lang,
            common_path,
            rel,
            content,
        });
    }
    Ok(out)
}
```

4. 在 `archive_document` 中，解析主文档后：

```rust
if let Some(err) = reject_legacy_translation_fields(payload) {
    return err;
}
let translations = match parse_translations(payload, &parsed.common_path) {
    Ok(v) => v,
    Err(v) => return v,
};
```

5. 用 `translations` 替代原 `extra_docs` 写文件 / conflict 检查 / `extra_paths`。
6. index entry：

```rust
if !translations.is_empty() {
    let mut map = serde_json::Map::new();
    for t in &translations {
        map.insert(t.lang.clone(), json!(t.common_path));
    }
    entry.insert("translations".to_string(), Value::Object(map));
}
```

保持现有 rollback / task_ref 逻辑不变。

- [ ] **Step 4: Run tests to verify they pass**

Run:

```bash
cd src-tauri && cargo test --lib archive_document_ -- --nocapture
```

Expected: PASS（含无 translations 的基线用例）

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/services/archive_write.rs src-tauri/src/unit-tests/services/archive_write.rs
git commit -m "$(cat <<'EOF'
feat(archive): digest translations list in archive_document; hard-cut legacy fields

EOF
)"
```

---

### Task 3: `should_skip_md` 覆盖 `-{lang}`

**Files:**
- Modify: `src-tauri/src/services/index_build/common.rs:28-35`
- Modify: `src-tauri/src/unit-tests/services/index_build/common.rs`

**Interfaces:**
- Produces: `should_skip_md(name)` 对 `*-zh.md` / `*-fr.md` 等返回 true；对普通 primary 文件名返回 false

- [ ] **Step 1: Write the failing test**

在 `unit-tests/services/index_build/common.rs` 追加：

```rust
#[test]
fn should_skip_md_translation_suffixes() {
    assert!(should_skip_md("202606191700-waymo-interview-zh.md"));
    assert!(should_skip_md("202606191700-waymo-interview-fr.md"));
    assert!(!should_skip_md("202606191700-waymo-interview.md"));
    assert!(!should_skip_md("README.md"));
}
```

（确认该测试模块已 `use` 到 `should_skip_md`；若未导出到测试作用域，按文件现有 `use super::*` 模式。）

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cd src-tauri && cargo test --lib should_skip_md_translation_suffixes -- --nocapture
```

Expected: FAIL on `-fr`（当前只跳 `-zh`）

- [ ] **Step 3: Minimal implementation**

替换 `should_skip_md` 内译稿分支：

```rust
pub fn should_skip_md(name: &str) -> bool {
    if SKIP_FILES.contains(&name) {
        return true;
    }
    // Translation variants: …-{lang}.md where lang is ISO 639-1 (two lowercase letters).
    // Indexed via entry.translations, not as standalone Meilisearch docs.
    name.strip_suffix(".md").is_some_and(|stem| {
        stem.rsplit_once('-')
            .is_some_and(|(_, lang)| lang.len() == 2 && lang.chars().all(|c| c.is_ascii_lowercase()))
    })
}
```

注意：slug 若以 `-xx`（两小写字母）结尾的主文件也会被 skip——与 design 接受的启发式一致；勿再加复杂例外。

- [ ] **Step 4: Run test to verify it passes**

Run:

```bash
cd src-tauri && cargo test --lib should_skip_md_translation_suffixes workbench_doc_id_matches_python_rules -- --nocapture
```

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/services/index_build/common.rs src-tauri/src/unit-tests/services/index_build/common.rs
git commit -m "$(cat <<'EOF'
fix(index-build): skip *-{lang}.md translation files in corpus scan

EOF
)"
```

---

### Task 4: MCP `archive_document` schema 透传

**Files:**
- Modify: `packages/knowledge-mcp/index.mjs:148-196`

**Interfaces:**
- Produces: tool input `{ document, source_type?, translations?: [{lang, content}] }` → POST body 同形

- [ ] **Step 1: Replace schema + handler**

将 `archive_document` 注册块改为：

```javascript
  server.registerTool(
    'archive_document',
    {
      description:
        'Archive a formatted document to raw/ and index.json. Returns entry id. Proxy POST /api/archive-document',
      inputSchema: {
        document: z.string().min(1).describe('Full Markdown document with header and body'),
        source_type: z
          .string()
          .optional()
          .describe('Index source_type; defaults to summary'),
        translations: z
          .array(
            z.object({
              lang: z
                .string()
                .regex(/^[a-z]{2}$/)
                .describe('ISO 639-1 language code'),
              content: z.string().min(1).describe('Translation markdown body'),
            }),
          )
          .optional()
          .describe('Optional translation bodies; paths derived by host'),
      },
    },
    async ({ document, source_type, translations }) => {
      const body = { document };
      if (source_type != null) {
        body.source_type = source_type;
      }
      if (translations != null) {
        body.translations = translations;
      }
      const result = await proxyPost('/api/archive-document', body);
      if (!result.ok) {
        return toolError(result.status, result.text);
      }
      return { content: [{ type: 'text', text: result.text }] };
    },
  );
```

- [ ] **Step 2: Run MCP verify**

Run:

```bash
cd packages/knowledge-mcp && npm run verify
```

Expected: PASS（mock 仍对任意 POST `/api/archive-document` 返回固定响应；无旧字段断言）

- [ ] **Step 3: Commit**

```bash
git add packages/knowledge-mcp/index.mjs
git commit -m "$(cat <<'EOF'
feat(mcp): archive_document accepts translations list and drops legacy fields

EOF
)"
```

---

### Task 5: theme-line SKILL Step 5（独立仓）

**Files:**
- Modify: `/Users/lulu/.cursor/skills/lulu-workbench-skills/theme-line/references/archive-steps.md`（仓根：`lulu-workbench-skills`）

**Interfaces:**
- Consumes: Task 4 MCP 契约

- [ ] **Step 1: Update Step 5 JSON example**

将 Step 5 示例与说明改为：

```markdown
### Step 5 · archive_document (MCP)

<HARD-GATE>
Workbench App **must be running** (`workbench-knowledge` MCP). On failure → stop; **do not** write `archive_root` directly.
</HARD-GATE>

```json
{
  "document": "<Step 3 full markdown>",
  "source_type": "theme-line",
  "translations": [
    { "lang": "zh", "content": "<Step 4 full markdown>" }
  ]
}
```

- Omit `translations` when language is not `en`.
- Do **not** send paths, `extra_documents`, or `index_extra` — host derives `-{lang}.md` and index map.
- Record returned `id`, `common_path`, `raw_path`, `extra_paths`.
```

（保持 Step 4「仅英文源生成 `-zh` 正文」不变；只改 MCP 调用形状。）

- [ ] **Step 2: Commit in skills repo**

```bash
cd ~/.cursor/skills/lulu-workbench-skills
git add theme-line/references/archive-steps.md
git commit -m "$(cat <<'EOF'
feat(theme-line): archive_document uses translations[{lang,content}]

EOF
)"
```

勿把 skills 仓改动混进 `lulu-workbench` 提交。

---

### Task 6: Spec 状态 + 全量回归

**Files:**
- Modify: `docs/superpowers/specs/2026-07-31-archive-document-translations-design.md`（状态 `待审` → `已批准`）

- [ ] **Step 1: Flip spec status**

将文首 `状态：待审` 改为 `状态：已批准`。

- [ ] **Step 2: Full regression**

Run:

```bash
cd src-tauri && cargo test --lib archive_document_ expected_lang_common_path expected_zh_common_path should_skip_md_translation_suffixes -- --nocapture
cd packages/knowledge-mcp && npm run verify
```

Expected: all PASS

- [ ] **Step 3: Commit (workbench)**

```bash
git add docs/superpowers/specs/2026-07-31-archive-document-translations-design.md
git commit -m "$(cat <<'EOF'
docs: mark archive_document translations design approved

EOF
)"
```

（若本计划文件尚未入库，一并 `git add docs/superpowers/plans/2026-07-31-archive-document-translations.md`。）

---

## Spec coverage checklist

| Spec 要求 | Task |
|-----------|------|
| `translations[{lang,content}]` 请求 | 2, 4, 5 |
| RS 派生路径 + index map | 1, 2 |
| 硬切旧字段 | 2, 4 |
| `lang` / 重复 / 空 content 校验 | 2 |
| 返回 `extra_paths` | 2 |
| `should_skip_md` 多 lang | 3 |
| theme-line Step 5 | 5 |
| MCP 透传 | 4 |
| 验收要点 1–6 | 2, 4, 6 |

## Self-review notes

- 无 TBD /「类似 Task N」占位
- `expected_zh_common_path` 保留为薄封装，避免其它调用点瞬时断裂
- skills 仓单独 commit，避免跨仓混提
