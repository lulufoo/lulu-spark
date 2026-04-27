# 提示词规则的两种性质：从画像模型对比中浮现的分类框架 — 摘要

> 创建时间：2026年4月26日 23:34


> **导航**：[distilled](../../../distilled/ai-thinking-framework/product-insight-portrait-model/202604262334-portrait-model-comparison-prompt-rule-classification.md) | [raw](../../../raw/ai-thinking-framework/product-insight-portrait-model/202604262334-portrait-model-comparison-prompt-rule-classification.md)
> 来源：202604262334-portrait-model-comparison-prompt-rule-classification.md（distilled/）
> 生成时间：2026-04-26


## 概述

对话从比较两个画像模型实现方案出发——固定结构（旧模型）vs 动态结构（新模型依赖视角原型识别）。核心发现是：同一条提示词规则在弱模型上是规范执行，在强模型上反而成为约束限制发挥；且强模型对契约类任务的措辞极为敏感，会主动推理出「更合理变体」而偏离原意——这意味着随模型版本持续适配提示词将成为负担。落点是将提示词分为契约层、意图层、判断层三类：契约层跨模型稳定，判断层随模型增强收缩，换模型成本因此可以控制而非线性增长。
