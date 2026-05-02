# DDM Phase 4：摘要

> **配置参考**：[ddm-concepts.md](ddm-concepts.md)（路径符号定义）
>
> **输入**：`CACHE_DISTILLED`；`DIAGNOSE`（`diagnose/<COMMON_PATH>`）；`<topic-path>`、`<ts>`、`<slug>` 从 `CACHE_RAW` 路径解析。
>
> **输出**：`CACHE_DIGEST`，写入本地 archive 的 `digest/<COMMON_PATH>`；同时更新 `index.json`。

---

## PART 1 — 术语定义

> 本 Phase 从 DIAGNOSE 读取字段时涉及以下术语。完整双轴归属模型见 [ddm-p1-diagnose.md](ddm-p1-diagnose.md) PART 1。

**反直觉结论候选**：DIAGNOSE 中记录的、违反多数人预设的结论，附带支撑证据标识。

**[U/U] / [U/AI] 高自主性事件**：定义见 [ddm-p3-trace.md](ddm-p3-trace.md) PART 1。

---

## PART 2 — 执行规范

> 前置：阅读 PART 1（本 Phase 术语定义）。

### [P4-0] 适用条件

digest 是可选产出。以下任意一项满足即生成：

```text
· distilled 包含 3+ 个保留 AI 块
· DIAGNOSE 的「反直觉结论候选」非空
· DIAGNOSE 中存在 [U/U] 或 [U/AI] 高自主性事件
```

三项均不满足 → 跳过 Phase 4，输出：`> ⏭ Phase 4 跳过（对话太短或无萃取价值）`，`digest` 标志不设为 `true`。

---

### [P4-1] 结构

```markdown
# [主题标题] — 摘要

> 创建时间：[与 raw 相同的 ts]

> 导航：[raw](`<prefix>`raw/`<COMMON_PATH>`) · [distilled](`<prefix>`distilled/`<COMMON_PATH>`) · [trace](`<prefix>`trace/`<COMMON_PATH>`)

---

## 核心结论

[1-3 句，来自 distilled 正文的最终落点结论，不引入新内容]

---

## 关键概念

[每个概念一行：**概念名**：一句话定义；从 distilled 提炼，不引入新内容]

---

## 反直觉点

[仅来自 DIAGNOSE 「反直觉结论候选」；为空则省略整个 section]

每个反直觉点写法：
> **结论**：[结论摘要]
> **打破的直觉**：[多数人的预设]

---

### [P4-2] 写法规则

```text
· 全部内容来源：distilled + DIAGNOSE，不引入新内容
· 核心结论：不超过 3 句，来自 distilled 最终落点
· 关键概念：与 distilled 正文术语一致
· 禁止输出 DDM 内部术语
· 导航前缀计算与 Phase 2/3 相同
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

```json
{
  "diagnose": true,
  "digest": true,
  "trace": <与 Phase 3 落盘结果一致>
}
```

覆盖写入 `{archive_root}/index.json`。

完成后输出：

```
> ✅ Step 4 完成 · digest：digest/<topic-path>/<ts>-<slug>.md
> 🗂 index.json 已更新（diagnose/digest/trace 标志已同步）
```
