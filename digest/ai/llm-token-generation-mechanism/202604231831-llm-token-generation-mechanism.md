# LLM 逐 Token 生成的完整机制 — 要点摘要

> **导航**：[distilled](../../../distilled/ai/llm-token-generation-mechanism/202604231831-llm-token-generation-mechanism.md)


## 要点

1. **自回归每步输入 = 原始 prompt + 历史所有已生成 token；串行不可并行**
   不是只用上一个 token，而是完整历史。这决定了生成天然串行（下一步必须等上一步完成）。上下文窗口是物理边界，超出则最早内容被截断；截断是应用层的工作，不属于自回归机制本身。

2. **Attention 是动态的：同一 token 在预测不同位置时权重完全不同**
   Attention 分数是"当前预测位置对历史 token 的关系"，不是 token 自身的固定属性。因果掩码保证只能看之前的 token（单向从左往右）。自回归规定生成节奏，Attention 决定每步如何看历史——两者分工，缺一不可。

3. **历史 token 的工程优化是缓存（KV Cache），不是过滤**
   不能提前过滤低权重 token：Attention 是动态的，某步权重低的 token 后续步可能权重高。KV Cache 缓存历史 token 的中间计算结果，每步只需计算新 token，不改变自回归语义，模型仍"看到"完整历史。
