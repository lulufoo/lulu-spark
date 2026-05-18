# Archive Digest — 主题摘要

> **配置参考**：[archive-concepts.md](archive-concepts.md)
>
> **输入**：`RAW`（`raw/<COMMON_PATH>`，必须存在）；`<topic-path>`、`<ts>`、`<slug>` 由 raw 路径解析。
>
> **输出**：`digest/<COMMON_PATH>`；更新 `index.json` 的 `layers`。
>
> **禁止**：生成概述正文时读取 `distilled/`、`diagnose/`、`trace/` 下任何文件（`index.json` 仅用于导航行追加层链接，不读其正文）。

---

## [AD-0] 适用条件

digest 是可选产出。以下任意一项满足即生成：

```text
· raw 含 2+ 个 Turn 块（`## Turn N` 或归一化/TURN_SEP 等价的 Turn 分隔）
· raw 含 2+ 个主题级小节（`## ` 标题且非 `## Turn`，如 ThemeLine 主题段）
· raw 正文（去掉标题行、引用块导航后）字符数 ≥ 600
· index 条目 `entry_kind` 为 `summary` 且 raw 正文 ≥ 200
```

均不满足 → 跳过，输出：`> ⏭ digest 跳过（raw 过短或无概述价值）`，不向 `layers` 追加 `digest`。

---

## [AD-1] 结构

```markdown
# [主题标题] — 摘要

> 创建时间：[与 raw 相同的 ts]

> 导航：[raw](`<prefix>`raw/`<COMMON_PATH>`)[可选追加链接，见下]

## 概述

[一段话：本对话/总结/稿围绕什么主题、讨论或收敛到什么落点；仅依据 raw，不展开章节、不列概念表。]
```

**导航行（条件追加）**：读取 `index.json` 对应条目 `layers`；含 `distilled` / `trace` 时追加链接。前缀 `<prefix>` 见 [archive-concepts.md](archive-concepts.md)。

---

## [AD-2] 写法规则

```text
· 全部内容来源：仅 RAW
· 概述：单段话，建议 80–300 字
· 禁止 DDM / archive 内部术语（[U/U]、Phase、layer 等）
```

---

## [AD-3] 落盘

可选写入 `{archive_root}/.cache/<topic-path>/<ts>-<slug>-digest.md`，然后写入：

```
{archive_root}/digest/<topic-path>/<ts>-<slug>.md
```

**theme-line**：仅对主 raw（`<ts>-<slug>.md`）生成 digest，不对 `-zh.md` 翻译件单独生成。

---

## [AD-4] 更新 index.json

将 `"digest"` 追加到对应条目 `layers`（去重）。完成后输出：

```
> ✅ digest 完成 · digest/<topic-path>/<ts>-<slug>.md
> 🗂 index.json 已更新（layers 已追加 digest）
```

---

## 补跑（无独立命令）

当 `raw/` 已存在但 `digest/` 缺失或需重写时：

1. 读取 `{skill_dir}/../config.json` 得 `archive_root`
2. 确认 `raw/<COMMON_PATH>` 存在
3. 按 [AD-0]–[AD-4] 执行（不满足 [AD-0] 则跳过）

不依赖单独 skill 参数；agent 直接读本文件即可。
