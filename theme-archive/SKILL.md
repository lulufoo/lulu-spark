---
name: theme-archive
description: >-
  将已格式化的文档写入 raw/、更新 index.json，并在适用时自动生成 digest。
  Use when: 归档文档、theme-archive、保存到 raw、写入 index.json、
  workbench archive、文档落盘、补跑 digest、重写 digest（经本 skill，勿单独调 theme-digest）。
argument-hint: '[文档路径 | Markdown 正文] [--source-type summary|article|theme-line] [--force-digest]'
---

# theme-archive — Workbench 文档归档

> **路径约定**：[archive-concepts.md](../shared/archive-concepts.md)（`COMMON_PATH`、`prefix`、`layers`）
>
> **输入**：已组好的 Markdown 文档（含 header + 正文）；可选附加文件（如 `-zh.md`）
>
> **输出**：`raw/<COMMON_PATH>`；更新 `index.json`；适用时 `digest/<COMMON_PATH>`
>
> **边界**：本 skill 负责 raw 落盘 + index，并在 `[AR-2]` 成功后 **自动**加载 shared digest 契约。digest 规则见 [digest-workflow.md](../shared/digest-workflow.md)（**非公开 skill**）。

## 触发后的首要动作

1. 确认 Workbench MCP 可用（见 [archive-concepts.md](../shared/archive-concepts.md) MCP Prerequisite）。
2. 若用户只要补跑/重写 digest（raw 已在库）：走 **Digest-only Repair**（见下），否则判断 **Embedded** / **Standalone**，执行 **Archive Workflow**。

---

## Embedded 模式

由 producer 在文档组好后传入完整 Markdown 与元数据。

- 上游已确定 `COMMON_PATH`、文档正文、`source_type` 等。
- 从 **Archive Workflow** 的 `[AR-1]` 起执行（跳过路径解析）。
- `[AR-2]` 成功后执行 `[AR-3]` 自动 digest（除非上游显式 `skip_digest: true`）。
- 完成后将 MCP 返回追加到上游输出。

---

## Standalone 模式

用户直接提供文档内容或文件路径，需本 skill 解析 `topic-path` / `slug` / `ts`。

**触发词**：`theme-archive`、`归档文档`、`保存到 raw`、`workbench archive`、`补跑 digest`、`重写 digest`

### 解析输入（优先级从高到低）

1. **Markdown 文件路径**（绝对或相对）→ 读全文作为 primary document
2. **用户粘贴**的完整 Markdown
3. **Workbench 上下文**：当前编辑的唯一 `.md` 文件

Standalone 从 `[AR-0]` 起执行路径解析，见 [references/standalone-resolve.md](references/standalone-resolve.md)。

---

## Digest-only Repair

当用户请求补跑/重写 digest、且 raw 已归档：

1. 解析 entry `id` 与 raw 正文（用户提供 id / COMMON_PATH / 粘贴 raw；Agent 不直读 corpus）。
2. 加载 [digest-workflow.md](../shared/digest-workflow.md)，以 Repair/Embedded 执行 `[AD-0]`–`[AD-3]`。
3. 用户声明 `--force-digest` / 「重写 digest」→ `force: true`；digest 已存在且未 force → 提示后结束。

**不要**引导用户去找已删除的公开 `theme-digest` 指令。

---

## Archive Workflow

<HARD-GATE mcp="archive">
Workbench App **必须运行**（MCP `workbench-knowledge` 可用）。**禁止**直写 corpus 文件系统。
</HARD-GATE>

### [AR-0] 路径与文件名（Standalone only）

Embedded → 跳过，使用上游传入的 `COMMON_PATH`。

Standalone：

1. 语义推断 `project`（最接近 topics；无匹配 → `inbox`）
2. 从文档 `# 标题` 或首行推断 `doc-theme`（kebab-case 英文，3–5 词）
3. `slug` = 标题 kebab-case 英文摘要
4. `ts` = `YYYYMMDDHHMM`（UTC+8）
5. `COMMON_PATH` = `<topic-path>/<ts>-<slug>.md`，`topic-path` = `<project>/<doc-theme>`

slug 冲突 → 与用户确认后再继续。

---

### [AR-1] 校验文档

Standalone：组档前对正文应用 [references/body-sanitize.md](references/body-sanitize.md)（剥离外链图片）。若输入为 GitHub blob/raw URL，按 [references/ts-inference.md](references/ts-inference.md) 推断 `ts`。


Primary document 必须满足：

```text
· 以 `# 标题` 开头
· 含 `> 创建时间：` 元数据行
· 含 `> 导航：` 行，且 digest 链接已 fully resolved（无占位符）
· 正文非空
```

附加文件（可选）：`raw/<topic-path>/<ts>-<slug>-zh.md` 等同目录命名规则。

Standalone 若 header 缺导航行，按 [archive-concepts.md](../shared/archive-concepts.md) 补全后再归档。

---

### [AR-2] archive_document (MCP)

调用 MCP `archive_document`：

```json
{
  "document": "<primary 全文 Markdown>",
  "source_type": "<summary|article|theme-line|dialogue|...>",
  "extra_documents": [
    {
      "rel": "raw/<topic-path>/<ts>-<slug>-zh.md",
      "content": "<附加 raw 全文>"
    }
  ],
  "index_extra": {
    "translations": { "zh": "<topic-path>/<ts>-<slug>-zh.md" }
  }
}
```

- 无附加文件时省略 `extra_documents` / `index_extra`。
- `source_type` 默认：Standalone 未指定 → `summary`；Embedded 必须显式传入。
- 记录返回的 `id`、`common_path`、`raw_path`、`extra_paths`。

---

### [AR-3] 自动 digest（shared 契约）

`[AR-2]` **成功后必须执行**（除非 Embedded 载荷 `skip_digest: true`）：

1. **Read** [digest-workflow.md](../shared/digest-workflow.md)（shared 契约，非公开 skill）。
2. 以 Embedded 传入：`id`、raw 正文（primary）、`COMMON_PATH`、`source_type`。
3. 按该文件 `[AD-0]`–`[AD-3]` 执行；适用则 MCP `archive_digest`，否则输出跳过行。

---

### [AR-4] 完成输出

```text
> ✅ ThemeArchive 归档完成
> 📄 raw：raw/<topic-path>/<ts>-<slug>.md
> 📄 zh： raw/.../-zh.md          （有翻译文件时）
> 📋 digest：digest/<topic-path>/<ts>-<slug>.md  （或 ⏭ 已跳过）
> 🗂 id: <id> · source_type: <type>
```

---

### [AR-5] 失败处理

MCP 返回错误 → **停止**；向用户报告 HTTP 状态与消息。**不要**尝试直写 corpus 作为回退。  
`[AR-2]` 已成功而 `[AR-3]` 失败 → 报告 raw 已落盘、digest 失败，可经 Digest-only Repair 重试；**不要**回滚 raw。

---

## Producer 集成

| Producer | 文档组稿 | 归档 | digest |
|----------|----------|------|--------|
| theme-fetch | Phase 2 Format | Phase 3 MCP `archive_document` | 按 [digest-workflow](../shared/digest-workflow.md) Embedded |
| theme-line | Phase 2 Compose | Step 5 MCP | 同上 Embedded |
| dialogue-summary / dialogue-archive | 各自 Output Shape | MCP `archive_document` | 同上 Embedded |
| **本 skill Standalone** | 用户已定稿文档 | `[AR-2]` | **`[AR-3]` 自动** |

Producer 若自管 MCP，仍须遵守内部 digest 的 `[AD-0]`–`[AD-3]`；**不要**再指向已删除的顶层 `theme-digest/` 公开 skill。

---

## Ask Only When Necessary

默认：语义推断 project · 标题推断 slug。

仅当 slug 冲突、project 无法推断、或文档 header 缺关键字段时询问用户。

---

## References

| Doc | Purpose |
|-----|---------|
| [digest-workflow.md](../shared/digest-workflow.md) | shared digest 契约（`[AD-0]`–`[AD-3]`） |
| [input-schema.md](references/input-schema.md) | Embedded 载荷字段（逻辑等价于 MCP 参数） |
| [standalone-resolve.md](references/standalone-resolve.md) | Standalone 路径解析 |
| [../shared/archive-concepts.md](../shared/archive-concepts.md) | 共享路径约定 |
