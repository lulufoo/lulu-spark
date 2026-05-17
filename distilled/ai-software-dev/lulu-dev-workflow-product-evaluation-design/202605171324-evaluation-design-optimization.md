# lulu-dev-workflow / product：评估设计优化

> 创建时间：2026年5月17日 13:24
>
> 导航：[digest](../../../digest/ai-software-dev/lulu-dev-workflow-product-evaluation-design/202605171324-evaluation-design-optimization.md) · [trace](../../../trace/ai-software-dev/lulu-dev-workflow-product-evaluation-design/202605171324-evaluation-design-optimization.md) · [raw](../../../raw/ai-software-dev/lulu-dev-workflow-product-evaluation-design/202605171324-evaluation-design-optimization.md)

---

**产品特性背景（搜索历史）**｜Turn 1–3

- Turn 1：以 `/product` 启动，要求为主页顶部搜索框在 workbench / 知识库两种模式下分别增加搜索记录（各最多 10 条）。
- Turn 2：确认产品文档可进入评估阶段。
- Turn 3：再次确认进入评估阶段（第二次请求；AI 正文已省略）。

---

**流程缺陷与根因排查**｜Turn 4–7

- Turn 4：对照架构文档提出两大问题——`workflow-state.md` 状态迁移不可用、评估后缺少独立 `pdqa-review.md` 且直接改 `product-doc.md`。
- Turn 5：聚焦问题 1，追问如何解决状态文件写入失败。
- Turn 6：要求先做实验验证 Plan 模式下 hook 事件的 `contents` 字段，暂不做架构调整。
- Turn 7：聚焦问题 2，追问缺少独立评估文档是否为 SKILL 约束所致。

---

**评估子状态机设计**｜Turn 8–11

- Turn 8：将流程失控归因于 SKILL 缺少状态机约束，询问是否应新增约束。
- Turn 9：提议新增 `evaluate-state.md` 子状态机以约束评估迭代。
- Turn 10：同意按「问题 A」设计状态转移及条件。
- Turn 11：澄清 `pending` / `in_progress` / `complete` 时序，并追问 `ReadyForDelivery` 前置条件及多轮评估如何衔接。

---

**多轮评估与目录结构**｜Turn 12–14

- Turn 12：要求 `pdqa-review` 与 `workflow-state.md` 增加评估轮次 `e{M}`，新轮次递增。
- Turn 13：追问同一 `conv_id` 多次产品文档修改如何区分，是否用 `r{N}` 文件夹递增。
- Turn 14：追问如何获知当前 `r{N}`，并说明 `product-doc.md` 应在评估完成后一次性同步。

---

**双轮次模型与线性约束**｜Turn 15–16

- Turn 15：纠正 AI 混淆——产品文档轮次 `r{N}` 与评估轮次 `e{M}` 是两个维度。
- Turn 16：确认两维轮次均线性向前、不可回退，以简化方案。

---
