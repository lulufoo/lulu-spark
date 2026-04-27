# Agent 上下文窗口与任务粒度设计模型 — 要点摘要

> 创建时间：2026年4月25日 23:44


> **导航**：[distilled](../../../../distilled/ai-software-dev/agentic-coding/agentic-design-patterns/202604252344-context-window-task-granularity-model.md) | [raw](../../../../raw/ai-software-dev/agentic-coding/agentic-design-patterns/202604252344-context-window-task-granularity-model-normalized.md)

## 要点

1. **上下文管理不是 FIFO 截断，而是优先级裁剪 + LLM 摘要替换**
   Copilot Chat 通过 `PromptRenderer` 优先级树（近期历史高优先，远期低优先）控制 token 写入；超出 budget 时触发 `ConversationHistorySummarizer` 单独发起 LLM 请求生成摘要替换旧历史。Cursor（服务端压缩）和 Claude Code（/compact 命令）有等价机制。这是所有主流 LLM Agent 的标准模式。

2. **Agent 任务设计的五条原则**
   P1 落盘（产出写文件，上下文不是存储）；P2 依赖最小（Phase 间依赖落盘物，不依赖过程）；P3 可恢复（框架+落盘文件可完整恢复）；P4 优先级排序（关键约束放前面）；P5 粒度控制（单轮操作量适中控制压缩时机）。核心思路：把上下文依赖转换为文件 I/O 依赖。

3. **Agent 不会自动处理大文件内联，必须在任务设计层解决**
   Compaction 处理的是"turn 历史"，不是"文件内容"。文件通过工具调用注入，Agent 不会主动提取大文件中的代码块。大代码文件（50KB）和大对话 raw 文件是同构问题：解法都是"大制品不进上下文，进上下文的是大制品的索引"。

4. **编程类对话蒸馏的两个专属设计点**
   错误-修复对（错误→诊断→修复三元组）比最终正确代码更有认知价值，应在 P1 强制保留；代码质量检查应补充结构性验证：代码块上下文完整性、错误示例标注、版本时序标记。
