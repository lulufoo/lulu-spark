# AI 方案预诊断模型设计 — 要点摘要

> **导航**：[distilled](../../../distilled/ai-thinking-framework/ai-solution-pre-diagnosis-model/ai-solution-pre-diagnosis-model-design-distilled.md) | [raw](../../../raw/ai-thinking-framework/ai-solution-pre-diagnosis-model/ai-solution-pre-diagnosis-model-design-2026-04-25-normalized.md)

## 要点

1. **方案能力是三层乘法：数据质量 × LLM 能力 × 框架设计质量**
   任何一层是 0，整体输出趋近于 0。LLM 能力又可分解为"领域知识密度"和"推理架构能力"，两者独立衰减。预诊断模型的核心作用是在设计开始前定位哪一层是当前瓶颈，避免将工作量投入到非瓶颈层。

2. **诊断时机的移位：从"设计完成后"到"设计开始前"**
   传统方案评审是事后纠错，成本高且方向性问题难以修复。预诊断将"能力瓶颈在哪里"提前暴露，框架设计方向本身可以根据诊断结论调整。能力需求（稳定，由问题决定）和当前瓶颈（随 LLM 版本变化）必须分离存档，以支持重复使用和跨版本对比。

3. **自我指涉矛盾：强制质疑规则依赖被其损害的能力**
   框架设计中若加入"强制质疑所有结论"的规则，执行该规则所需的批判性推理能力，正是被"质疑一切"所破坏的能力——规则的执行前提和规则的效果互相矛盾。解法是"定向用户质疑节点"：在特定决策点引入用户掌握的真实约束（外部信息），绕过 LLM 的自我指涉回路。
