---
name: theme-digest
description: >-
  从 raw/ 生成或补跑 digest/ 主题摘要，并更新 index.json layers。
  Use when: digest 补跑、生成 digest、theme-digest、dtd_digest、
  raw 已有缺 digest、重写 digest、归档摘要。
argument-hint: '[COMMON_PATH | index id | raw 路径] [--force]'
---

# theme-digest — 主题摘要

> **路径与配置**：[archive-concepts.md](../shared/archive-concepts.md)（`COMMON_PATH`、`prefix`、`layers`、读 `config.json`）
>
> **输入**：`RAW` = `{archive_root}/raw/<COMMON_PATH>`（必须存在）
>
> **输出**：`digest/<COMMON_PATH>`；更新 `index.json` 的 `layers`
>
> **禁止**：生成概述正文时读取 `distilled/`、`diagnose/`、`trace/` 下任何文件（`index.json` 仅用于导航行追加层链接，不读其正文）

## 触发后的首要动作

1. 读取 `{skill_dir}/../config.json`，获取 `archive_root`，确认存在：

   `> ✅ config.json 读取完成 · archive_root: <路径>`

2. 判断 **Embedded** 或 **Standalone**（见下），定位 `RAW`，执行 **Archive Digest Workflow** `[AD-0]`–`[AD-4]`。

---

## Embedded 模式

由 `theme-fetch` 在 `raw/` 落盘后链式调用。`theme-summary`、`theme-line`、`dialogue-summary` 经 MCP 自包含落盘（不链式加载本 skill）。**不**由 theme-archive 触发。

- **不**重新解析路径；上游已确定 `COMMON_PATH` 与 `RAW`。
- 从 **Archive Digest Workflow** 的 `[AD-0]` 起执行。
- 将 digest 结果追加到上游完成输出。

上游调用示例：

```text
加载并完整执行 ../theme-digest/SKILL.md（Embedded：RAW = raw/<COMMON_PATH>）
```

---

## Standalone 模式

当 `raw/` 已存在但 `digest/` 缺失或需重写时，用户可直接触发本 skill。

**触发词**：`theme-digest`、`dtd_digest`、`补跑 digest`、`生成 digest`、`补充 digest`

### 解析 RAW（优先级从高到低）

1. **COMMON_PATH**：`<topic-path>/<ts>-<slug>.md`（无 `raw/` 前缀）
2. **层路径**：`raw/<COMMON_PATH>` 或绝对路径含 `.../raw/<COMMON_PATH>`
3. **index id**：32 位小写 hex → 读 `index.json` → `common_path` → `RAW`
4. **Workbench 上下文**：用户当前唯一 raw 文件

确认 `{archive_root}/raw/<COMMON_PATH>` 存在后，输出：

```text
> 📄 RAW：raw/<COMMON_PATH>
```

### `--force`

- digest 文件**已存在**且用户**未**声明 force → 提示路径并结束，不覆盖。
- 用户声明 `--force` / 「重写 digest」→ 按 Workflow 覆盖写入。

---

## Archive Digest Workflow

### [AD-0] 适用条件

digest 是可选产出。以下任意一项满足即生成：

```text
· raw 含 2+ 个 Turn 块（`## Turn N` 或归一化/TURN_SEP 等价的 Turn 分隔）
· raw 含 2+ 个主题级小节（`## ` 标题且非 `## Turn`，如 ThemeLine 主题段）
· raw 正文（去掉标题行、引用块导航后）字符数 ≥ 600
· index 条目 source_type 为 summary 且 raw 正文 ≥ 200（无 source_type 时读 entry_kind）
```

均不满足 → 跳过，输出：`> ⏭ digest 跳过（raw 过短或无概述价值）`，不向 `layers` 追加 `digest`。

---

### [AD-1] 结构

```markdown
# [主题标题] — 摘要

> 创建时间：[与 raw 相同的 ts]

> 导航：[raw](`<prefix>`raw/`<COMMON_PATH>`)[可选追加链接，见下]

## 概述

[一段话：本对话/总结/稿围绕什么主题、讨论或收敛到什么落点；仅依据 raw，不展开章节、不列概念表。]
```

**导航行（条件追加）**：读取 `index.json` 对应条目 `layers`；含 `distilled` / `trace` 时追加链接。前缀 `<prefix>` 见 [archive-concepts.md](../shared/archive-concepts.md)。

---

### [AD-2] 写法规则

```text
· 全部内容来源：仅 RAW
· 概述：单段话，建议 80–300 字
· 禁止 DDM / archive 内部术语（[U/U]、Phase、layer 等）
```

---

### [AD-3] 落盘

可选写入 `{archive_root}/.cache/<topic-path>/<ts>-<slug>-digest.md`，然后写入：

```
{archive_root}/digest/<topic-path>/<ts>-<slug>.md
```

**theme-line**：仅对主 raw（`<ts>-<slug>.md`）生成 digest，不对 `-zh.md` 翻译件单独生成。

---

### [AD-4] 更新 index.json

将 `"digest"` 追加到对应条目 `layers`（去重）。完成后输出：

```text
> ✅ digest 完成 · digest/<topic-path>/<ts>-<slug>.md
> 🗂 index.json 已更新（layers 已追加 digest）
```
