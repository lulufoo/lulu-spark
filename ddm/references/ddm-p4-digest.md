# DDM Phase 4：摘要

> **配置参考**：[ddm-concepts.md](ddm-concepts.md)（路径符号定义）
>
> **输入**：`RAW`（`raw/<COMMON_PATH>`，由调用方传入，必须存在）；`<topic-path>`、`<ts>`、`<slug>` 由调用方从 raw 路径传入。
>
> **输出**：`CACHE_DIGEST`，写入本地 archive 的 `digest/<COMMON_PATH>`；同时更新 `index.json`。
>
> **禁止**：生成概述正文时读取 `distilled/`、`diagnose/`、`trace/` 下任何文件（`index.json` 仅用于导航行追加层链接，不读其正文）。

---

## PART 2 — 执行规范

### [P4-0] 适用条件

digest 是可选产出。以下任意一项满足即生成：

```text
· raw 含 2+ 个 Turn 块（`## Turn N` 或归一化/TURN_SEP 等价的 Turn 分隔）
· raw 正文（去掉标题行、引用块导航后）字符数 ≥ 600
· index 条目 `entry_kind` 为 `summary` 且 raw 正文 ≥ 200
```

三项均不满足 → 跳过 Phase 4，输出：`> ⏭ Phase 4 跳过（raw 过短或无概述价值）`，不向 `layers` 追加 `digest`。

---

### [P4-1] 结构

```markdown
# [主题标题] — 摘要

> 创建时间：[与 raw 相同的 ts]

> 导航：[raw](`<prefix>`raw/`<COMMON_PATH>`)[可选追加链接，见下]

## 概述

[一段话：本对话/总结围绕什么主题、讨论或收敛到什么落点；仅依据 raw，不展开章节、不列概念表。]
```

**导航行（条件追加，非正文输入）**：

读取 `{archive_root}/index.json` 对应条目的 `layers`：

- 含 `distilled` → 追加 ` · [distilled](\`<prefix>\`distilled/\`<COMMON_PATH>\`)`
- 含 `trace` → 追加 ` · [trace](\`<prefix>\`trace/\`<COMMON_PATH>\`)`

前缀 `<prefix>` 计算与 Phase 2/3 相同。默认至少含 `raw` 链接。

---

### [P4-2] 写法规则

```text
· 全部内容来源：仅 RAW，不引入 raw 外信息
· 概述：单段话，建议 80–300 字
· 禁止输出 DDM 内部术语（[U/U]、Phase、layer 等）
```

---

### [P4-3] 落盘

将生成内容写入 `CACHE_DIGEST`（`{archive_root}/.cache/<topic-path>/<ts>-<slug>-digest.md`），然后写入本地 archive：

```
{archive_root}/digest/<topic-path>/<ts>-<slug>.md
```

---

### [P4-4] 更新 index.json（最终步骤）

读取 `{archive_root}/index.json`，更新对应条目（按 `COMMON_PATH` 匹配）：

将 `"digest"` 追加到 `"layers"` 数组（去重；顺序保持 `raw → digest → distilled → diagnose → trace`）。

覆盖写入 `{archive_root}/index.json`。

完成后输出：

```
> ✅ Phase 4 完成 · digest：digest/<topic-path>/<ts>-<slug>.md
> 🗂 index.json 已更新（layers 已追加 digest）
```
