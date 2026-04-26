# Agent 上下文窗口与任务粒度设计模型

> **导航**：[digest](../../../../digest/ai-software-dev/agentic-coding/agentic-design-patterns/context-window-task-granularity-model-digest.md) | [raw](../../../../raw/ai-software-dev/agentic-coding/agentic-design-patterns/context-window-task-granularity-model-normalized.md)

> 本文档基于一次 Agent 机制调查式对话整理。
> 目标是重建推理过程——跟着推导走一遍，而不是直接读结论。

## 本次对话在追什么

对话从一个关于上下文窗口滑动模型的文档出发，通过工具查证 Copilot Chat、Cursor、Claude Code 的实际实现，最终建立了一套 Agent 任务设计原则，并将其映射到对话蒸馏工具（DDM）和代码生成任务的具体实践：

- 转折 1：Copilot Chat 的上下文管理不是简单的"FIFO 截断"，而是优先级裁剪 + LLM 压缩摘要两层机制
- 转折 2：Cursor 和 Claude Code 都有类似机制（Cursor 服务端压缩，Claude Code 显式 /compact 命令），说明这是所有 LLM Agent 的共同设计模式
- 转折 3：Agent 的 compaction 处理的是"执行历史（turn）"，而不是"文件内容"——不能依赖 Agent 自动处理大文件的内联问题
- 转折 4：大文件的上下文压力与大对话文件是同构问题，解法也相同：大制品不进上下文，进上下文的是大制品的索引

---

## Copilot Chat 的上下文管理：优先级裁剪 + LLM 压缩

通过查证 Copilot 扩展源码（`extension.js`），实际机制是两层：

**第一层：优先级裁剪（Prompt Rendering 阶段）**

Prompt 不是"一段文字"，而是一棵带优先级的组件树（`PromptElement`）。渲染时按 token budget 从高优先级向低优先级填充，budget 耗尽时低优先级（旧对话）自动不被写入请求。这是"滑动"——**不是删除，是从不写入**。

优先级顺序：系统 prompt / 当前 query > 近期历史 > 远期历史

**第二层：LLM 压缩摘要（BudgetExceededError 触发）**

当第一层裁剪后 token 仍然不够，触发 `ConversationHistorySummarizer`：单独发起一次 LLM 请求生成历史摘要，摘要替换原始历史轮次，UI 显示"Compacting conversation..."。

这与文档描述的"静默截断"有本质区别：Copilot 不是丢弃，而是**用摘要替换**。落盘文件的价值不受影响——文件通过工具调用注入，不依赖 turn 历史是否存活。

---

## Cursor 和 Claude Code 的对应机制

✅ 已验证（`cursor-agent-exec/dist/main.js`）：Cursor 有专门的 `preCompact` hook + `compaction` 机制，`force_summarization` 字段可强制触发，`summarization_strategy` 枚举挂在 `CodeChunk` 上允许每个代码块有不同策略（全文/摘要/仅签名）。`COMPRESSED_COMPOSER_FILE`（值=2）区别于普通 `COMPOSER_FILE`，是"大文件被压缩表示"的直接证据。Cursor 的摘要压缩发生在服务端（`aiserver.v1`），与 Copilot 的客户端触发不同。

✅ 已验证（Claude Code 官方文档）：`/compact` 命令是公开的设计——Claude 对整个对话生成结构化摘要，替换原始历史，新 turn 从摘要继续。本地 `~/.claude/sessions/` 独立存储每个会话，本身就是"落盘 + 分 Phase"的体现。

**共同结论**：上下文优先级管理 + 摘要压缩是所有主流 LLM Agent 的标准模式，不是 Copilot 特有的。

---

## Agent 任务设计原则

基于上述机制，从任务设计角度提炼五条原则：

| 原则 | 一句话 | 机制依据 |
|------|--------|---------|
| **P1 落盘** | 产出写文件，上下文不是存储 | Compaction 后 turn 历史变摘要 |
| **P2 依赖最小** | Phase 间依赖落盘物，不依赖过程细节 | 过程细节会被滑走，落盘物永久存在 |
| **P3 可恢复** | 框架文件 + 落盘文件 = 完整恢复 | 对应 `force_summarization` 正常状态转换 |
| **P4 优先级排序** | 关键约束写在前面 | 对应 PromptRenderer 优先级树 |
| **P5 粒度控制** | 单轮操作量适中，控制压缩时机 | `modelMaxPromptTokens * zji` 触发阈值 |

**用户提出的核心原则**——主动分阶段，依赖保持近窗口——本质上是 P1 + P2 的组合：把阶段产出落盘，让每个阶段只依赖上一阶段的落盘文件而非它的执行过程。

---

## 大文件与大对话文件：同构问题，同构解法

对话中深入讨论了 DDM 中代码内联的问题，并将其推广到代码生成任务。

**关键澄清**：Agent 的 compaction 处理的是"执行历史（turn）"，不是"文件内容"。文件内容是作为工具结果注入上下文的，Agent 不会主动说"这个文件太大，帮你提取"。因此不能依赖 Agent 自动处理大文件内联。

**等价映射**：

| DDM（对话处理） | 代码生成任务 |
|---|---|
| raw 文件（对话原文）含大量代码块 | 被修改的源文件 50KB |
| P1 诊断需读全文，代码块占用诊断 token | Agent 需读全文理解上下文，大文件占用生成 token |
| phase 间依赖 raw 文件 | 跨 turn 依赖同一个大文件 |

**统一解法**：大制品不进上下文，进上下文的是大制品的索引。

DDM 的实现：在 P0 归一化阶段提取代码块（>= 15 行）到独立文件，原位替换为 `code-ref` 注释，P1 读到的是紧凑的诊断材料而非完整代码。

代码生成的等价实现：任务开始前生成文件结构概要（函数列表 + 签名 + 一句话说明）落盘为索引文件，Agent 在每个编辑子任务里只读索引 + 需要修改的具体函数，不读全文。

---

## 编程领域专属设计经验

对话还提炼了针对编程类对话蒸馏的额外设计点：

- **错误-修复对强制保留**：编程对话里的"错误→诊断→修复"三元组比"最终正确代码"更有认知价值，应在 P1 扫描时强制标记保留，不参与舍弃判断
- **代码结构可测试性检查**：P3 质量检查应补充三个结构性验证：代码块是否有足够上下文可独立运行、错误示例是否标注"这是错的"、代码版本是否有时序标记

---

## 对话中出现的事实性偏差（回答者视角）

| 触发语境 | 偏差描述 | 校准后的表述 |
|---------|---------|-------------|
| Turn 1 引用文档模型 | 文档描述"静默截断"和"滑动窗口" | Copilot Chat 实际不是截断，而是优先级不写入 + LLM 摘要替换，文档是有用的近似模型 |

---

## 遗留问题

1. DDM 的 P0 代码块提取规则（15 行门槛、提取路径结构）尚未实际写入 `ddm-p0-normalize.md`，需要在工具迭代中补充
2. P2 遇到 `code-ref` 时的处理策略（内联还是引用）尚未在 DDM 规则中定义
3. 编程场景的 P1-1b（错误-修复对扫描）和 P3-CODE（代码结构可测试性检查）作为扩展点，尚未在 DDM 主框架中体现
