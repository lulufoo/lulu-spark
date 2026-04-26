# AI 任务执行范式的天花板：prompt 范式、Agent 调度与领域知识密度 — 要点摘要

> **导航**：[distilled](../../../distilled/ai-assisted-domain-learning/domain-deepening/ai-task-execution-paradigm-and-domain-learning-ceiling-distilled.md) | [raw](../../../raw/ai-assisted-domain-learning/domain-deepening/ai-task-execution-paradigm-and-domain-learning-ceiling-normalized.md)
> 来源：ai-task-execution-paradigm-and-domain-learning-ceiling-distilled.md（distilled/）
> 生成时间：2026-04-26

## 要点

1. **多文件拆分不改变 prompt 范式**
   .md 文件被读进上下文后，执行仍是 LLM 单次生成"顺着文本走"。多文件只是人类作者的组织工具，LLM 执行时两个文件都被展平进同一上下文。真正改变范式的是代码调度（LangGraph），它让 LLM 只负责节点内的策略判断，调度决策由代码保证。

2. **MD 驱动 vs LangGraph：根本区别只有"谁调度执行"**
   两种方式的领域知识都仍在 prose 里。LangGraph 固化的是流程拓扑，不是领域模型。MD 驱动的问题不是写得不够好，而是 LLM 同时负责策略判断和调度决策，两件事的错误叠加，这是无法通过改写 prompt 跨越的结构差异。

3. **meta-agent 的瓶颈是基础模型的领域知识密度，不是任务复杂度**
   meta-agent 要生成某领域的 Agent，自己必须先理解那个领域，知识密度要求递归叠加。真实落地路径是：专域预训练模型 + 人工设计工作流 + LLM 节点执行，meta-agent 在这条路上几乎缺席。

4. **知识密度和推理架构能力是两个独立瓶颈，作用在不同步骤**
   深挖节点依赖知识密度（子概念边界在知识里，不在推理里）；结构重组依赖推理架构能力（元认知判断，知识多也无帮助）。关键区分：深挖是量变问题（更多 token 有帮助），结构重组是质变问题（LLM 用同一套权重检验自己，多算只会强化错误框架）。
