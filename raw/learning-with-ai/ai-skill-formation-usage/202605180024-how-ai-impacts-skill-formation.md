# How AI Impacts Skill Formation：如何使用AI

> 创建时间：2026年5月18日 00:24

> 导航：[distilled](../../../distilled/learning-with-ai/ai-skill-formation-usage/202605180024-how-ai-impacts-skill-formation.md) · [digest](../../../digest/learning-with-ai/ai-skill-formation-usage/202605180024-how-ai-impacts-skill-formation.md) · [trace](../../../trace/learning-with-ai/ai-skill-formation-usage/202605180024-how-ai-impacts-skill-formation.md)

<!-- DDM:TURN_SEP:v1 -->

## User

这个论文在说什么？ https://arxiv.org/abs/2601.20245

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 1）

## 一句话

这篇论文用**随机对照实验**问：新手开发者用 AI 学一门**陌生的 Python 异步库（Trio）**时，任务做得更快了吗？**真正学会**了吗？结论是：**平均并不更快，但学得明显更差**；只有把 AI 当「陪练」而不是「代写」的用法，才更可能保住学习效果。

## 研究背景与问题

已有大量研究显示：AI 能提升生产力，**新手受益往往最大**。但一个悬而未决的问题是：若人越来越依赖 AI 完成任务，**监督 AI、纠错、兜底**所需的能力会不会反而长不起来？

两个研究问题（RQ）：

1. **RQ1**：学新技能时，AI 是否加快任务完成？
2. **RQ2**：用 AI 是否损害对这些新技能的掌握？

## 实验怎么做

- **对象**：参与者完成基于 **Trio** 的编码任务。
- **设计**：随机分组——一组可用 **GPT-4o** 编码助手，一组不用。
- **评估**：概念理解、读代码、调试、写代码等。

## 主要发现

### 1. 技能：用 AI 的一组明显更差

库相关技能测评分数约 **下降 17%**，受损的主要是：**概念理解、读代码、调试**。

### 2. 效率：平均并没有明显更快

整体上，**AI 组并未显著缩短完成时间**。

### 3. 完全外包：有一点快，但几乎没学会

**把编码任务完全交给 AI** 的参与者，生产力略有提升，但**几乎没学到这门库**。

### 4. 六种 AI 用法，三种更「保学习」

六种模式，其中 **3 种**仍保持较高测评分数，共同点是**认知参与更多**。

## 论文想传达的结论

| 常见假设 | 论文证据 |
|---------|---------|
| AI = 又快又好 | **快**并不稳定；**好（学会）** 在很多用法下会受损 |
| 新手用 AI 最划算 | 短期产出可能上去，但**监督 AI 的能力**可能被透支 |
| 生产力 ≈ 能力 | **不是捷径** |

作者：Judy Hanwen Shen、Alex Tamkin（Anthropic Fellows Program）。

<!-- DDM:TURN_SEP:v1 -->

## User

Judy Hanwen Shen、Alex Tamkin 是谁 ？

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 2）

## Judy Hanwen Shen

**现任主要身份**：斯坦福大学计算机系 **理论组博士生**，导师 **Omer Reingold**。

**研究方向**：人机协作、高级 AI 的社会影响；公平性、差分隐私、数据质量、负责任 AI。

**与 Anthropic 的关系**：这篇论文在 **Anthropic Fellows** 期间完成；主要学籍/隶属仍是 **Stanford**。

个人主页：https://heyyjudes.github.io/

## Alex Tamkin

**现任主要身份**：**Anthropic 研究员**，关注 AI 在真实世界中的影响。

**代表性工作**：Anthropic Economic Index、CLIO、Claude Artifacts、语言模型决策中的歧视评估等。

**教育背景**：斯坦福机器学习 PhD（导师 Noah Goodman）。

个人主页：http://alextamkin.com/

## 分工理解

| | Judy | Alex |
|---|------|------|
| 机构 | 斯坦福 PhD | Anthropic 全职 |
| 这篇论文 | 学术侧实验与评估 | 社会影响线 + Anthropic 资源 |

<!-- DDM:TURN_SEP:v1 -->

## User

https://www.anthropic.com/research/AI-assistance-coding-skills  主要说的是什么？ 和论文是什么关系 ？

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 3）

## 这篇博客主要说什么？

Anthropic 对同一项研究的**通俗解读**（2026-01-29）。

核心：AI 能提速，但可能因**认知外包**损害**技能形成**；在编程里人还要能读代码、抓错、监督 AI。

实验：52 名工程师，Trio 两任务 + 测验；AI 组约 50% vs 手写组 67%；速度差约 2 分钟但未显著。

六种交互模式：AI delegation、Progressive AI reliance、Iterative AI debugging（低分）；Generation-then-comprehension、Hybrid code-explanation、Conceptual inquiry（高分）。

## 和论文是什么关系？

```
论文（2601.20245）  =  完整学术记录
        ↑ 同一实验、同一结论
Anthropic 博客       =  论文导读 + 政策/产品启示
```

博客多：与 80% 提速观测的对照、Claude Code vs 侧边栏助手区分、Learning Mode 建议。

论文多：完整方法、统计、文献综述。

文末 BibTeX 指向 arXiv:2601.20245，「Read the full paper」。

<!-- DDM:TURN_SEP:v1 -->

## User

核心结论是什么 ？

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 4）

## 核心结论（4 条）

### 1. 学新东西时：用 AI 更容易「没学会」

测验约 **低 17%**；受损最明显：**概念理解、读代码、调试**。

### 2. 学新东西时：平均并没有明显更快

AI 组略短但未显著；不少人大量时间花在 prompt 上。

### 3. 关键不在「用不用 AI」，而在「怎么用」

低分用法：外包、逐步依赖、代调试。高分用法：只问概念、要解释、生成后再追问。

### 4. 总判断

> **AI 带来的生产力，不是能力的捷径。**

**一句话**：学新技能时，AI 平均**帮不了多少速度、却明显伤学习**；只有**保持认知参与**的用法，才可能兼顾辅助与掌握。

<!-- DDM:TURN_SEP:v1 -->

## User

高分用法，使用AI还需要学习，那AI带来的优势是什么？

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 5）

## 先对齐：这项研究在比什么

「高分用法」不是说「用 AI 又轻松又快还学得最好」，而是：**在必须学新东西时，少牺牲掌握度**。

## 1. 在这项实验里，「高分用法」的 AI 优势

相对**完全不用 AI**：

| 可能优势 | 说明 |
|----------|------|
| 按需讲解 | 随时问概念，不必翻很久文档 |
| 卡在概念上时更快解开 | 代码仍 largely 自己写 |
| 不必为交差而全外包 | conceptual inquiry 在高分模式里往往最快 |

实验**没有**证明高分 AI 组在速度+分数上**同时显著优于**无 AI 组。

## 2. AI 的「真正优势」应放在哪类任务？

- **已会做的、重复/熟悉的活** → 观测里可有很大提速。
- **正在学的新库** → 这项 RCT：平均既不显著更快，又显著更学不会（若用法偏外包）。

## 3. 仍值得用 AI 学吗？

1. **学新东西**：AI 做 tutor，你负责写、改、debug。
2. **已经会的东西**：AI 做执行者。
3. **全外包**：适合一次性交差，不适合要会 debug 的岗位。

## 直接回答

- **不是**：让你不学还能考高分。
- **是**：降低**理解门槛**，不是取消学习。
- **相对无 AI**：高分用法是「若非用 AI 不可，怎样少亏」。
- **相对低分用法**：保留以后能独立 debug、审 AI 的能力。

一句话：**AI 的优势是「降低理解门槛」；真正的提速优势更多在「你已经会」的任务上。**

<!-- DDM:TURN_SEP:v1 -->

## User

执行： /ddm  dtd_normanize， 主题： How AI Impacts Skill Formation： 如何使用AI

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 6）

> ✅ dtd_normalize 完成 · 已写入 raw 与 distilled，index 条目 `41c8b3a4a27a6f5f822f703140d6b376`
