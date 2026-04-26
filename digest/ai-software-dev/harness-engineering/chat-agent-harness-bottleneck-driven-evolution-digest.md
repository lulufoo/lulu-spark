# 从 Copilot Chat 到 Harness Engineering：瓶颈驱动的三层技术演进 — 要点摘要

> **导航**：[distilled](../../../distilled/ai-software-dev/harness-engineering/chat-agent-harness-bottleneck-driven-evolution-distilled.md)

## 要点

1. **三层演进的驱动机制是"前者的成功制造下一层的问题空间"，不是替代关系**
   Chat 的根因是"无状态朴素拼接"（context 挤爆=无外部记忆，手动 marker=无持久状态）→Agentic Engineering 将工作记忆外化、按需拉取最小上下文、计划持久化；Agent 成功后产生新问题空间→Harness Engineering 应对漂移、QA 瓶颈、不可审计、legibility 缺失、架构 drift 被放大。

2. **Harness Engineering 回应的所有问题只在 Agent 成功之后才出现**
   代码漂移（Agent 复刻仓库里的坏模式并自我强化）、QA 人力瓶颈（写爆炸化后瓶颈转移到验证侧）、长任务不可见（单任务运行 6 小时无中间审查）、agent legibility 缺失（仓库可读性变成 agent 能力的决定因素）——这些在 Chat 时代根本不存在。

3. **手动 marker 是识别范式边界的信号，不是技巧**
   在 Chat 范式内被迫写"输出 marker 再读回来驱动状态机"，本质是在手工模拟最简陋的 agent loop。这不是用户技巧问题，而是"不做成 agent 就过不去"的范式边界——Agentic Engineering 把这一层变成架构层的默认能力。
