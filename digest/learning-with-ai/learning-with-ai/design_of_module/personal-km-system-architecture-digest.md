# 个人知识管理系统：管道在哪里断、认知轨迹为什么消失 — 要点摘要

> **导航**：[distilled](../../../../distilled/learning-with-ai/learning-with-ai/design_of_module/personal-km-system-architecture-distilled.md) | [raw](../../../../raw/learning-with-ai/learning-with-ai/design_of_module/personal-km-system-architecture-normalized.md)
> 来源：personal-km-system-architecture-distilled.md（distilled/）
> 生成时间：2026-04-27

## 要点

1. **知识管理系统的主轴是 AADL 学习循环，而非"谁生产了内容"**
   ai-authored-learning 是学习输入材料，不是知识产物；cognitive-trace-archive 是认知过程记录；领域库（android-dev-docs 等）才是真正"属于自己"的内容。所有权边界在 distilled 之后——raw 和 distilled 均属粗加工。

2. **管道的断点在 distilled → 领域库，缺的是触发器而不是方法**
   人工合成的方向是正确的，问题是没有客观触发条件。"积累到一定量自然浮现"是感知信号，不是执行触发器——没有触发器，合成动作就依赖"想起来要整理"的时刻，最容易断。

3. **认知轨迹被 AI 输出淹没是整个系统最核心的信号丢失问题**
   对话里 AI 占篇幅 90%，但价值密度正好相反——你的 10%（疑问序列、卡点、有效类比、跃迁点）才是独一无二的。现有 distilled 在压缩知识内容的同时进一步稀释了这 10%，导致从中提取的"认知画像"反映的是 AI 的认知模式，不是你的。

4. **trace 层是解法：单独文档、固定 6 字段、与 distilled 分离**
   trace 文件只记录你的认知信号（入口假设、卡点、跳过决策、有效类比、跃迁点、遗留），与 digest 分离的原因是未来需要跨文件聚合（"你在多个主题里都用了同一类比"这种洞察需要从多篇 trace 提取）。遗留字段天然成为下一轮 AADL 的入口问题，驱动学习连续性。
