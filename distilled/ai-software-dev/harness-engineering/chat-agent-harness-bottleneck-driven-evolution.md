# 从 Copilot Chat 到 Harness Engineering：瓶颈驱动的三层技术演进

> 本文档基于一次方案推演式对话整理。
> 目标是重建理解的过程——跟着推导走一遍，而不是直接读结论。

## 对话目标与边界

这次对话要回答的不是"这三个概念分别是什么"，而是一个更具体的问题：**为什么 Copilot Chat 之后会出现 AI Agent，而 AI Agent 之后又会出现 Harness Engineering？驱动这条链条的到底是什么？**

边界：不讨论具体框架实现（LangChain、AutoGen 等），不讨论模型能力本身的演进，只讨论"上一层技术的成功如何制造出下一层的问题空间"。

## 1. Copilot Chat 的使用瓶颈到底卡在哪里

最初的直觉来自你使用 Copilot Chat 做复杂任务时的两个具体症状：

- 任务稍复杂，文件 + 对话上下文 + 输出堆起来就把 context 挤爆，而且没有上下文压缩
- 任务中途加载几个文件就会把早期状态挤掉，新请求没有状态，只能自己往输出里写 marker，下一轮再把 marker 读回来当状态机驱动

你主动把这两个症状并列提出来。顺着往下推，会发现它们不是两个独立问题。

这里的关键是：它们是**同一个根因的两种表现**——裸 LLM Chat 本质上是"无状态 LLM 调用的朴素拼接"，每一次请求都需要把历史重新塞进 context，没有外部记忆，没有跨请求的持久状态，也没有按任务动态取用上下文的能力。

- context 被挤爆 → 是"没有压缩和外部记忆"的直接后果
- 需要手动 marker → 是"没有持久状态"的直接后果

你手动做的"输出 marker 再读回来驱动状态机"，本质就是在手工模拟一个最简陋的 agent loop。这个识别很重要：它说明你在 Chat 范式内已经被逼到了范式边界上——不是你想用 agent，而是**不做成 agent 就过不去**。

## 2. Agentic Engineering 具体在解决什么

很容易把 Agent 理解成"context 更长、能调工具、能多轮"的 Chat 增强版。但这样理解会错过它真正的突破点。

顺着上一节的根因继续推：Chat 的问题是"无状态的朴素拼接"，那 Agentic Engineering 的突破也必然发生在**状态这一侧**，而不是发生在"模型一次能看多少字"这一侧。

具体地说：

- **工作记忆外化**：plan、task list、中间结果、历史决策都写到外部（文件、scratchpad、memory store），不再堆在对话里
- **按需拉取最小上下文**（context engineering）：每次 LLM 调用只注入当前子任务需要的内容，而不是全量历史
- **任务规划机制化**：plan 成为持久化工件，agent 可以读回恢复执行，而不是依赖对话连续性
- **工具调用结果按需注入**：结果进外部状态，不默认全部塞回 context

把这四件事对照回你手动 marker 的做法：你在做的就是一个极度简化的工作记忆外化 + 状态恢复机制。Agentic Engineering 只是把你**被迫手工做**的那一层，变成了架构层的默认能力。

所以这一层的出现**不是偶然创新**，而是对 Chat 时代"无状态瓶颈"的直接结构化回应。

## 3. Harness Engineering 回应的是什么新问题

到这一层，疑问变了：既然 Agentic Engineering 已经把 context、planning、tool use 都解决了，Harness Engineering 为什么还需要存在？它到底在回应什么？

你给出的第一个直觉是：Agent 解决了基础能力问题，但**业务层的执行可控性和结果可信度仍然解决不了**——这些问题和具体任务耦合，无法在 agent 框架层统一处理。所以需要一种"驾驭工程"作为方法论，让 AI 的执行更可控、结果更有预期。

这个直觉的方向是对的。但"驾驭"这个中文翻译容易把 Harness 推向哲学口号那一侧，需要落地到具体的问题。

读 OpenAI 的 [Harness engineering: leveraging Codex in an agent-first world](https://openai.com/index/harness-engineering/) 会发现一个更关键的特征：**Harness 回应的所有问题，都只在 Agent 范式成功之后才会出现**。

列五个具体的问题，对应文章中的原始论述：

| 新问题 | 来源 | 为什么 Chat 时代不存在 |
|---|---|---|
| Agent 输出漂移 / AI slop | "Codex replicates patterns that already exist in the repository—even uneven or suboptimal ones. Over time, this inevitably leads to drift." | Chat 每轮都是新起点，输出不会沉淀回代码库自我强化 |
| QA / 验证瓶颈 | "As code throughput increased, our bottleneck became human QA capacity." | Chat 时代瓶颈在"写"；agent 把写爆炸化，瓶颈转移到"读和验证" |
| 长任务过程不可审计 | "Single Codex runs work on a single task for upwards of six hours" | Chat 是一次性问答，不存在 6 小时不可见的执行过程 |
| Agent legibility 缺失 | "From the agent's point of view, anything it can't access in-context while running effectively doesn't exist." | Chat 由用户自己补上下文；agent 必须自己找，仓库可读性变成决定因素 |
| 架构 drift 被放大 | "This is the kind of architecture you usually postpone until you have hundreds of engineers. With coding agents, it's an early prerequisite." | 人有经验兜底，agent 没有，任何坏模式会被立即、全面地复制 |

这张表带出一个关键校准：你最初说的"Harness 既驾驭 LLM 也驾驭 Agent"里面，**真正的问题域是 Agent 那一侧**。虽然 Harness 落地下去 LLM 的调用也更可控了，但那是副产物。它不是为了让 LLM 调用更稳定而生的，是为了让大规模、长周期、自主执行的 agent 仍然可控而生的。

这是一个反直觉的结论——名字里没有 "agent"，官方标题里却已经写得很明白："**in an agent-first world**"。

## 4. 三者为什么会依次出现

这里出现了这次对话最关键的方向修正——来自你。

AI 在 Turn 1 把你的论述理解成了"三者是不同抽象层，所以不是替代关系"。你 Turn 2 明确纠偏：你本来就没说是替代关系，你说的是**前者使用遇到瓶颈，而带来后者的出现**——这是**瓶颈驱动的演进**，而不是"谁替代谁"。

这个修正把讨论从"静态分层"推到了"动态驱动"，是这次对话的真正落点。

把前三节放到这个框架里重组：

```
裸 LLM Chat
  ↓ 被广泛使用 → 暴露瓶颈：无状态、context 挤爆、任务规划无法承载
Agentic Engineering
  ↓ 被广泛使用 → 暴露瓶颈：drift / QA 瓶颈 / 长任务不可控 / legibility / 架构 drift 放大
Harness Engineering
  ↓ 被广泛使用 → 暴露瓶颈：？（尚未显现）
```

这里有一个很重要的观察：**每一代技术的成功本身，才是下一代问题空间的制造者**。

Chat 不流行，就不会有人用它做复杂任务，context 和状态瓶颈不会暴露成结构性问题。Agent 不大规模进生产代码库，drift / AI slop / QA 瓶颈也不会浮现——这些问题在玩具工程里根本不存在。所以下一代技术不是"更好"的上一代，而是**上一代普及之后才暴露出来的隐性约束的显性化**。

这个模式在技术史上有明确的同构案例：

- 结构化编程普及 → 规模问题暴露 → OO / 设计模式 / 架构模式
- 单体架构普及 → 扩展与部署问题暴露 → 微服务
- 微服务普及 → 服务协调与可观测性问题暴露 → Service Mesh / Observability Engineering

三者的关系可以用一句话收：**每一层抽象的普及，都会把下一层的隐性约束暴露成显性问题**。LLM Chat → Agent → Harness 只是这条历史规律在 AI 编程领域的最新一次展开。

## 对话中出现的事实性偏差（回答者视角）

> 以下是本次对话推导过程中出现的明确事实性偏差，记录在此供后续参考。

| 偏差描述 | 准确表述 |
|---|---|
| 无明确事实性偏差 | — |

## 对话中出现的引导质量问题（回答者视角）

> 以下是本次对话中出现的引导策略失效或方向偏移，记录在此供路径复用时参考。

| 问题描述 | 影响范围 | 对话中的处理结果 |
|---|---|---|
| AI Turn 1 将用户的"瓶颈驱动演进"部分解读为可能的"替代关系"，并主动花篇幅澄清"不是简单的替代、是不同抽象层"。用户原文并未主张替代。 | Turn 1 AI 回复中有一整节是在反驳用户并未提出的观点，挤占了本应用于验证和深化用户真实论断的篇幅 | 用户 Turn 2 明确纠偏："我的意思并不是说他们是渐进的替代关系，而是说前者使用遇到了瓶颈，而带来后者的出现。" Turn 2 AI 确认偏差并沿用户修正后的框架继续推进。 |

## 遗留问题

1. 将"瓶颈驱动的技术代际演进"作为 mental model 沉淀到 `docs/principles/ai-driven-coding/` 下，对齐现有 ARCHITECTURE_PRINCIPLES 风格
2. 把 `.cursor/rules/` 下架构治理、TR 分解、TA 生成等规则中可机械化的条款，逐步落地为 linter / 结构测试 / CI job，而不是停留在文字劝告
3. 主动观察 Harness Engineering 普及之后会暴露出哪一类"当前还是隐性"的约束——这会是下一代抽象的问题空间

## 附录

### 操作清单：问题归属的验证判据

> 来源：章节 3 / 章节 4

当遇到一个"AI 做得不够好"的问题，不确定它属于 Agentic 问题域还是 Harness 问题域时，用这个二分法判据：

```
问：「这个问题在裸 LLM Chat 时代就存在吗？」

是 → 属于 Agentic Engineering 要解决的问题域
    （典型：context 不够、无状态、任务规划无法承载、工具调用缺失）

否 → 属于 Harness Engineering 要解决的问题域
    （典型：drift / AI slop、QA 跟不上 agent 速度、长任务过程不可审计、
          agent 看不到仓库外的 tacit knowledge、架构 drift 被放大）
```

这条判据的价值在于：它把"该在哪一层解决这个问题"从主观判断变成了一个可机械执行的检查——问一个问题就能定位。

---

## 对话质量诊断

### 对话质量

| 维度 | 评级 | 说明 |
|---|---|---|
| 推导过程完整度 | 高 | 包含主动猜测（用户提出演进链条）、关键反例（用户指出 AI 的"替代"解读是稻草人）、反直觉结论的证据补充（5 条 agent 时代独有问题） |
| 结论直给比例 | 低 | 80%+ 内容是围绕"为什么是这样"展开，而非"结论是什么"的罗列 |
| 关键转折覆盖度 | 高 | 关键转折（Turn 2 用户对"替代"的纠偏）被完整记录并用作章节 4 的推进起点 |
| 用户主体性 | 高 | [U/U] 事件占主导，用户始终主导方向，AI 主要承担验证、证据补充与精确化 |

**综合评级**：高质量

### 路径来源

路径来源：**用户主导**（[U/U] 占多数）

复用建议：可直接重走，文档高度自足。读者沿章节 1 → 4 的顺序走一遍，不需要额外引导就能重建完整认知路径。章节 4 的"瓶颈驱动"框架可独立抽取作为通用技术史分析工具。

### 模型适用性

**完全适用**。本次对话具备清晰的"疑问（Chat 瓶颈） → 推导（Agent 回应什么 / Harness 回应什么） → 落点（瓶颈驱动的分层演进）"主线，且中途有用户主动发起的方向修正（从"不同抽象层"修正为"瓶颈驱动"），DDM 能完整捕捉这条路径。
