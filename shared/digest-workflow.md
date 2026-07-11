# Digest workflow（shared · 非公开 skill）

> **可见性：** 共享契约，**不是**可触发的公开 skill。由 `theme-archive` 在 `[AR-3]` 自动执行；其它 producer（`theme-fetch` / `theme-line` / `dialogue-*`）在 `archive_document` 成功后 **Embedded** 按本文件 `[AD-0]`–`[AD-3]` 写 digest 并调 MCP。
>
> **用户入口：** 补跑/重写 digest → 走 [`theme-archive`](../theme-archive/SKILL.md)（Digest-only Repair）。不要寻找已删除的顶层 `theme-digest`。
>
> **路径约定：** [archive-concepts.md](archive-concepts.md)（`COMMON_PATH`、`prefix`、`layers`）
>
> **输入：** raw 正文 + entry `id`（来自 `archive_document`；补跑时由 theme-archive 解析后传入）
>
> **输出：** `digest/<COMMON_PATH>`；更新 `index.json` 的 `layers`
>
> **禁止：** 生成概述时读取 `distilled/`、`diagnose/`、`trace/` 下任何文件

## 谁加载本文件

| 调用方 | 模式 |
|--------|------|
| `theme-archive` | `archive_document` 成功后 **自动**；或 Digest-only Repair |
| `theme-fetch` / `theme-line` / `dialogue-*` 等 | Embedded：自管 `archive_document` 后按本文件执行 |

---

## Embedded 模式（默认）

上游已确定：`COMMON_PATH`、raw 正文、entry `id`、`source_type`（可选）。

1. 确认 Workbench MCP 可用。
2. 执行 **Archive Digest Workflow** `[AD-0]`–`[AD-3]`。
3. 将 MCP 返回追加到上游完成输出。

---

## Repair 模式（仅经 theme-archive）

raw 已存在、需补跑或 `--force` 重写：由 **theme-archive** 解析 `id` / raw 后进入本 workflow，并按需传 `force: true`。

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

**导航行（条件追加）**：若已知 index 条目 `layers` 含 `distilled` / `trace` 时追加链接。前缀 `<prefix>` 见 [archive-concepts.md](archive-concepts.md)。

---

### [AD-2] 写法规则

```text
· 全部内容来源：仅 RAW
· 概述：单段话，建议 80–300 字
· 禁止 DDM / archive 内部术语（[U/U]、Phase、layer 等）
```

**theme-line：** 仅对主 raw（`<ts>-<slug>.md`）生成 digest，不对 `-zh.md` 翻译件单独生成。

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
