# 从 Copilot Chat 到 Harness Engineering：瓶颈驱动的三层技术演进 — 摘要

> **导航**：[distilled](../../../distilled/ai-software-dev/chat-agent-harness-bottleneck-evolution/202604231831-chat-agent-harness-bottleneck-driven-evolution.md)


## 要点

## 概述

对话从用户使用 Copilot Chat 时遭遇的两个具体症状出发（context 挤爆、手动 marker 驱动状态机），推导出这是「无状态朴素拼接」这一共同根因的两种表现，手动 marker 本质是在 Chat 范式内模拟最简陋的 agent loop。进而按「前者的成功制造下一层问题空间」的逻辑，追溯 Agentic Engineering 如何将工作记忆外化解决 Chat 瓶颈，以及 Agent 大规模成功之后才会出现的新问题（代码漂移、QA 瓶颈、长任务不可见、仓库 legibility 决定 agent 能力）如何驱动了 Harness Engineering 的诞生。
