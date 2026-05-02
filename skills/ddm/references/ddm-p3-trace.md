# DDM Phase 3：认知轨迹

> **输入**：`DIAGNOSE`（`diagnose/<COMMON_PATH>`）；`CACHE_DISTILLED`；`<topic-path>`、`<ts>`、`<slug>` 从 `CACHE_RAW` 文件路径解析。
>
> **输出**：`CACHE_TRACE`，写入本地 archive 的 `trace/<COMMON_PATH>`。

---

## PART 1 — 术语定义

> 本 Phase 从 DIAGNOSE 读取以下字段。完整双轴归属模型见 [ddm-p1-diagnose.md](ddm-p1-diagnose.md) PART 1。

**[U/U] 事件**：用户自发发起并完成的认知动作。

**[U/AI] 高自主性事件**：AI 设计方向、但用户独立产出了 AI 未给出的新认知的事件。

**入口假设**：对话开始时用户持有的预设（通常是走偏前的认知状态），来自 DIAGNOSE 中记录的第一个偏差节点触发语境。

**跃迁点**：整体认知模型突然对齐的那一轮——多个前序疑问被同一机制一并解释的节点。

---

## PART 2 — 执行规范

> 前置：阅读 PART 1（本 Phase 术语定义）。

## [P3-0] 适用条件

trace 是可选产出。以下任意一项满足即生成：

```text
· 整体认知模型发生了跃迁（DIAGNOSE 的「跃迁点」字段非空）
· 学习目标有可检测的起点预设，且被修正
· 对话包含 [U/U] 事件或 [U/AI] 高自主性事件（DIAGNOSE 对应清单非空）
```

三项均不满足 → 跳过 Phase 3，输出：`> ⏭ Phase 3 跳过（无跃迁点 / 无 [U/*] 事件）`，`trace` 标志不设为 `true`。

---

## [P3-1] 结构

```markdown
# [主题标题] — 认知轨迹

> 创建时间：[与 raw 相同的 ts]

> 导航：[raw](`<prefix>`raw/`<COMMON_PATH>`) · [distilled](`<prefix>`distilled/`<COMMON_PATH>`) · [digest](`<prefix>`digest/`<COMMON_PATH>`)

---

## 入口假设

[从 DIAGNOSE 「入口假设」字段直接取值]

---

## 认知事件序列

[见 P3-2]

---

## 认知结构变化

[见 P3-3]

---

## 遗留

[仅来自 User 明示的未解决问题；User 未提则省略此 section]
```

---

## [P3-2] 认知事件序列

从 `DIAGNOSE` 的「全量事件序列」提取。只保留以下类型：

```text
· [U/U] 事件
· [U/AI] 高自主性事件
· 跃迁点（若 DIAGNOSE 中非空）
· 使前序疑问闭环的落点事件
```

每个事件写成一个 **blockquote 段落**：

```markdown
> **[归属]** 触发：[触发条件]
>
> [一句话描述认知动作]
>
> 落点：[打开疑问 X / 终结疑问 X / 校准偏差 X]
```

---

## [P3-3] 认知结构变化

```markdown
**学习起点**：[入口假设对应的认知状态，来自 DIAGNOSE]

**关键跃迁**：[DIAGNOSE 「跃迁点」字段；若为空则省略此行]

**落点**：[最后一个闭环事件后的认知状态，1-2 句]
```

---

## [P3-4] 落盘

将生成内容写入 `CACHE_TRACE`（`{archive_root}/.cache/<topic-path>/<ts>-<slug>-trace.md`），然后写入本地 archive：

```
{archive_root}/trace/<topic-path>/<ts>-<slug>.md
```

同时更新 `{archive_root}/index.json`，将对应条目的 `"trace"` 设为 `true`。

完成后输出：`> ✅ Step 3 完成 · trace：trace/<topic-path>/<ts>-<slug>.md`
