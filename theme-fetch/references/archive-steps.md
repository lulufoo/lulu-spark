# Save to Archive — theme-archive + theme-digest

> **路径与配置：** [archive-concepts.md](../../shared/archive-concepts.md)
>
> **职责：** theme-fetch Phase 3 **编排**两次链式调用——先 [theme-archive](../../theme-archive/SKILL.md)（raw/index），再 [theme-digest](../../theme-digest/SKILL.md)（digest）。digest **不由** theme-archive 触发。

After Phase 2 Format produces the Markdown body:

### Step 1 · Select project and doc-theme

Read `{archive_root}/topics.json` → `project` + `doc-theme`；无匹配 → `inbox`。从 `bundle.meta.title` 推断 `slug`、`ts`、`COMMON_PATH`。

### Step 2 · Detect source language

Read `bundle.meta.language`：`en` → source + `-zh.md`；`zh` / `mixed` / `unknown` → source only。

### Step 3 · Build documents

Primary： [output-templates.md](output-templates.md) header + Phase 2 body。

英文源 → 翻译 body → `-zh.md`（同 header 模板）。

### Step 4 · theme-archive Embedded

构造载荷（见 [../../theme-archive/references/input-schema.md](../../theme-archive/references/input-schema.md)）：

```text
加载并完整执行 ../../theme-archive/SKILL.md（Embedded，从 [AR-1] 起：
  COMMON_PATH = <topic-path>/<ts>-<slug>.md
  documents = [
    { rel: "raw/<COMMON_PATH>", content: "<primary 全文>" },
    { rel: "raw/.../-zh.md", content: "..." }   # en 时
  ]
  index_entry = {
    common_path, created_at: <ts>, source_type: "article", layers: ["raw"],
    fetch: { platform, adapter, url },
    translations: { zh: "..." }   # en 时
  }
）
```

将 theme-archive `[AR-5]` 输出追加为 Phase 3 中间结果。

### Step 5 · theme-digest Embedded（theme-fetch 触发）

Primary raw 作为 **RAW**（不对 `-zh.md` digest）：

```text
加载并完整执行 ../../theme-digest/SKILL.md（Embedded：RAW = raw/<COMMON_PATH>，从 [AD-0] 起）
```

`source_type = article` 时按 `[AD-0]` 常规阈值。将 digest 结果追加为 Phase 3 完成输出：

```text
> 📋 digest：digest/<topic-path>/<ts>-<slug>.md  （或 "skipped"）
```
