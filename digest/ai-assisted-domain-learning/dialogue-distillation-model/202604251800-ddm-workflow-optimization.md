# DDM 工作流优化 — 要点摘要

> 创建时间：2026年4月25日 18:00


> **导航**：[distilled](../../../distilled/ai-assisted-domain-learning/dialogue-distillation-model/202604251800-ddm-workflow-optimization-distilled.md) | [raw](../../../raw/ai-assisted-domain-learning/dialogue-distillation-model/202604251800-ddm-workflow-optimization-normalized.md)
> 来源：ddm-workflow-optimization.md（distilled/）
> 生成时间：2026-04-26

## 要点

1. **先定「三文件 + index」契约**  
   同一 `<topic-path>` 下挂 `raw`、同树 `distilled` 与 `digest`，并用 `chat_id` 串 `index.json`，否则后面导航与落盘会算错相对路径。

2. **digest 是「对 distilled 的二次压缩」**  
   采用要点式，**3–5 条**，按 distilled 章节各取一条高价值要点，每条 2–3 句；不是把整段对话再摘要一遍。

3. **「接 ACN」落到文件与删除，而不止是主模型里多一段**  
   用户纠偏后，目标变成：在 `dialogue-distillation-model/` **放** `ai-conversation-normalize` 的完整可执行稿、**删** CTA 侧旧文件，并更新 `dialogue-to-doc`、快捷引用等路径。

4. **收束用对照表**  
   Turn5 以「创建/删除/改引用」列表结题，便于你打开仓库逐项核对，而不是只记住「Phase 又加了一级」这类抽象话。

5. **读者需自证的一点**  
   本对话发生在规范迭代中；若当前 DDM 阶段编号或文件树已变，以**现仓库**为准，本 digest 不替代你对 `ddm-p*.md` 的再读。
