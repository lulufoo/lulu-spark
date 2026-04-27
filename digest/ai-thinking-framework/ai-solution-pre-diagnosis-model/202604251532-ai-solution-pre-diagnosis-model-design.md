# AI 方案预诊断模型设计 — 摘要

> 创建时间：2026年4月25日 15:32


> **导航**：[distilled](../../../distilled/ai-thinking-framework/ai-solution-pre-diagnosis-model/202604251532-ai-solution-pre-diagnosis-model-design.md) | [raw](../../../raw/ai-thinking-framework/ai-solution-pre-diagnosis-model/202604251532-ai-solution-pre-diagnosis-model-design.md)


## 概述

对话从「AI 方案的能力上限是什么」切入，将瓶颈拆解为数据质量、LLM 能力、框架设计质量的三层乘法结构，设计了一个在方案设计开始前诊断当前瓶颈层的预诊断模型。在两个真实案例上执行验证时，模型触碰到自身的根本限制：若在框架中加入「强制质疑所有结论」的规则，执行该规则所需的批判性推理能力恰好是被规则本身损害的能力，形成自我指涉回路——唯一出路是在关键决策点引入用户持有的真实约束，绕过 LLM 内部检验。
