---
name: theme-digest
description: >-
  从 raw/ 生成或补跑 digest/ 主题摘要，并更新 index.json layers。
  Use when: digest 补跑、生成 digest、theme-digest、dtd_digest、
  raw 已有缺 digest、重写 digest、归档摘要。
argument-hint: '[COMMON_PATH | index id | raw 路径] [--force]'
---

# theme-digest — 主题摘要

> **路径约定**：[archive-concepts.md](../shared/archive-concepts.md)（`COMMON_PATH`、`prefix`、`layers`）
>
> **输入**：raw 正文（Embedded 由上游提供；Standalone 由用户粘贴 / Workbench 上下文 / 已知 entry `id`）
>
> **输出**：`digest/<COMMON_PATH>`；更新 `index.json` 的 `layers`
>
> **禁止**：生成概述正文时读取 `distilled/`、`diagnose/`、`trace/` 下任何文件

## 触发后的首要动作

1. 确认 Workbench MCP 可用（见 [archive-concepts.md](../shared/archive-concepts.md) MCP Prerequisite）。
2. 判断 **Embedded** 或 **Standalone**（见下），定位 RAW 正文，执行 **Archive Digest Workflow** `[AD-0]`–`[AD-3]`。

---

## Embedded 模式

由 producer 在 `archive_document` 成功后链式调用。`theme-summary`、`theme-line`、`dialogue-summary`、`theme-fetch` 经 MCP 自包含落盘时内嵌本 workflow。**不**由 theme-archive 单独触发。

- 上游已确定 `COMMON_PATH`、raw 正文、entry `id`（来自 `archive_document` 返回）。
- 从 **Archive Digest Workflow** 的 `[AD-0]` 起执行。
- 将 MCP 返回追加到上游完成输出。

---

## Standalone 模式

当 `raw/` 已存在但 `digest/` 缺失或需重写时，用户可直接触发本 skill。

**触发词**：`theme-digest`、`dtd_digest`、`补跑 digest`、`生成 digest`、`补充 digest`

### 解析 entry（优先级从高到低）

1. **index id**：32 位小写 hex（用户直接提供）
2. **COMMON_PATH**：`<topic-path>/<ts>-<slug>.md`（无 `raw/` 前缀）→ 请用户提供对应 `id`，或通过 Workbench 界面确认
3. **Workbench 上下文**：用户当前唯一 raw 文件 → 请用户确认 entry `id`
4. **用户粘贴** raw 全文 + 提供或推断 `COMMON_PATH` → 若无 `id`，须先 `archive_document` 或向用户索取 `id`

### 获取 RAW 正文

- 用户粘贴 raw 全文，或
- Workbench 中打开的唯一 raw 文件，或
- 已知 `id` 且用户确认 raw 内容（Agent 不直读 corpus 文件系统）

确认后输出：

```text
> 📄 RAW：raw/<COMMON_PATH>
```

### `--force`

- digest **已存在**且用户**未**声明 force → 提示并结束，不覆盖。
- 用户声明 `--force` / 「重写 digest」→ MCP `archive_digest` 传 `"force": true`。

---

## Archive Digest Workflow

<HARD-GATE mcp="archive">
落盘 MUST 经 MCP `archive_digest`。**禁止**直写 digest 或 index。
</HARD-GATE>

### [AD-0] 适用条件

digest 是可选产出。以下任意一项满足即生成：

```text
· raw 含 2+ 个 Turn 块（`## Turn N` 或归一化/TURN_SEP 等价的 Turn 分隔）
· raw 含 2+ 个主题级小节（`## ` 标题且非 `## Turn`，如 ThemeLine 主题段）
· raw 正文（去掉标题行、引用块导航后）字符数 ≥ 600
· source_type 为 summary 且 raw 正文 ≥ 200
· source_type 为 article 且 raw 正文 ≥ 600（常规阈值）
```

均不满足 → 跳过，输出：`> ⏭ digest 跳过（raw 过短或无概述价值）`。

---

### [AD-1] 结构

```markdown
# [主题标题] — 摘要

> 创建时间：[与 raw 相同的 ts]

> 导航：[raw](`<prefix>`raw/`<COMMON_PATH>`)[可选追加链接，见下]

## 概述

[一段话：本对话/总结/稿围绕什么主题、讨论或收敛到什么落点；仅依据 raw，不展开章节、不列概念表。]
```

**导航行（条件追加）**：若已知 index 条目 `layers` 含 `distilled` / `trace` 时追加链接。前缀 `<prefix>` 见 [archive-concepts.md](../shared/archive-concepts.md)。

---

### [AD-2] 写法规则

```text
· 全部内容来源：仅 RAW
· 概述：单段话，建议 80–300 字
· 禁止 DDM / archive 内部术语（[U/U]、Phase、layer 等）
```

**theme-line**：仅对主 raw（`<ts>-<slug>.md`）生成 digest，不对 `-zh.md` 翻译件单独生成。

---

### [AD-3] archive_digest (MCP)

调用 MCP `archive_digest`：

```json
{
  "id": "<entry id>",
  "digest": "<[AD-1]–[AD-2] 完整 digest Markdown>",
  "force": false
}
```

完成后输出：

```text
> ✅ digest 完成 · digest/<topic-path>/<ts>-<slug>.md
> 🗂 layers 已追加 digest（MCP）
```
