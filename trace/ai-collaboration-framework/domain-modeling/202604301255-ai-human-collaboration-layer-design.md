# AI + 人类协作层设计 — 认知摘要

> 创建时间：2026年4月30日 12:55

> 导航：[distilled](../../../distilled/ai-collaboration-framework/domain-modeling/202604301255-ai-human-collaboration-layer-design.md) | [digest](../../../digest/ai-collaboration-framework/domain-modeling/202604301255-ai-human-collaboration-layer-design.md) | [raw](../../../raw/ai-collaboration-framework/domain-modeling/202604301255-ai-human-collaboration-layer-design.md)

- **认知 delta**：以为三层结构（表示层/目标层/过程层）是 AI + 人类协作设计的完整框架 → 修正为表示层前还有问题定义层；各层主导性反向分布；过程约束分补偿性和替代性两类，而非越少越好
- **有效类比**：CoT ≈ 通用 CNC 机床，领域模型 ≈ 加工图纸——两者是不同来源的组合，而非高低版本关系（来自 Turn 1 AI，被 User 在后续讨论中复用）
- **反直觉**：问题定义不是 Spec 里独立的一章，而是内化在表示层结构本身——Spec 的形状是问题定义压出来的；过程约束中补偿性的（如"必须先找证据"）比目标层约束约束力更强，不可替代
- **跃迁点**：User 主动反驳"过程约束应尽量少"并举出"必须先找证据"反例，推动 AI 承认偏差并给出补偿性 vs 替代性的精确区分——把一个直觉化原则变成了可操作的判断标准
- **遗留**：结果约束（形式性 vs 验证性）是否需要独立展开建模；DFVM 与本框架的完整映射关系未展开
