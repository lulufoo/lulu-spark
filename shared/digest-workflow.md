# Digest workflow（shared · 非公开 skill）

> **可见性：** 共享契约，**不是**可触发的公开 skill。由 `theme-archive` 在 `[AR-3]` 自动执行。Producer 经 theme-archive Embedded 录入，**不要**自管 `create_note_digest`。
>
> **用户入口：** 补跑/重写 digest → 走 [`theme-archive`](../theme-archive/SKILL.md)（Digest-only Repair）。不要寻找已删除的顶层 `theme-digest`。
>
> **路径约定：** [archive-concepts.md](archive-concepts.md)（`COMMON_PATH`、`prefix`、`layers`）
>
> **输入：** raw 正文 + entry `id`（来自 `create_note`；补跑时由 theme-archive 解析后传入）
>
> **输出：** `digest/<COMMON_PATH>`；更新 `index.json` 的 `layers`
>
> **禁止：** 生成概述时读取 raw 以外的任何 notes 层

## 谁加载本文件

| 调用方 | 模式 |
|--------|------|
| `theme-archive` | `create_note` 成功后 **自动**；或 Digest-only Repair |
| 各 producer | 经 theme-archive Embedded；digest 仍按本文件 `[AD-0]`–`[AD-3]` |

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
落盘 MUST 经 MCP `create_note_digest`。**禁止**直写 digest 或 index。
</HARD-GATE>

### [AD-0] 适用条件

digest 是可选产出。以下任意一项满足即生成：

```text
· raw 含 2+ 个 Turn 块（`## Turn N` 或归一化/TURN_SEP 等价的 Turn 分隔）
· raw 含 2+ 个主题级小节（`## ` 标题且非 `## Turn`，如 ThemeLine 时间/来源章节标）
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

**导航行**：仅链回 raw。前缀 `<prefix>` 见 [archive-concepts.md](archive-concepts.md)。

---

### [AD-1b] 内容约束（dialogue-* 必填；其它 producer 建议）

写 digest 前须有非空 **内容约束** `content_constraint`（写作合同，非 MCP 字段）。允许：Turn/节点范围叙述、一句话焦点、或二者组合。

Digest 头 **必须**含：

```markdown
> 内容约束：<content_constraint 原文>
```

**禁止：** 为写摘要再次拉取整份 raw 进模型；约束外扩写。  
`dialogue-archive`：约束可窄于脚本节点范围；两者都应在当轮说明里写清。

### [AD-2] 写法规则

```text
· 全部内容来源：仅 RAW（或 dialogue-*：对话上下文 + 内容约束，不重拉整份 raw）
· 概述：单段话，建议 80–300 字
· 禁止 DDM / archive 内部术语（[U/U]、Phase、layer 等）
· dialogue-*：遵守 [AD-1b] 内容约束
```

**theme-line：** 仅对主 raw（`<ts>-<slug>.md`）生成 digest，不对 `-zh.md` 翻译件单独生成。

---

### [AD-3] create_note_digest (MCP)

调用 MCP `create_note_digest`：

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
