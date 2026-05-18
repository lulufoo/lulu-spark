# Transformer Attention 机制的完整运作链 — 要点摘要

> **导航**：[raw](../../../raw/ai/transformer-attention-mechanism-complete-chain/202604231831-transformer-attention-mechanism-complete-chain.md)


## 要点

1. **Q/K/V 的语义分工：同一 token 同时扮演三个角色**
   Q（"我在找什么"）、K（"我是什么"）、V（"被选中后贡献什么"）由同一 token 的 embedding 乘三个不同权重矩阵得到。K/V 分离让"容易被匹配"和"贡献的语义内容"可以独立优化，强制 K=V 会让两个目标互相干扰。

2. **权重矩阵 W_Q/W_K/W_V 是全层共享的，不是每个 token 各自一套**
   所有 token 用同一组矩阵变换，但每个 token 的 embedding 不同，算出的 Q/K/V 也不同。权重矩阵在推理时固定，训练阶段持续更新，逐渐学到"K=容易被匹配，V=有用内容"的表示。

3. **Attention 三步计算：点积→softmax→加权求和**
   点积计算 Q 与所有 K 的匹配分；softmax 用 e^x 归一化（放大大值压缩小值，权重更集中）；加权求和所有 token 的 V 得输出向量。输出向量维度由 V 决定，不由 K 决定。Attention 分数是当前预测位置对历史的动态关系，不是 token 的固有属性。
