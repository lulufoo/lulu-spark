# 思维工具的客观与主观维度 — 要点摘要

> 创建时间：2026年4月25日 15:32


> **导航**：[distilled](../../../distilled/ai-thinking-framework/thinking-pattern/202604251532-thinking-tools-objective-and-subjective-dimensions-distilled.md) | [raw](../../../raw/ai-thinking-framework/thinking-pattern/202604251532-thinking-tools-objective-and-subjective-dimensions-normalized.md)

## 要点

1. **新工具与 LCCM 的本质区别：认知主体不同**
   LCCM 的对象是"某个概念/世界"，新工具的对象是"用户自己的认知框架"。LCCM 问"这个概念是什么"（深度），新工具问"你在用什么镜头看？你还可以换什么镜头"（元认知）。防退化保障：禁止展开概念内容，只输出视角标签；必须要求用户先描述当前认知作为前置输入。

2. **五阶段核心流程，Phase 1（锚定领域）必须先于 Phase 2（视角诊断）**
   不先知道领域，就无法判断用户视角是否局限。Phase 2 用 AI 主动 3-5 轮追问充分诊断，内部构建视角覆盖地图（基于 LCCM 侧面词汇表），不直接输出给用户。Phase 3 用差集法（高价值侧面 − 用户已覆盖侧面）输出 3-4 个可探索视角。

3. **Phase 5 必须是直接结论式输出，不能引导式推导**
   工具的核心价值在前四个阶段（诊断、匹配、打破边界），Phase 5 是交付结果。引导式推导会让工具退化为 LCCM——一旦开始"引导用户逐步理解某个视角下的知识"，工具本质就变了。视角下的深化需求交给 LCCM 接力处理。

4. **LCCM 的侧面词汇表可直接作为视角匹配的基础**
   用户自然产生的视角（"别人是如何学习的" → WHO；"提示词工程的底层" → WITH+HOW）恰好落在 LCCM 侧面里，证明这套词汇表对视角分类有足够的覆盖度。两工具形成接力：新工具打破视角默认值 → LCCM 深入选定视角。
