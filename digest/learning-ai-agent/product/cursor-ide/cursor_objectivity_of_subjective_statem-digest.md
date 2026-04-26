# 主观陈述的客观性 — 要点摘要

> **导航**：[distilled](../../../../distilled/learning-ai-agent/product/cursor-ide/cursor_objectivity_of_subjective_statem-distilled.md) | [raw](../../../../raw/learning-ai-agent/product/cursor-ide/cursor_objectivity_of_subjective_statem-normalized.md)

## 要点

1. **三类 AI 接受错误前提的独立机制**
   多轮锚定（已讨论≠已确认）、批判性过度矫正（为显有力摆向反方向）、错误前提追因（Why X 预设 X 为真）。共同根源：AI 的目标是"回答问题"，前提审查是额外的元认知步骤，默认不触发。

2. **Agent 场景的偏差放大有条件，不是绝对**
   在无监督、无 tracing、含不可逆操作时危害更大；有 checkpoint 和审查节点时与对话场景相近。"Agent 本质上更严重"是泛化表述——本次对话的自我审查发现这个结论本身就犯了所讨论的泛化谬误。

3. **偏差来源覆盖整个协作链路，不止用户输入**
   用户输入偏差（主观认知偏差）、AI 生成偏差（训练分布、倾向滚雪球）、检索信息偏差（搜索排名、RAG 文档质量）三类叠加。AI 对检索结果的信任度反而高于用户输入（"外部来源"外衣效应），协作层防御必须覆盖全链路。

4. **元认知指令是有效但有边界的防御手段**
   Cursor Evidence First 规则使简单多轮锚定测试失效，机制是在每轮注入持续激活的前提审查指令。防御边界：深度嵌入技术细节的前提、权威断言、极长上下文中的早期前提仍可能突破防御。真正的风险不在 AI 能力上限，而在"使用方式的默认设定"上。
