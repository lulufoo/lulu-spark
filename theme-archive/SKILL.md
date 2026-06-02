---
name: theme-archive
description: >-
  将已格式化的文档写入 raw/、更新 index.json（不触发 digest）。
  Use when: 归档文档、theme-archive、保存到 raw、写入 index.json、
  workbench archive、文档落盘（producer 链式 Embedded 或独立归档）。
argument-hint: '[文档路径 | Markdown 正文] [--source-type summary|article|theme-line]'
---

# theme-archive — Workbench 文档归档

> **路径与配置**：[archive-concepts.md](../shared/archive-concepts.md)（`COMMON_PATH`、`prefix`、`layers`、读 `config.json`）
>
> **输入**：已组好的 Markdown 文档（含 header + 正文）；可选附加文件（如 `-zh.md`）
>
> **输出**：`raw/<COMMON_PATH>`；更新 `index.json`（`layers: ["raw"]`）
>
> **边界**：本 skill **仅**负责 raw 落盘与 index 更新。**不**触发、**不**调用 [theme-digest](../theme-digest/SKILL.md)——digest 由 producer（如 theme-fetch）在归档完成后自行 Embedded 调用。

## 触发后的首要动作

1. 读取 `{skill_dir}/../config.json`，获取 `archive_root`，确认存在：

   `> ✅ config.json 读取完成 · archive_root: <路径>`

2. 判断 **Embedded** 或 **Standalone**（见下），执行 **Archive Workflow** `[AR-0]`–`[AR-6]`。

---

## Embedded 模式

由 `theme-fetch`、`theme-summary`、`theme-line` 等在文档组好后链式调用。

- 上游已确定 `COMMON_PATH`、文档正文、index 元数据（`source_type` 等）。
- 从 **Archive Workflow** 的 `[AR-1]` 起执行（跳过路径解析）。
- 完成后将归档结果追加到上游输出。

上游调用示例：

```text
加载并完整执行 ../theme-archive/SKILL.md（Embedded：
  COMMON_PATH = <topic-path>/<ts>-<slug>.md
  documents = [{ rel: "raw/<COMMON_PATH>", content: "<完整 Markdown>" }, ...]
  index_entry = { common_path, created_at, source_type, layers: ["raw"], ... }
）
```

Embedded 载荷字段见 [references/input-schema.md](references/input-schema.md)。

---

## Standalone 模式

用户直接提供文档内容或文件路径，需本 skill 解析 `topic-path` / `slug` / `ts`。

**触发词**：`theme-archive`、`归档文档`、`保存到 raw`、`workbench archive`

### 解析输入（优先级从高到低）

1. **Markdown 文件路径**（绝对或相对）→ 读全文作为 primary document
2. **用户粘贴**的完整 Markdown
3. **Workbench 上下文**：当前编辑的唯一 `.md` 文件

Standalone 从 `[AR-0]` 起执行路径解析，见 [references/standalone-resolve.md](references/standalone-resolve.md)。

---

## Archive Workflow

### [AR-0] 路径与文件名（Standalone only）

Embedded → 跳过，使用上游传入的 `COMMON_PATH`。

Standalone：

1. 读 `{archive_root}/topics.json` 选 `project`（`dir` 或 repo 短名；无匹配 → `inbox`）
2. 从文档 `# 标题` 或首行推断 `doc-theme`（kebab-case 英文，3–5 词）
3. `slug` = 标题 kebab-case 英文摘要
4. `ts` = `YYYYMMDDHHMM`（UTC+8）
5. `COMMON_PATH` = `<topic-path>/<ts>-<slug>.md`，`topic-path` = `<project>/<doc-theme>`

slug 冲突 → 与用户确认后再继续。

---

### [AR-1] 校验文档

Primary document 必须满足：

```text
· 以 `# 标题` 开头
· 含 `> 创建时间：` 元数据行
· 含 `> 导航：` 行，且 distilled / digest / trace 链接已 fully resolved（无占位符）
· 正文非空
```

附加文件（可选）：`raw/<topic-path>/<ts>-<slug>-zh.md` 等同目录命名规则。

Standalone 若 header 缺导航行，按 [archive-concepts.md](../shared/archive-concepts.md) 补全后再写入。

---

### [AR-2] 生成 entry ID

32 字符小写 hex：`secrets.token_hex(16)`（Python）或等效。

Embedded 若上游已提供 `id` → 复用。

---

### [AR-3] 准备 index.json 条目

最小字段：

```json
"<id>": {
  "common_path": "<topic-path>/<ts>-<slug>.md",
  "created_at": "<ts>",
  "layers": ["raw"],
  "source_type": "<summary|article|theme-line|...>"
}
```

可选扩展（按上游传入合并，不臆造）：

| 字段 | 用途 |
|------|------|
| `fetch` | theme-fetch：`platform`、`adapter`、`url` |
| `translations` | 英文源：`{ "zh": "<topic-path>/<ts>-<slug>-zh.md" }` |

`source_type` 默认：Standalone 未指定时 → `summary`；Embedded 必须显式传入。

---

### [AR-4] 落盘顺序

**先写文件，后写 index**（避免 index 指向不存在的路径）：

1. `{archive_root}/raw/<topic-path>/<ts>-<slug>.md`（primary）
2. 附加 raw 文件（如 `-zh.md`）
3. `{archive_root}/index.json`（`entries` 追加新条目）

目录不存在则创建。

---

### [AR-5] 完成输出

```text
> ✅ ThemeArchive 归档完成
> 📄 raw：raw/<topic-path>/<ts>-<slug>.md
> 📄 zh： raw/.../-zh.md          （有翻译文件时）
> 🗂 index.json 已更新（source_type: <type> · id: <id>）
```

---

### [AR-6] 失败与回滚

任一步失败 → **停止**；若 index 已写入但 raw 缺失，修正 index 或补写 raw。**不要**留下 `common_path` 指向不存在文件的 index 条目。

---

## Producer 集成

| Producer | 文档组稿 | 归档 (theme-archive) | digest (theme-digest) |
|----------|----------|----------------------|------------------------|
| theme-fetch | Phase 2 Format | Phase 3 Step 4 Embedded | Phase 3 Step 5 Embedded |
| theme-summary | § Core Output Shape | Step 3 Embedded | Step 4 Embedded |
| theme-line | Phase 2 Compose | Step 5 Embedded | Step 6 Embedded |

各 producer **编排** theme-archive 与 theme-digest 两次链式调用；本 skill **不**代劳 digest。

---

## Ask Only When Necessary

默认：最近 `topics.json` 匹配 · 标题推断 slug。

仅当 slug 冲突、project 无法推断、或文档 header 缺关键字段时询问用户。

---

## References

| Doc | Purpose |
|-----|---------|
| [input-schema.md](references/input-schema.md) | Embedded 载荷字段 |
| [standalone-resolve.md](references/standalone-resolve.md) | Standalone 路径解析 |
| [../shared/archive-concepts.md](../shared/archive-concepts.md) | 共享路径约定 |
