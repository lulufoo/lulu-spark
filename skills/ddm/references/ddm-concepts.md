# 对话蒸馏模型（DDM）— 理论说明

> 版本：v2.8（新增 trace 认知摘要层）  
> 适用对象：DDM 执行规则（DDM_P1 / DDM_P2 / DDM_P3 / DDM_P4）的概念前置，Phase 1 和 Phase 2 执行时需加载。

---

## 这是什么？

**对话蒸馏模型（DDM）** 是一套把对话转成可复用认知文档的框架。

distilled 是原始对话的**最小可理解子图**：User 节点完整保留，AI 节点按依赖规则裁剪，共同构成足以重建理解过程的最小必要结构。保留标准由四条规则形式化定义，见下节。DDM 不复现原始对话，也不只摘录结论。

---

## 模型文件注册表

| 标识 | 文件名 | 角色 |
|------|--------|------|
| `DDM_ENTRY` | `dialogue-to-doc.md` | 执行入口 |
| `DDM_CONCEPTS` | `ddm-concepts.md` | 概念定义（本文） |
| `DDM_P0` | `ddm-p0-normalize.md` | Phase 0 执行规范 |
| `DDM_P1` | `ddm-p1-diagnose.md` | Phase 1 执行规范 |
| `DDM_P2` | `ddm-p2-generate.md` | Phase 2 执行规范 |
| `DDM_P3` | `ddm-p3-trace.md` | Phase 3 执行规范 |
| `DDM_P4` | `ddm-p4-digest-archive.md` | Phase 4/5 执行规范 |

---

## 归档文件（cognitive-trace-archive）

适用于 **CTA_BASE**（`archive_root`，从 `config.json` 读取）；`<topic-path>` 在 Phase 0 Step 1 确定后全程不变。

**符号定义**

| 符号 | 类型 | 约束 |
|------|------|------|
| `ts` | string[12] | `YYYYMMDDHHMM`，Phase 0 落盘时东八区本地时间；同一轮归档共享 |
| `slug` | string | 全小写连字符；不含 `ts` |
| `topic-path` | string | Phase 0 Step 1 确定，全程不变 |

**路径定义**

    COMMON_PATH  := <topic-path>/<ts>-<slug>.md
    RAW          := CTA_BASE/raw/<COMMON_PATH>
    DISTILLED    := CTA_BASE/distilled/<COMMON_PATH>
    DIAGNOSE     := CTA_BASE/diagnose/<COMMON_PATH>     -- Phase 1 诊断摘要
    DIGEST       := CTA_BASE/digest/<COMMON_PATH>       -- 可选，无则不建
    TRACE        := CTA_BASE/trace/<COMMON_PATH>        -- 可选，无则不建

**文内创建时间**：一级标题下一行写 `> 创建时间：YYYY年M月D日 HH:MM`（月日不补零），须与 `<ts>` 一致。

**`index.json`（真源在 CTA_BASE/index.json）**

```json
{
  "version": 4,
  "entries": {
    "<id>": {
      "common_path": "<topic-path>/<ts>-<slug>.md",
      "created_at": "<ts>",
      "raw":         boolean,
      "distilled":   boolean,
      "diagnose":    boolean,
      "digest":      boolean,
      "trace":       boolean
    }
  }
}
```

`raw / distilled / diagnose / digest / trace` 均为布尔值，表示对应层文件是否存在。

**链接维护**：只替换完整相对路径（`../../../<layer>/…/<ts>-<slug>.md`）；禁止对已含 `<ts>-<slug>` 前缀的路径再做 basename 替换，避免双前缀。

---

## 缓存文件

```
- CACHE_INDEX     := CTA_BASE/.cache/index.json
- CACHE_RAW       := CTA_BASE/.cache/<topic-path>/<ts>-<slug>-raw.md
- CACHE_DISTILLED := CTA_BASE/.cache/<topic-path>/<ts>-<slug>-distilled.md
- CACHE_TRACE     := CTA_BASE/.cache/<topic-path>/<ts>-<slug>-trace.md
- CACHE_DIGEST    := CTA_BASE/.cache/<topic-path>/<ts>-<slug>-digest.md
```

---

## distilled 的结构定义

**distilled = 原始对话的最小可理解子图**

| 节点 | 处理方式 |
|---|---|
| User 节点 | 完整原文，一字不改（含错别字、口语、不完整句） |
| AI 节点 | 仅在依赖路径上保留，裁剪到最小 |

**四条保留规则**（执行时从后往前扫）：

- **规则 1**：A[n] 支撑 U[n] 的理解——AI 输出服务于紧接的前一条 User
- **规则 2**：A[n] 支撑后续任意 U[m] 的理解——AI 输出是后续 User 的前提
- **规则 3**：A[n] 支撑后续保留的 A[m] 的理解——依赖传递闭包，需从后往前扫
- **规则 4**：被保留的 A[n] 内部自圆其说——不允许因裁剪导致内部思维跳跃；若保留了结论，使其成立的最少内部前提必须同时保留

执行顺序：先用规则 1-3 确定哪些 AI 节点保留；再用规则 4 对每个保留节点做内部一致性检查，按需补全最少前提。

---

## 保留什么，删除什么？

对保留的 AI 节点内部，裁剪只做两步：**先过滤噪音，再排除冗余**。

**第一步：噪音过滤。**  
不携带认知推进价值的内容直接删除，例如语气词、寒暄、过渡性确认、已达成共识后的重复校准、协议内部状态标注（如「内部判断，不输出」段落）。

**第二步：冗余判断。**  
通过噪音过滤后，再判断它是否与已保留内容表达的是同一件事。重复表述删除；非重复内容保留。

保留边界：**它是否对路径重建不可替代**。

**例外：反直觉结论的证据。**  
若某结论违背常见直觉，即使结论文字已保留，支撑它的原始证据（源码、日志、文档原文等）仍不视为冗余。

---

## 双轴归属模型

归属从两个独立维度判断：

| 轴 | 描述 |
|---|---|
| **认知轴** | 谁完成了这个认知动作：[U] 用户 / [A] AI |
| **引导轴** | 谁决定了这一步往哪走：/AI / /U |

两轴正交，构成四个象限：

| 象限 | 定义 | 文档叙述重点 |
|---|---|---|
| **[U/U]** | 用户自发发起并完成 | 保留用户主动推进的主体性 |
| **[U/AI]** | AI 设计方向，用户完成 | 区分复述与独立产出 |
| **[A/AI]** | AI 提问并给出答案或纠偏 | 作为路径背景或校准补充 |
| **[A/U]** | 用户提问，AI 补充 | 作为路径背景或校准补充 |

[U/AI] 再区分自主性强度：

| 强度 | 判断依据 | 叙述方式 |
|---|---|---|
| 低自主性 | 输出未超出 AI 划定边界 | 「顺着这个方向，你归纳出……」 |
| 高自主性 | 独立产出 AI 未给出的新认知 | 「在这个方向上，你独立识别出……」 |

若用户通过主动质疑或方向跳转改变了后续推进方向，则从转移点后重新判断引导轴。

---

## 认知事件的完整语境

**保留一个认知事件，不等于只保留认知动作。** 一个完整认知事件至少包含三部分：

| 要素 | 丢失后的后果 |
|---|---|
| **触发条件**：前一步结论、类比、反例、约束或提问方向 | 不知道为什么走到这里 |
| **认知动作**：谁完成了什么推导 | 路径断裂 |
| **落点指向**：终结旧疑问，还是打开新疑问 | 无法判断它在路径中的位置 |

处理原则：认知事件一旦进入保留候选，立即执行**双向追溯**——向前检查触发条件是否已覆盖，向后检查它是否闭合了前序遗留问题。缺失的必要上下文必须补回。

---

## 认知事件的层次

| 层次 | 定义 | 用途 |
|---|---|---|
| **认知事件** | 一次有意义的认知推进，包含触发条件、认知动作、落点指向 | Phase 1 诊断与 Phase 4 轨迹的分析单位 |
| **归属单元** | 不可再分的最小认知动作 | Phase 1 [P1-2] 标注的内部单位，不直接输出 |
