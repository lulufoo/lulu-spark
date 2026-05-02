# 领域建模驱动 AI：编程协作设计(从AI打补丁说起） — 摘要

> 创建时间：2026年5月1日 17:28

> 导航：[raw](../../../raw/ai-collaboration-framework/domain-modeling/202605011728-patch-first-vs-domain-modeling-collaboration.md) · [distilled](../../../distilled/ai-collaboration-framework/domain-modeling/202605011728-patch-first-vs-domain-modeling-collaboration.md) · [trace](../../../trace/ai-collaboration-framework/domain-modeling/202605011728-patch-first-vs-domain-modeling-collaboration.md)

---

## 核心结论

在 AI + 人类协作编程中，人类通过领域建模（从架构到方法级的结构约束）定义代码的形状，AI 在形状内施工。避免"规则补丁"是这种协作规范的自然结果，不是目标本身。AI 加速了实现，但没有降低架构思考的难度——协作层能让代价变得显式和可管理，不能消除它。

---

## 关键概念

**AI打补丁**：AI 在没有外部结构约束时用最小改动、最具体规则响应问题的结构性倾向，根源于训练数据偏向、RLHF 强化具体性、逐 token 顺序生成三个机制

**任务分层**：将 AI + 人类协作拆分为认知类型不同的独立阶段（架构/实现/审查），阶段间单向传递——上层输出是下层的意图约束，不回流

**强区间覆盖弱区间**：混合认知类型任务中，AI 会用实现（强区间）覆盖架构（弱区间）的结构性倾向，导致架构输出被实现惯性吞掉

**代码形状约束**：通过领域模型/设计模式/类型系统/测试等手段，预先定义代码结构，使 AI 的自由度被限定在填充行为，而非结构决策

**设计模式词汇表**：实现阶段向 AI 提供的设计模式约束——实现层对应领域建模的细粒度版本；设计模式是对结构的预定义（事前），不是代码质量规则（事后）
