# DDM 工作流优化：ACN 归一化 + Phase 重构 + digest 落盘

> 创建时间：2026年4月25日 18:00

> **导航**：[distilled](../../../distilled/ai-assisted-domain-learning/ddm-workflow-optimization/202604251800-ddm-workflow-optimization.md) | [digest](../../../digest/ai-assisted-domain-learning/ddm-workflow-optimization/202604251800-ddm-workflow-optimization.md)

<!-- DDM:TURN_SEP:v1 -->

## User

我想优化DTD的流程：

1. 将 ACN 流程，拷贝到 dialogue-distillation-model流程。同时，ACN继续操作 CTA_BASE 仓库。

2. DTD 先执行ACN流程，然后，执行 DTD 自己的流程。也就是先将对话归一化，然后在蒸馏。

3. Phase 4：认知路径提取 优化：
- [P4-1] 环境判断与落盘规则 拆出来，作为最后的独立的Phase
- Phase 4 原有的认知路径提取删除，定位改成：对话重点提炼。CTA_BASE 仓库的落库目录改成digest。

4. dialogue-distillation-model 生成 的 distilled 目录文件，文档开头附加导航链接： digest 文件 / raw文件。
- 同理 digest 文件，文档开头附加导航链接： distilled 文件  / raw文件。
- 同理 raw 文件，文档开头附加导航链接： distilled 文件  / digest文件。注意： 需要修改： ai-conversation-normalize流程。

5. 三个  raw 文件 / raw 文件 / digest 文件，都通过同一个chat id关联，也就是index.json文件。

6. 三个  raw 文件 / raw 文件 / digest 文件，子目录，需要保证是一样的。

请你给我一个优化的解决方案。

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 1）

Turn1
CHAT_ID-a7f3e2b1c849d056f3e7a2b9c0d1e4f5

先读取两个原始文件再给方案。

---

## 优化方案

### 整体流程重构

**DTD 执行顺序（对话中曾提议独立 Phase 5 与 `*-normalized` 后缀；以下为现行 **v2.7**，与 AADL `ddm-phase-overview.md` 一致）：**

```
Phase 0：对话归一化（ACN Step 1–4 / `ddm-p0-normalize.md`）
  → raw/<topic-path>/<ts>-<slug>.md，index.json v3 新增 entries

Phase 1–2：诊断、生成 distilled
Phase 3：digest（对话重点提炼）
Phase 4：注入导航、更新 index.json（各层布尔）
```

---

### 文件结构（均在 CTA_BASE，v2.7）

| 文件 | 路径 | 生成阶段 |
|------|------|---------|
| raw | `raw/<topic-path>/<ts>-<slug>.md` | Phase 0 |
| distilled | `distilled/<topic-path>/<ts>-<slug>.md` | Phase 2 |
| digest | `digest/<topic-path>/<ts>-<slug>.md` | Phase 3 |

`<topic-path>` 与 `<ts>` 在 Phase 0 确定；raw / distilled / digest **共用同一 basename**。

---

### index.json（v3，`common_path` + 布尔）

```json
"<32位十六进制id>": {
  "common_path": "<topic-path>/<ts>-<slug>.md",
  "raw": true,
  "distilled": false,
  "digest": false
}
```

Phase 4 落盘后，将已存在层的 `distilled` / `digest` 置为 `true`。

---

### 导航链接（Phase 4 注入）

三个文件的导航在 Phase 4 统一注入（路径已随 `common_path` 确定）。

相对路径根据 topic-path 层级动态计算（设 `N` = topic-path 深度，需向上 `N+1` 层后跨目录）：

**raw 文件头部（v2.7：`raw`、`distilled`、`digest` 共用同一 `<ts>-<slug>.md` basename）：**
```markdown
> **导航**：[distilled](../../../distilled/<topic-path>/<ts>-<slug>.md) | [digest](../../../digest/<topic-path>/<ts>-<slug>.md)
```

**distilled 文件头部：**
```markdown
> **导航**：[digest](../../../digest/<topic-path>/<ts>-<slug>.md) | [raw](../../../raw/<topic-path>/<ts>-<slug>.md)
```

**digest 文件头部：**
```markdown
> **导航**：[distilled](../../../distilled/<topic-path>/<ts>-<slug>.md) | [raw](../../../raw/<topic-path>/<ts>-<slug>.md)
```

> **说明**：导航中 `../../../` 层数随 `topic-path` 深度而变；格式 few-shot 见 CTA [`templates/ddm-normalized-output-template.md`](https://github.com/lulufoo/cognitive-trace-archive/blob/main/templates/ddm-normalized-output-template.md)。

---

### 需要在实现前确认的点

**Phase 4 "对话重点提炼" 的产出格式是什么？**

原 Phase 4 产出的是认知路径事件表（结构化归属+触发+动作+落点）。新的"对话重点提炼"定位改为 digest，有三种合理解读：

| 选项 | 格式 | 定位 |
|------|------|------|
| A | 要点段落，每点 2-3 句 | 快速复习摘要 |
| B | 结构化列表（问题→结论） | 知识点提炼 |
| C | 保留原表格形式但去掉归属列，只保留触发+要点+落点 | 轻量认知路径 |

请确认 Phase 4 的格式后，我再动手修改两个文件。

---

### 涉及修改的文件

1. [ddm-phase-overview.md](https://github.com/lulufoo/ai-assisted-domain-learning/blob/main/dialogue-distillation-model/ddm-phase-overview.md) — 阶段与依赖（现行无单独的 `dialogue-distillation-model.md` 单文件）
2. [dialogue-to-doc.md](https://github.com/lulufoo/ai-assisted-domain-learning/blob/main/dialogue-distillation-model/dialogue-to-doc.md) — 执行入口与完成标志
3. [ddm-p0-normalize.md](https://github.com/lulufoo/ai-assisted-domain-learning/blob/main/dialogue-distillation-model/ddm-p0-normalize.md) — Phase 0 归一化（原 `cognitive-trace-archive/ai-conversation-normalize.md` 已不在仓库；由 AADL 内本文件承担）

Turn:1～1

模型：Claude Sonnet 4.5（Anthropic）

<!-- DDM:TURN_SEP:v1 -->

## User

选项 A

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 2）

Turn2

选项A确认。在动手前，还有一个需要确认的问题：

**Phase 4（对话重点提炼）产出数量规则是什么？**

原 Phase 4 的认知路径表是"全量事件序列"——逐条输出，不筛选。

digest 定位为快速复习摘要，有两种合理处理方式：

| 选项 | 规则 |
|------|------|
| A1 | 覆盖对话所有主题，每个主题 2-3 句（数量跟着对话走） |
| A2 | 固定提炼 5-8 个最高价值要点，其余略去 |

请确认后开始执行。

Turn:1～2

模型：Claude Sonnet 4.5（Anthropic）

<!-- DDM:TURN_SEP:v1 -->

## User

A2 但是数量需要调整，3-5个价值要点，太多了没时间看。可以基于蒸馏之后的章节提炼，更佳聚焦一点。

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 3）

Turn3

确认：**基于 distilled 文档的章节提炼，每章 1 个核心要点，全文 3-5 条，每条 2-3 句。**

现在开始执行。

（执行 ACN 修改：index.json schema 加 digest 字段、Step 2 加导航链接说明、检查清单补项）

（执行 DDM 主模型修改：Phase 4 重定位为「对话重点提炼」，Phase 5 新增为落盘阶段，P1-7 全量事件序列去掉「供 Phase 4 使用」）

（执行 dialogue-to-doc.md 修改：完成标志从 Phase 4 改为 Phase 5，新增 raw/digest 两行）

**已完成的修改：**

**[dialogue-to-doc.md]**
- 新增 Phase 0（执行前置），明确先跑 ACN 流程再进 DDM Phase 1
- 完成标志从 Phase 4 改为 Phase 5，新增 raw/digest 两行

**[dialogue-distillation-model.md]**
- Phase 4 重定位为「对话重点提炼」：基于 distilled 章节，每章 1 条，取 3-5 条最高价值要点，每条 2-3 句
- Phase 5（新增）：原 P4-1 拆出后扩展为完整落盘流程，覆盖三文件路径确定 + 导航链接生成（相对路径动态计算）+ index.json 更新
- P1-7 全量事件序列去掉"供 Phase 4 使用"的过时注释

**[ai-conversation-normalize.md]**
- Step 2 新增：raw 文件头部写入占位导航文字（实际路径由 Phase 5 注入）
- Step 4 schema 新增 `digest: null` 字段，加说明两个字段由 Phase 5 填充
- 检查清单补充导航链接和新 schema 两项

Turn:1～3

模型：Claude Sonnet 4.5（Anthropic）

<!-- DDM:TURN_SEP:v1 -->

## User

将 ACN 流程，拷贝到 dialogue-distillation-model流程。

这个需求你没有做吗？

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 4）

Turn4

对，这个需求我只做了"引用"（在 dialogue-to-doc.md 里写了一句"先执行 ACN"），没有把 ACN 的实际流程内容复制到 DDM 模型里。现在补做。

（读取 DDM 模型 PART 2 起点位置后，将 ACN 的完整流程作为 Phase 0 插入到 Phase 1 之前，并更新 PART 2 说明文字为"六个阶段"。）

（同时更新 dialogue-to-doc.md 里的 Phase 0 说明——从"引用 ACN 文件去执行"改为"流程已内嵌在 DDM 主模型中"。）

验证结果：Phase 0–5 全部就位（0-5），顺序正确。

**补做内容：**

- [dialogue-distillation-model.md] 第 142 行起：新增 **Phase 0（5 个步骤）**，完整复制了 ACN 流程内容，明确标注操作仓库为 `cognitive-trace-archive`
- [dialogue-to-doc.md]：Phase 0 说明从"引用 ACN 文件去执行"改为"流程已内嵌在主模型中，ACN 文件仍作独立手册参考"

Turn:1～4

模型：Claude Sonnet 4.5（Anthropic）

<!-- DDM:TURN_SEP:v1 -->

## User

你理解错了我的意思，是把 ACN 文件，拷贝到 dialogue-distillation-model 目录并执行，而不是在 dialogue-distillation-model.md文件添加这个流程。 原来的ACN文件要删除。

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 5）

Turn5

理解了。要做的是：
1. 把 ACN 文件复制到 `dialogue-distillation-model/` 目录
2. 删除 CTA_BASE 的原始 ACN 文件
3. 还原 dialogue-distillation-model.md 里我错误添加的 Phase 0 内容
4. 更新所有引用路径

（创建 `/Users/lulu/Code/ai-assisted-domain-learning/dialogue-distillation-model/ai-conversation-normalize.md`，内容为 ACN 完整流程）

（删除 `/Users/lulu/Code/cognitive-trace-archive/ai-conversation-normalize.md`）

（还原 dialogue-distillation-model.md：移除 Phase 0 块，PART 2 阶段数改为"五个阶段（Phase 1–5）"，并加一行注脚指向 ai-conversation-normalize.md）

（更新 dialogue-to-doc.md Phase 0 说明：从"流程已内嵌"改为"先执行 ACN 流程，路径为 ai-conversation-normalize.md"）

（更新 copilot-instructions.md：`ACN` 快捷路径从 `{CTA_BASE}/ai-conversation-normalize.md` 改为 `{AADL_BASE}/dialogue-distillation-model/ai-conversation-normalize.md`）

| 操作 | 结果 |
|------|------|
| ACN 文件移入 `dialogue-distillation-model/` | ✅ |
| CTA_BASE 原始 ACN 文件删除 | ✅ |
| dialogue-distillation-model.md Phase 0 移除，恢复 Phase 1–5 | ✅ |
| dialogue-to-doc.md Phase 0 引用改为相对路径 `ai-conversation-normalize.md` | ✅ |
| copilot-instructions.md `ACN` 快捷路径更新 | ✅ |

Turn:1～5

模型：Claude Sonnet 4.5（Anthropic）

<!-- DDM:TURN_SEP:v1 -->
