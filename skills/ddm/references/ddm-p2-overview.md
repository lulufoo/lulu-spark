# DDM Phase 2：对话概要

> **配置参考**：[ddm-concepts.md](ddm-concepts.md)（路径符号定义）
>
> **输入**：`RAW`（`{archive_root}/raw/<COMMON_PATH>`）；`<topic-path>`、`<ts>`、`<slug>` 从 raw 文件路径解析。
>
> **输出**：`DISTILLED`，写入 `distilled/<COMMON_PATH>`。

> ⚠️ 生成时直接读取 `RAW` 文件，从头逐轮扫描，不依赖记忆。

---

## PART 1 — 对话概要定义

**对话概要 = 以子话题为单位的主题索引**

将整个对话按内容分组，每组提炼为：
- 一行标题（抽象话题名，不含层次术语）
- Turn 范围标识
- 一段话：聊了什么主要问题，有什么关键结论或推进

**输出块格式**：

```markdown
**[话题标题]**｜Turn X–Y

[一段话]
```

**分组原则**：
- 按内容话题聚合，不按对话框架的层次名称分组
- 一个子话题内容方向一致，典型粒度 3–8 个 Turn
- 话题标题用名词短语，对不了解对话背景的人也能看懂

**话题标题规范**：

| ✅ 保留 | ❌ 去掉 |
|---|---|
| 概念辨析 · 两种类型与边界 · 底层规律 | 感知层·定义 · 理解层·穿透 |
| 记忆机制 · 芒格与 Ausubel 的分工 | 洞察层·规律提炼 · 升层流程 |

---

## PART 2 — 执行规范

## [P2-0] 强制加载 raw（第一步，不可跳过）

执行的**第一个动作**必须是读取 `RAW` 全文（`{archive_root}/raw/<topic-path>/<ts>-<slug>.md`）。

加载完成后输出：`> ✅ 已读取 raw：<路径>`，然后才可进入 [P2-1]。

- 禁止凭记忆生成内容
- 禁止跳过任何 User Turn

---

## [P2-1] 文档结构

```markdown
# [主题标题]

> 创建时间：[YYYY年M月D日 HH:MM，月日不补零，与 raw 的 ts 一致]
>
> 导航：[digest](`<prefix>`digest/`<COMMON_PATH>`) · [trace](`<prefix>`trace/`<COMMON_PATH>`) · [raw](`<prefix>`raw/`<COMMON_PATH>`)

---

**[话题一标题]**｜Turn 1–3

[一段话]

---

**[话题二标题]**｜Turn 4–9

[一段话]

---
```

主题标题从 raw 文件标题或对话内容推断。每个话题块之间插入 `---` 分隔线。

---

## [P2-2] 生成步骤

**Step 1：扫描全部 User 轮次，识别话题走向。**

从 raw 逐轮读取，识别：话题转折点（提问方向切换、结论已达成后进入新方向、追问退回子问题等）。

**Step 2：归组，确定 Turn 范围。**

- 从 User 侧的提问方向识别话题边界
- 同一追问链（用户在同一问题下层层追问）归入同组
- 不跨越明显话题断裂点；若一轮 Turn 有承上启下作用，归入内容更重的一侧

**Step 3：逐组生成标题 + 段落。**

- 标题：名词短语，抽象话题名
- Turn 范围：`Turn X–Y`（含首尾，使用全角竖线 `｜` 分隔）
- 段落：聊了哪些主要问题 + 关键结论；不列举所有细节；一段话，不用 bullet

---

## [P2-3] 纯执行指令的处理

User 发出的纯执行指令（如「执行：dtd_normalize」「是的，继续」）：
- 若整组只有此类 Turn，将其并入相邻最近话题组的 Turn 范围，不单独建组
- 若混在内容 Turn 中，正常包含在 Turn 范围内，段落中无需提及

---

## [P2-5] 落盘

**导航前缀**：`N := |topic-path|`，`prefix := "../" × (N+1)`（与 P0 一致）。

将生成内容写入本地 archive `DISTILLED`：

```
{archive_root}/distilled/<topic-path>/<ts>-<slug>.md
```

同时更新 `{archive_root}/index.json`，将 `"distilled"` 追加到对应条目的 `"layers"` 数组（若不存在）。

完成后输出：

```
> ✅ Step 2 完成 · distilled：distilled/<topic-path>/<ts>-<slug>.md（输入来源：raw）
```
