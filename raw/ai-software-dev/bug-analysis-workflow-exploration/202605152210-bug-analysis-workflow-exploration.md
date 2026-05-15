# BUG分析工作流：发散式探讨

> 创建时间：2026年5月15日 22:10

> 导航：[distilled](../../../distilled/ai-software-dev/bug-analysis-workflow-exploration/202605152210-bug-analysis-workflow-exploration.md) · [digest](../../../digest/ai-software-dev/bug-analysis-workflow-exploration/202605152210-bug-analysis-workflow-exploration.md) · [trace](../../../trace/ai-software-dev/bug-analysis-workflow-exploration/202605152210-bug-analysis-workflow-exploration.md)

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 1）

请先查看： https://github.com/lulufoo/ai-software-dev/blob/main/sys-prompt/cursor-rule-guard-design.md ，我现在有一个更大的命题，不知道是否可以基于当前的架构方案承接，请你思考。

我们在SKILL里面，A文件执行的时候，需要在某个时机，要求加载并执行B文件。这个A 和 B都是md文档。你可以理解为这是一种工作流的分阶段执行，也可以理解为某个任务的拆分为多个阶段执行。那当前的方案，是否可以扩展，支持上述的场景呢？ 如果B不加载，就不允许继续。从hook禁止执行。

<!-- DDM:TURN_SEP:v1 -->
## AI

Turn1

CHAT_ID-a7f3c8d291e04f6b85a2d09c1e7b3f40

用户意图：了解当前 cursor-rule-guard 架构是否可以扩展，支持 SKILL 工作流中「A 执行到某阶段必须加载 B，否则从 hook 层面阻断继续执行」这一场景。

先读取 GitHub 上的设计文档，以及本地的设计记录。

两份文档都读完了。现在来思考你的命题。

---
## 现有架构能承接这个场景吗？

**结论：可以扩展，但有一个结构性约束需要先确认。**

---

### 约束分析

现有系统的核心机制是：

```
"即将写文件 X" → guard 检查 rules-state → X 匹配 glob → 必须已 Read 规则 Y
```

触发守卫的锚点是**文件写操作**（Write/Edit/Shell）。

你的新命题是：

```
"A 执行到某个阶段" → 必须已 Read B → 否则阻断后续执行
```

这里有一个关键问题需要先澄清：**"A 执行到某个阶段"这个时机，在 hook 层面是什么操作？**

---

### 两种可行的扩展路径

#### 路径一：Checkpoint Write（与现有架构最契合）

原理：**把"到达阶段 N"映射为一次 mandatory 的 Write 操作**。

Skill A 的 md 中在阶段 N 的指令里明确要求 AI 写一个 checkpoint 文件（如 `.cursor/cursor-rule-guard/checkpoints/skill-a-stage2.checkpoint`），guard 对这个 Write 检查：是否已读 B。

```
AI 执行 A 到阶段 N
  → A 指令要求写 checkpoint 文件
  → preToolUse(Write) 触发
  → guard-write.py 查 skill-chains.json：写 "skill-a-stage2.checkpoint" → 要求已读 B
  → B 未读 → deny，用户消息提示必须先加载 B
```

**需要新增的东西很少**：
- `skill-chains.json`：声明 `"checkpoints/skill-a-stage2.checkpoint" → requires: ["skill-b.md"]`
- `audit-read.py` 扩展：追踪 skill 文件的读取（路径如 `~/.cursor/skills/**/*.md`），写入 `skills-state/{cid}/`
- `guard-write.py` 小扩展：同时查 `skill-chains.json` + `skills-state`

#### 路径二：Skill Active Guard（更粗粒度，更简单）

原理：**只要 A 在当前会话被 Read 过，且 B 未读，就阻断所有 Write/Edit/Shell。**

无需 checkpoint 文件，"阶段"的概念被弱化——变成"A 激活后的全局前置"：

```
skill-chains.json: {"skill-a.md": {"requires": "skill-b.md"}}
```

guard 逻辑：若 `skills-state/{cid}/skill-a.md.json` 存在（A 被 Read 过），但 `skills-state/{cid}/skill-b.md.json` 不存在 → deny 所有写操作。

**优点**：实现更简单。**缺点**：粒度是"A 一旦加载就锁定"，而非"到特定阶段才锁定"。如果 A 的前几步不需要 B，这会造成误阻断。

---

### 需要新增的核心组件

| 组件 | 路径一（Checkpoint） | 路径二（Active Guard） |
|------|------|------|
| `audit-read.py` | 扩展：追踪 skill 文件读取 → `skills-state/` | 同左 |
| `guard-write.py` | 扩展：查 `skill-chains.json` + `skills-state/` | 同左，逻辑稍简 |
| `skill-chains.json` | 新增：checkpoint path → required skills | 新增：skill-a → required skills |
| Skill A 的 md | 在阶段 N 指令中加入"写 checkpoint"步骤 | 无需改 |

---

### 我的判断

如果你需要**精确到"某个阶段"**，走路径一，成本可控，且完全复用现有的 deny 机制。

如果"B 是 A 的全局前置条件"（即 A 开始执行就必须有 B），路径二更简洁，改动最小。

两条路径都不需要重构现有架构，只是在现有 `guard-write.py` 的查找链上增加一个新数据源（`skill-chains.json` + `skills-state/`）。

---

你有明确偏向哪条路径吗？或者你想继续讨论"阶段时机"如何在 Skill A 的 md 里表达？

Turn:1～1

Model: Claude Sonnet 4.6

<!-- DDM:TURN_SEP:v1 -->
## User（Turn 2）

你先抛开我给你举例的文档思考，我如下给你再细化描述。

我需要的是A文件里面写：先加载B，然后执行B的流程。或者： 先加载B，然后执行接下来的流程。 这个加载B，是否可以在执行过程中通过rules强制化，也就是检测是否执行，没有执行不允许继续。

<!-- DDM:TURN_SEP:v1 -->
## AI

Turn2

用户意图：理解在 A 文件中声明「先加载 B，再继续」这条指令，hook 系统能否机械强制执行——没加载 B 就阻断后续。

---
## 核心问题：hook 的拦截锚点是什么

先确认一件事：**hook 只能拦截工具调用（tool call），无法拦截 AI 的纯推理/生成文字**。

所以「强制不允许继续」能否成立，取决于「继续」这个动作是否落到至少一个工具调用上。

实际工作流里：
- 继续执行任何实质性操作 → 必然有 Read、Write、Edit 或 Shell 调用
- 纯文字回复（不调用工具）→ 无法拦截

对于绝大多数 SKILL 工作流，「继续」都会产生工具调用，所以**拦截是可行的**。

---
## 技术上如何实现

现有架构缺的只有两件东西：

**① 对 skill 文件读取的追踪**

`audit-read.py` 目前只追踪 `.cursor/cursor-rule-guard/docs/` 下的文件。需要扩展：当 AI Read 任意 `SKILL.md` 或指定路径的 `.md` 文件时，也写入 `skills-state/{cid}/{basename}.json`。

**② A 文件中声明自己的前置依赖**

沿用现有「文档自描述 frontmatter」的设计模式，A.md 头部声明：

```yaml
---
skill-guard:
  requires: ["B.md"]
---
```

`guard-write.py` 扩展逻辑：若当前会话已 Read A（`skills-state/{cid}/A.md.json` 存在），则 B 也必须已被 Read，否则 deny。

---
## 一个你需要确认的边界

上面的方案阻断的是 **Write/Edit/Shell**，这覆盖了大部分「继续执行」的场景。

但如果「继续」的下一步是 **Read 另一个文件**（纯读操作），现有架构默认 allow。你需要决定：

- **只拦截写类操作**（改动最小，覆盖大多数场景）
- **同时拦截 Read**（覆盖更完整，但 `preToolUse(Read)` 需要从 always-allow 改为条件判断，改动更大）

---
## 结论

| 问题 | 答案 |
|------|------|
| 架构能否承接？ | ✅ 可以，现有 deny 机制直接复用 |
| 需要新增什么？ | skill 文件读取追踪 + frontmatter 声明 + guard 扩展查 skill-chains |
| 能 100% 阻断所有「继续」? | 写操作 100%；纯 Read 操作需额外扩展；无工具调用的推理无法拦截 |
| A 文件怎么声明依赖？ | frontmatter 里 `skill-guard.requires: [B.md]`，自描述，不需要外部配置 |

你要继续推进的话，下一步需要你确认一点：拦截范围是只管写操作，还是连 Read 也要管？

Turn:1～2

Model: Claude Sonnet 4.6

<!-- DDM:TURN_SEP:v1 -->
## User（Turn 3）

我不是拦截，而是要求A在执行过程中，在某个分支，必须加载B文件。而不是声明，让AI加载B。但是，我不知道实际是否加载

<!-- DDM:TURN_SEP:v1 -->
## AI

Turn3

现在理解了。你的问题不是"如何声明"，而是**"AI 是否真的执行了 Read B 这个工具调用"**——你不信任 AI 的自我报告，需要外部机械验证。

这正是 cursor-rule-guard 的原始哲学。

---
## 完全可以——而且逻辑完全一致

现有系统对规则文档做的事，就是你想对 B 做的事：

```
不问 AI "你有没有读过 coding-global.md"
而是：Read 工具调用有没有发生过 → hook 独立记录 → guard 独立核查
```

你的场景是同一个机制，目标换成了 skill 文件 B：

```
不问 AI "你有没有执行 Load B"
而是：Read B.md 的工具调用有没有发生过 → hook 独立记录 → guard 独立核查
```

---
## 需要扩展的只有两处

**现在的问题：`audit-read.py` 只追踪 `.cursor/cursor-rule-guard/docs/` 下的文件。**

扩展之后：

1. **`audit-read.py` 扩展**：当 AI Read 任何被声明为"需追踪"的 `.md` 文件（包括 skill 文件），写入 `skills-state/{cid}/B.md.json`

2. **`guard-write.py` 扩展**：检查链里增加一条——若当前会话已读 A，则 `skills-state/{cid}/B.md.json` 必须存在，否则 deny

---
## 这解决了你的核心不确定性

| 情况 | hook 的行为 |
|------|------------|
| AI 真的 Read 了 B | `skills-state/{cid}/B.md.json` 存在 → allow |
| AI 跳过了，没有 Read B | `skills-state/{cid}/B.md.json` 不存在 → deny，提示必须先加载 B |
| AI 声称加载了但没有调用 Read | 同上，deny |

AI 无法伪造这个记录——`skills-state` 只由 hook 脚本写入，不由 AI 控制。

---

唯一需要你再确认的：A 里的「在某个分支加载 B」之后，下一步一定会有 Write/Edit/Shell 吗？如果是的话，这个方案完全闭合，改动极小。

Turn:1～3

Model: Claude Sonnet 4.6

<!-- DDM:TURN_SEP:v1 -->
## User（Turn 4）

澄清一下，你说的：“若当前会话已读 A，则 skills-state/{cid}/B.md.json 必须存在”。实际，并不是 读A的时候， B必须存在。A有很多分支，B1 ；B2；B3等。每个分支对应一个文件读取。

<!-- DDM:TURN_SEP:v1 -->
## AI

Turn4

理解了。问题本质是：**hook 看不见"AI 走了 A 的哪个分支"**，它只能看见"某个工具调用即将发生"。

所以分支 → B 文件这个映射，需要一个 hook 能识别的锚点。

---
## 问题核心

```
A 的分支 1 → 应该加载 B1
A 的分支 2 → 应该加载 B2
A 的分支 3 → 应该加载 B3
```

hook 无法感知"AI 走了哪个分支"，只能感知"AI 要做什么操作"。所以必须找一个**分支和操作的对应关系**。

---
## 两条路径

### 路径一：不同分支写不同类型的文件（现有机制直接可用）

如果每个分支最终操作的文件类型/路径不同：

```
分支 1 → 操作 *.js 文件  → 要求已读 B1.md
分支 2 → 操作 *.py 文件  → 要求已读 B2.md
```

这和现有的 glob 映射完全一致，不需要任何扩展。

**但**：如果不同分支最终都写同类文件（比如都写 `.md`），这条路不通。

---

### 路径二：分支过渡时，写一个 checkpoint 文件作为锚点

这是更通用的方案。A 的 md 在每个分支里加一步：

```
分支 1：
  1. 加载 B1
  2. 写入 checkpoints/A-branch1.done  ← 这是 hook 的拦截点
  3. 继续执行分支 1 的流程
```

guard 的配置：
```
写 A-branch1.done → 必须已读 B1.md
写 A-branch2.done → 必须已读 B2.md
```

**效果**：
- AI 如果跳过了 Load B1，试图写 `A-branch1.done` 时 → deny，提示必须先加载 B1
- AI 加载了 B1 后再写 → allow，分支继续

---
## 本质上，你在描述的是

```
A 的指令本身无法被强制执行
只有 A 的指令产生的工具调用，才能被 hook 拦截
```

所以「强制加载 B」的唯一可靠锚点，是**在 Load B 之后必须发生的某个具体操作**。checkpoint write 就是专门为这个设计的显式锚点。

---

路径二需要在 A.md 的每个分支里加一行"写 checkpoint"的步骤，你可以接受这个约束吗？

Turn:1～4

Model: Claude Sonnet 4.6

<!-- DDM:TURN_SEP:v1 -->
## User（Turn 5）

你说的太狭隘了，我需要的是一个更通用的工作流：A -> B1 / B2 / B3  -> C1 / C2 / C3 ...。这个类似一个状态机。就像你收到prompt，可能有很多中分支路径。例如：先做web搜索，然后去查某个文件。只是你的这个状态机是不断增长的，而且是实时AI预判并生成的。我要的是把这个状态机以一种约束固化下来。

<!-- DDM:TURN_SEP:v1 -->
## AI

Turn5

理解了。你描述的是一个更大的命题：**把 AI 的工作流用状态机约束固化下来，让 hook 层机械执行这个状态机。**

让我先把这个命题的结构说清楚，因为里面有一个设计张力需要先暴露。

---
## 你描述的状态机

```
A → B1 → C1
      ↘ C2
  ↘ B2 → C3
  ↘ B3 → C4
           ↘ C5
```

AI 每一步根据情境选择分支，选定之后被锁定，不能混走路径。

---
## 一个必须解决的设计张力

你说"状态机是实时 AI 预判并生成的"——这里有一个核心矛盾：

| 角色 | 期望 |
|------|------|
| 你 | 状态机是约束，AI 不能绕过它 |
| AI | 状态机是 AI 自己判断决定的 |

如果 AI 既是状态机的生成者，又是被状态机约束的执行者——它可以生成一个"方便自己绕过的"状态机。

**这个矛盾必须被解决，否则系统在机械层面是空的。**

---
## 两种可行的设计方向

### 方向一：状态机预定义（约束是外部的）

状态机由你写死，AI 只能在允许的路径里走。

```json
// workflow-graph.json
{
  "A.md": ["B1.md", "B2.md", "B3.md"],
  "B1.md": ["C1.md", "C2.md"],
  "B2.md": ["C3.md"]
}
```

hook 检查：当前状态（已读序列）→ 下一个操作是否是合法的转移？不是就 deny。

**优点**：机械约束完全可信。  
**代价**：状态机由你来写和维护，AI 只是执行者。

### 方向二：AI 声明 + 锁定（约束是 AI 自己生成但随后外化）

AI 在每一步**先写下它的下一步计划**（提交到 state 文件），hook 锁定这个承诺，之后 AI 无法偏离。

```
AI 决定走 B1 分支
  → 写 workflow-state/{cid}/committed.json: {"next": "B1.md"}
  → hook 锁定：接下来只允许 Read B1.md，其他 Read 被 deny
  → AI 读了 B1 后，再写下 committed.json 的下一步
```

**优点**：路径是 AI 动态生成的，但一旦声明就被外部锁定，AI 无法反悔。  
**代价**：AI 的"第一步声明"本身没有被约束（它可以声明任何路径），但之后的执行被硬约束。

---
## 对现有架构的复用程度

| 组件 | 复用程度 | 说明 |
|------|------|------|
| `audit-read.py` + Read 追踪 | ✅ 直接复用 | 状态转移的原始数据就是 Read 序列 |
| `guard-write.py` 的 deny 机制 | ✅ 直接复用 | 拦截点不变 |
| glob→rules 映射 | ⚠️ 部分复用 | 要扩展为图结构（DAG）而非平铺映射 |
| 新增：状态机引擎 | ❌ 全新组件 | 计算"当前状态 + 合法下一步" |
| 新增：Read 也可以被 deny | ❌ 全新能力 | 当前架构 Read 永远 allow |

---
## 核心结论

cursor-rule-guard 的 hook 基础设施（intercepting tool calls → check state → deny/allow）是对的，可以作为地基。

但你描述的系统是它的**泛化版本**：从"写文件前检查规则是否已读"升级为"任何工具调用前检查当前状态机状态是否允许这一步"。

这是一个新系统，不是一个 patch。

---

在继续之前，你需要先决定：**状态机是预定义的（方向一），还是 AI 动态生成但随后被锁定的（方向二）？** 这个决定决定了整个架构的信任模型。

Turn:1～5

Model: Claude Sonnet 4.6

<!-- DDM:TURN_SEP:v1 -->
## User（Turn 6）

我给你描述一下应用的场景：我需要描述AI修复BUG的过程。这个过程是如下流程（注意：流程是草稿， 用于我们本此工作流设计讨论，不是最终版本）

流程	名称	核心动作
流程零	可复现步骤	确认最小复现路径
流程一	Bug 分析	确认根因（需有证据支撑）
流程二	影响面分析	确认影响范围
流程三	修复方案设计	选定方案并获得授权
流程四	失败测试先行	写出暴露 bug 的测试（当前失败）
流程五	Bug 修复	实现变更，测试变绿
流程六	验证	功能 + 影响面 + 回归，通过后关闭
流程七	防止复现	补测试 / lint 规则

我需要将每个流程拆成一个执行环节，流程零 确认可复现步骤，如果没有确定，就重复执行。查到之后，进入复现流程校验，这个校验有一个标准，也就是一个Spec文档。校验通过了才能进入下一个环节，也就是流程一Bug 分析。

进入 流程一 查出根因，查出根因之后，也有一个流程校验，审查AI给的原因是否有依据，没有依据则提示依据不充分，需要重新查。

然后，流程继续。这里不继续描述。

这个流程环节中，可以嵌套一些人工输入信息、确认等环节。

这是否是一个编程BUG分析修复的工作流？ 你觉得这个工作流，是否类似 LangGraph 实现？

<!-- DDM:TURN_SEP:v1 -->
## AI

Turn6

用户意图：确认这个 bug 修复工作流的定性，以及它和 LangGraph 的异同。

---
## 是的，这是一个标准的 Bug 修复工作流

结构很扎实：复现 → 根因 → 影响面 → 方案 → 测试先行 → 修复 → 验证 → 防复现。每个环节有 Spec 校验 + 失败则回退。这是业界系统化 debugging 流程的完整形态。

---
## 和 LangGraph 非常像——但有一个根本性的差异

**结构上几乎一一对应：**

| 你的设计 | LangGraph 概念 |
|------|------|
| 流程零、流程一… | Node |
| Spec 校验通过 → 进入下一环节 | Conditional Edge |
| 校验失败 → 重复执行 | Cycle（循环节点） |
| 人工输入/确认 | Human-in-the-loop |
| 当前所在哪个流程 + 已收集的证据 | State |

**但有一个根本性的架构差异：**

| 维度 | LangGraph | 你的 Cursor 场景 |
|------|------|------|
| 谁控制流程？ | 外部图引擎，LLM 是被调用的工具 | LLM（Cursor Agent）是主体，自主决策 |
| 约束从哪来？ | 图结构本身就是约束，LLM 无法绕过 | 需要从外部（hook）给自主 LLM 加约束 |
| 状态机在哪？ | 在编排代码里 | 需要你自己设计并让 hook 执行 |

LangGraph 的强约束来自于**你掌控执行权**：你写代码决定什么时候调用 LLM，LLM 只是图里的一个节点。

你在 Cursor 里面对的是**反过来的**：LLM 自主运行，你只能通过 hook 在侧面插入检查和拦截。

---
## 这意味着什么

你的目标（把这个 bug 修复流程用约束固化下来）有两条路：

**路 A：用 LangGraph / 类似框架自建**  
自己写 Python/TS 编排代码，图引擎控制整个流程，Cursor 或其他 LLM API 是图里的执行单元。**状态机是代码层的硬约束，AI 无法绕过。** 代价：脱离 Cursor 的原生 Agent 体验，工程量大。

**路 B：在 Cursor 原生 Agent 上用 hook 约束**  
延续 cursor-rule-guard 的思路。SKILL.md 定义流程，hook 检查每个环节的完成状态。**AI 是主体，hook 是守卫。** 约束强度低于路 A，但完全原生于 Cursor，工程量小。

**两条路的核心区别只有一个：你是要控制执行权，还是要对自主 AI 施加约束？**

---

你现在是想先把这个流程在 Cursor 原生方案里跑起来，还是考虑做一个独立的编排层？

Turn:1～6

Model: Claude Sonnet 4.6

<!-- DDM:TURN_SEP:v1 -->
## User（Turn 7）

你给我调查一下，类似的工作流，业界都是怎么做的？

<!-- DDM:TURN_SEP:v1 -->
## AI

Turn7

用户意图：调查业界在 AI Agent 工作流编排和状态机约束方面的主流做法。

---
## 业界现状：这是一个 2025-2026 年快速成熟的领域

### 主流框架分布

| 框架 | 定位 | 控制模型 |
|------|------|------|
| **LangGraph** | 图状态机，最强可控性 | 外部编排，LLM 是节点 |
| **CrewAI** | 角色分工，快速上手 | 外部编排，多 agent 协作 |
| **AutoGen** | 对话优先，多 agent 协商 | 对话驱动，弱结构 |

LangGraph 在 2025 年 10 月发布 v1.0，已成为**复杂分支 + 合规审计场景的业界标准**。你的 Bug 修复工作流（阶段校验 + 失败回退 + 人工确认）和它的设计几乎一一对应。

---

### 2026 年学界的直接对应研究

有两篇论文与你描述的场景高度吻合：

**TDD Governance for Multi-Agent Code Generation（2026，EASE 会议）**

这篇论文做的事，就是你想做的事——把 TDD 流程变成 AI 不可绕过的执行约束。核心架构原则是：

> "Language models never directly write to the file system. Instead, they return structured patch proposals that are subject to **validation gates** prior to application."
> 
> "The **orchestration engine** retains sole authority over workspace mutation."

它把 Red→Green→Refactor 变成了强制的 **phase ordering**：没有 failing test（RED），不允许写实现代码（GREEN）。这和你的"流程零没完成，不能进流程一"完全同构。

**TDAD（Test-Driven Agentic Development，2026）**

实验数据：在 SWE-bench 上，加了结构化约束后，regression 从 6.08% 降到 1.82%，issue 解决率从 24% 提升到 32%。但同时发现：**直接加 TDD 提示词不加结构约束，regression 反而上升到 9.94%**——比没有任何约束还差。说明"告诉 AI 要走流程"远不如"机械强制 AI 必须走流程"。

---

### 业界对 Cursor 原生场景的做法

2026 年两个代表性实现：

- **Oasis + Cursor**：用 `beforeMCPExecution` hook 做 intent-based access control，每个工具调用前向中央 policy API 查权限，生成完整审计链
- **Agent Control**：同样走 hook，用外部评估 API 做 governance

这是你的 cursor-rule-guard 架构的同类实现，只不过把"规则是否已读"换成了"这个 tool call 是否符合当前 policy"。

**业界把这类系统叫做 Harness（线束）。** 2026 年 Meta 以 20 亿美元收购 Manus，买的不是模型，买的就是这个 harness。

---

### 你的方案在业界版图里的位置

```
外部编排（LangGraph）          Harness 约束（cursor-rule-guard）
      │                                    │
 你控制执行权                        AI 自主执行
 LLM 是工具                         Hook 是守卫
 状态机是代码                        状态机是配置+脚本
 约束最强                            约束较弱但原生
      │                                    │
      └──────────── 你的 Bug 修复流程 ──────┘
```

两条路业界都有真实实现。核心区别只有一个：**你愿不愿意脱离 Cursor 原生 Agent，自己掌控执行权。**

---
## 结论

你描述的 Bug 修复工作流是业界 2026 年的前沿命题，有大量真实工程实践和学术研究支撑。LangGraph 是最对标的参照系。cursor-rule-guard 的 hook 架构是轻量版的同类方案，适合在 Cursor 原生环境里低成本落地，代价是约束强度弱于外部编排。

Turn:1～7

Model: Claude Sonnet 4.6

<!-- DDM:TURN_SEP:v1 -->
## User（Turn 8）

业界讨论的 多Agent协作，属于上述哪一块呢？

<!-- DDM:TURN_SEP:v1 -->
## AI

Turn8

用户意图：理解多 Agent 协作在上述框架版图中的位置。

---
## 多 Agent 协作是正交维度，不属于哪一块

你之前我们讨论的两个轴是：

> **谁控制执行权？**（外部编排 vs. Harness 约束）

多 Agent 协作回答的是另一个问题：

> **工作由谁来做？**（单个 AI vs. 多个专职 AI）

这两个维度是正交的，可以任意组合：

```
                    单 Agent
                       │
  Harness 约束 ─────────┼───────── 外部编排
(cursor-rule-guard)    │         (LangGraph)
                       │
                    多 Agent
```

---
## 多 Agent 解决的是分工问题

| 概念 | 解决的问题 | 典型问题 |
|------|------|------|
| 工作流状态机 | **什么时候** 做，**按什么顺序** | 流程零没完成不允许进流程一 |
| 多 Agent | **谁** 来做，**角色怎么分** | 专职 agent 分析根因，另一个写测试 |

你的 Bug 修复流程里，可以是：
- **单 Agent 走完全部 8 个流程**（一个通才 AI）
- **8 个专职 Agent，每人负责一个流程**（分工协作）

状态机约束独立于这个选择存在。

---
## 生产系统通常是两者叠加

```
外部编排（LangGraph）
    │
    ├── 状态机：流程零 → 流程一 → 流程二 → ...
    │            ↑ 校验门控，失败回退
    │
    └── 每个节点内部：一个或多个专职 Agent
            ├── ReproAgent: 复现路径
            ├── AnalysisAgent: 根因分析
            └── FixAgent: 代码修复
```

LangGraph 的 Multi-agent 支持就是这个架构——图结构管流程顺序，每个节点是一个专职 agent。

---
## 对你的场景的意义

你的 Bug 修复工作流目前可以做单 Agent（Cursor 原生）。如果将来某个流程的上下文太长、需要隔离（比如流程四写失败测试时需要全新上下文），才需要引入多 Agent 拆分。这是优化手段，不是必须的起点。

Turn:1～8

Model: Claude Sonnet 4.6

<!-- DDM:TURN_SEP:v1 -->
## User（Turn 9）

多Agent的手段介入，是因为需要给不同的流程，不同的上下文，给任务设定不同的思维/工作模式，完成不同类型的任务吗？

<!-- DDM:TURN_SEP:v1 -->
## AI

Turn9

用户意图：理解多 Agent 的本质驱动力——是否是为了给不同任务提供不同的上下文和工作模式。

---
## 是的，这是核心驱动力之一，但不是全部

你的表述抓住了最重要的那个原因。拆分成三个层次来说：

---

### 驱动力一：上下文隔离（你说的）

单个 agent 跑完整个 Bug 修复流程，到流程五的时候，上下文里已经堆满了流程零到四的分析内容。这些内容对"写修复代码"这个动作来说大部分是噪音。

专职 agent 的好处是：

```
ReproAgent 的上下文:  [复现工具] + [系统信息] + [复现步骤]
FixAgent 的上下文:    [根因结论] + [代码库] + [失败测试]
```

每个 agent 拿到的是对自己任务恰好合适的信息密度。

---

### 驱动力二：工作模式/人格（你说的）

不同流程需要不同的 system prompt / 思维框架：

| 流程 | 所需模式 |
|------|------|
| 根因分析 | 怀疑一切，证据驱动，不下结论 |
| 修复实现 | 最小化改动，不扩散 |
| 影响面分析 | 全局视角，主动发现 |
| 代码 review | 挑剔，找反例 |

单 agent 一个 system prompt 无法同时持有这些互相冲突的态度。

---

### 驱动力三：并行（这个你没提）

不同的工作如果没有依赖关系，可以同时跑。比如流程二（影响面分析）确认范围后，可以同时启动多个 FixAgent 分别处理不同模块，再由 ReviewAgent 汇总。时间压缩。

---

### 驱动力四：专业工具配置

不同的 agent 可以挂不同的工具集：

```
ReproAgent:  [shell 执行权] + [日志读取]
AnalysisAgent: [代码搜索] + [call graph 分析]
FixAgent:    [代码写入权] + [测试运行]
```

最小权限原则——每个 agent 只拿做这件事需要的工具，降低意外副作用。

---
## 一句话总结

多 Agent 本质上是把**上下文、态度、工具、权限**四件事，按任务分开配置，让每个 agent 在最适合自己任务的状态下工作。你直觉抓到的是前两个，后两个是工程层面的额外收益。

Turn:1～9

Model: Claude Sonnet 4.6

<!-- DDM:TURN_SEP:v1 -->
## User（Turn 10）

那如果基于Cursor这样的IDE去实现类似的工作流，业界目前的方案有哪些？方案的大众化程度分别怎么样？

<!-- DDM:TURN_SEP:v1 -->
## AI

Turn10

用户意图：调查在 Cursor 这类 IDE 原生环境里实现结构化工作流的业界方案及其普及程度。

The user wants to know what industry solutions exist for implementing workflow-like systems within Cursor (or similar IDEs), and how popular/widespread each approach is.

现在有足够的调查结果了。

---
## Cursor 原生环境下的工作流方案全景

### 方案一：SKILL.md + Subagent（官方原生，最主流）

**做法**：把每个流程写成一个 SKILL.md 文件。父 Agent 执行主流程，关键阶段通过 `Task` 工具派发子 Agent，每个子 Agent 有独立上下文。

```
主 Agent 读 bug-fix-workflow.md
  → 派发 ReproAgent（子 Agent，独立上下文）
  → 接收结果，进入下一阶段
  → 派发 AnalysisAgent
  → ...
```

**普及程度**：⭐⭐⭐⭐⭐ — Cursor 官方推荐路径，2025 年 Cursor 2.4 加入原生 Subagent 支持后成为主流做法。几乎所有认真使用 Cursor 做工程的团队都在用 SKILL.md。

**工作流约束强度**：弱。SKILL.md 是指令，不是约束——AI 可以选择不按顺序走。

---

### 方案二：cursor-prompt-queue（社区工具，顺序编排）

**做法**：用队列文件描述多步顺序工作流，每一步在一个全新子 Agent 上下文里执行，步骤间通过 `{{step_N.key}}` 传递变量，有 state 文件支持跨会话恢复。

```yaml
# queue.yaml
steps:
  - prompt: "复现 bug，输出最小复现步骤"
    outputs: [repro_steps]
  - prompt: "基于 {{step_1.repro_steps}} 分析根因"
    outputs: [root_cause]
```

**普及程度**：⭐⭐ — GitHub 上存在但受众较小，主要解决"上下文腐烂"问题，不解决约束强制问题。

**工作流约束强度**：中。顺序是编排决定的，但 AI 在每步内部没有约束。

---

### 方案三：hooks.json 守卫（官方支持，企业为主）

**做法**：cursor-rule-guard 就属于这一类。用 `preToolUse` / `beforeMCPExecution` 等 hook 在工具调用层插入检查脚本，返回 allow/deny。

生态里的代表：Oasis、Stacklok、MintMCP、Agent Control——都是走这条路，主要面向企业安全合规场景（敏感数据扫描、MCP 权限控制）。

**普及程度**：⭐⭐⭐ 企业场景，个人开发者用得少。cursor-rule-guard 是目前极少数针对**工作流合规**而非安全防护的 hook 实现。

**工作流约束强度**：强。唯一能机械阻断工具调用的方案。

---

### 方案四：MCP Server 作为工作流引擎（新兴）

**做法**：把工作流状态机做成一个 MCP Server，AI 通过调用 MCP 工具"申请进入下一阶段"，MCP Server 内部做状态检查，不合格就返回错误。

```
AI 调用 mcp.advance_phase("phase_1", evidence={...})
  → MCP Server 校验 evidence 是否符合 Spec
  → 符合 → 返回 phase_2 的指令
  → 不符合 → 返回错误，AI 必须重新做
```

**普及程度**：⭐⭐ — 思路在 2025-2026 年开始出现在社区讨论中，但成熟实现很少，大多数 MCP 用途还是工具访问（搜索、数据库、Linear），不是流程编排。

**工作流约束强度**：强。且天然支持状态机逻辑，是目前**最有潜力**的方向。

---
## 对比总结

| 方案 | 普及度 | 约束强度 | 工程成本 | 适合场景 |
|------|------|------|------|------|
| SKILL.md + Subagent | ⭐⭐⭐⭐⭐ | 弱（指令） | 低 | 大多数日常工作流 |
| cursor-prompt-queue | ⭐⭐ | 中（顺序编排） | 低 | 顺序多步、上下文隔离 |
| hooks.json 守卫 | ⭐⭐⭐ | 强（机械阻断） | 中 | 合规审计、流程门控 |
| MCP 工作流引擎 | ⭐⭐ | 强（状态机） | 高 | 复杂分支、校验门控 |

你描述的 Bug 修复工作流（有 Spec 校验、失败回退、人工确认）——如果要在 Cursor 原生环境里做，**hooks.json + SKILL.md 的组合是目前可行性最高的路径，MCP 工作流引擎是长期更优解但成本更高**。

Turn:1～10

Model: Claude Sonnet 4.6

<!-- DDM:TURN_SEP:v1 -->
## User（Turn 11）

如果不采用Cursor的原生实现，而是采用LangGraph，是不是类似GitHub的方案，例如发起PR之后的执行某个工作流？

<!-- DDM:TURN_SEP:v1 -->
## AI

Turn11

用户意图：理解 LangGraph 方案和 GitHub Actions 类工作流的异同。

---
## 类比成立，但有一个关键差异

GitHub Actions 的心智模型是对的——**外部编排，有状态，阶段触发**。但有一处根本不同：

| 维度 | GitHub Actions | LangGraph Bug 修复 |
|------|------|------|
| **触发时机** | git 事件之后（post-hoc） | 开发过程中（in-process） |
| **执行者** | bash 脚本、Docker 容器 | LLM Agent |
| **人工介入** | 主要无人工，偶有 approval | 可以每个阶段都有人工审查 |
| **输出** | CI 日志、artifacts | 代码变更、分析报告 |

GitHub Actions 是代码写完之后运行检查。LangGraph 的 Bug 修复工作流是**代替人写代码**的过程本身。

---
## 更准确的类比

如果非要找一个相似的东西，LangGraph 更像：

```
GitHub Actions（结构）
  + Devin / SWE-agent（执行主体是 LLM）
  + Linear 工单流（状态 + 人工审批节点）
```

---
## 实际部署形态

如果你用 LangGraph 实现你的 Bug 修复工作流，架构长这样：

```
用户提交 Bug 报告
    ↓
LangGraph 应用（独立运行，不在 IDE 里）
    ├── Node 0: ReproAgent → 输出复现步骤
    ├── Gate 0: 人工确认复现路径 ← 你在浏览器/API 里审批
    ├── Node 1: AnalysisAgent → 输出根因
    ├── Gate 1: Spec 校验（自动）→ 不通过则循环
    ├── Node 2: ...
    └── 最终：PR 提交到 GitHub
```

**Cursor 已经不在主链路里了**。LangGraph 直接调 LLM API（Claude / GPT），自己管文件系统。Cursor 如果出现，只是 LangGraph 调用的一个工具（通过 `@cursor/sdk`），不是主体。

---
## 这意味着你要做一个选择

| | Cursor 原生（SKILL + Hook） | LangGraph 外部编排 |
|---|---|---|
| 开发体验 | 在 IDE 里，实时看到 AI 工作 | 脱离 IDE，运行独立应用 |
| 工作流约束 | 弱（Hook 辅助） | 强（图结构硬约束） |
| 人工介入 | 自然（就是在 Cursor 里对话） | 需要单独做审批 UI 或 CLI |
| 工程门槛 | 低 | 高 |
| 适合场景 | 开发者用工具辅助自己 | 自动化流水线，低人工介入 |

你的 Bug 修复流程里有"人工输入/确认"环节——这在 Cursor 原生里天然成立（就是对话），在 LangGraph 里需要额外设计 human-in-the-loop 的交互方式。

---

**所以：** GitHub Actions 的类比适合描述 LangGraph 的结构，但它的使用场景更接近"把 Bug 修复自动化成一条流水线"而不是"在 IDE 里辅助开发者修 Bug"。两个目标选哪个，你需要先确定。

Turn:1～11

Model: Claude Sonnet 4.6

<!-- DDM:TURN_SEP:v1 -->
## User（Turn 12）

如果使用 LangGraph，更像是线上BUG自动分析，你觉得呢？ LangGraph 适合部署服务端。 Cursor的方案，适合本地开发过程中的BUG分析修复，依赖本地开发、测试、设备环境。

<!-- DDM:TURN_SEP:v1 -->
## AI

Turn12

完全准确。你刚才完成了一个非常清晰的场景切分：

---
## 你的判断是对的

| | LangGraph | Cursor 原生 |
|---|---|---|
| **部署位置** | 服务端，持续运行 | 本地开发机 |
| **触发方式** | 监控系统报警、用户提 Issue | 开发者在 IDE 里主动发起 |
| **依赖环境** | 可访问线上日志、APM、DB | 本地代码库、本地测试、本地 runtime |
| **典型场景** | 线上 P0 告警 → 自动定位 → 创建修复 PR | 开发者遇到 bug → 在 IDE 里修复 |
| **人工介入** | 少，或事后审批 | 全程参与，实时确认 |

---
## 两者可以是同一个流程的两段

```
线上告警触发
    ↓
LangGraph（服务端）
  → 自动复现
  → 自动根因分析
  → 生成修复方案草稿
  → 创建 Issue + 上下文文档
    ↓
开发者打开 Cursor
  → 读取 Issue 和上下文
  → 用 Cursor 原生工作流（SKILL + Hook）本地修复
  → 跑本地测试验证
  → 提 PR
    ↓
LangGraph（服务端）再次接管
  → CI 验证 → 合并 → 验证修复有效
```

---

**这是目前 AI-native 工程团队正在探索的完整闭环**：服务端自动化（LangGraph）负责感知和分析，本地 IDE 工作流（Cursor）负责需要人工判断和本地环境的修复执行，两者各守自己的边界。

你的 Bug 修复工作流里，流程零到流程二（复现、根因、影响面）可能既适合服务端跑，也适合本地跑，取决于日志和复现环境在哪里。流程三以后（方案设计、写测试、修复、验证）几乎必须在本地。

Turn:1～12

Model: Claude Sonnet 4.6

<!-- DDM:TURN_SEP:v1 -->
## User（Turn 13）

那 以后测试也可以自动化；测试出的问题，交给服务端BUG分析自动化 LangGraph；然后，简单的问题，甚至可以自动写代码，提交PR；AI解决不了的问题，可以再任何阶段停止，丢给研发。这是目前，最新的研发过程方向吗？

<!-- DDM:TURN_SEP:v1 -->
## AI

Turn13

是的，你描述的正是 2025-2026 年业界正在落地的方向，而且每个环节都有真实实现。

---
## 你描述的闭环，业界叫什么

**Autonomous Software Engineering** 或 **AI-native SDLC**（Software Development Lifecycle）。

完整的闭环是：

```
测试 / 监控 自动触发
    ↓
AI 自动分析（LangGraph 服务端）
    ↓
简单问题 → AI 自动修复 → 自动提 PR → CI 通过 → 自动合并
复杂问题 → 生成上下文 + 诊断报告 → 分配给开发者
    ↓
开发者在 Cursor 本地处理 → 提 PR
    ↓
AI 做 Code Review → 合并
```

---
## 每个环节的真实实现现状

| 环节 | 成熟度 | 代表实现 |
|------|------|------|
| 测试自动化触发 | ✅ 成熟 | GitHub Actions + CI 流水线 |
| AI 自动分析 Bug | ✅ 落地中 | Sentry AI、Datadog Watchdog、Linear AI |
| AI 自动修复 + 提 PR | ✅ 有案例 | Devin、SWE-agent、GitHub Copilot Autofix |
| 复杂问题转人工 | ✅ 成熟 | 任何 human-in-the-loop 系统 |
| AI 做 Code Review | ✅ 落地中 | GitHub Copilot Review、CodeRabbit |
| 本地 IDE 工作流约束 | ⚠️ 早期 | cursor-rule-guard（你的场景）|

---
## 一个真实的数据点

SWE-bench Verified（模拟真实 GitHub Issues 的修复基准）：

- 2024 年初：最好的 AI 解决率 < 5%
- 2025 年底：Claude + SWE-agent 达到 ~50%+
- 2026 年预测：简单 bug 的自动修复率 > 70%

**所以"简单问题 AI 自动修复"在 2026 年已经不是假设，是正在量产的能力。**

---
## 这个方向的本质是什么

你描述的闭环，核心是把研发过程里的**人工判断节点最小化**：

```
之前：每个环节都需要人来触发和决策
现在：AI 先跑，遇到置信度低 / 超出能力边界 / 需要授权 → 才停下来问人
```

这和你设计的 Bug 修复工作流完全同构——你的"校验通过才能进下一阶段"就是在定义**AI 什么时候可以自主继续，什么时候必须停下来**。这是整个方向的核心设计问题。

---

**是的，这是目前最新的研发过程方向，而且正在从"技术验证"进入"工程落地"阶段。** 你思考的问题和业界前沿完全对齐。

Turn:1～13

Model: Claude Sonnet 4.6

<!-- DDM:TURN_SEP:v1 -->
## User（Turn 14）

按照你说的，核心流程都在服务端做了， 本地 IDE 工作流约束，是否不是一个主流方向了？

<!-- DDM:TURN_SEP:v1 -->
## AI

Turn14

好问题。但结论不是"不主流"——而是**两个方向服务不同的人群和场景**。

---
## 服务端自动化 ≠ 本地 IDE 工作流消失

服务端能自动化的，是**已知类型的、可复现的、有清晰输入输出的**问题。

开发者在本地 IDE 里处理的，是：

- 需要深度理解业务上下文的复杂 bug
- 架构决策、权衡取舍
- 全新功能开发
- 服务端 AI 解决不了、转交给人的那部分

**服务端自动化的天花板，就是本地 IDE 工作流的地板。**

---
## 更准确的判断

| 场景 | 趋势 |
|------|------|
| 简单 bug、已知模式 | 服务端自动化正在吞噬，本地 IDE 介入越来越少 |
| 复杂 bug、业务耦合深 | 仍然在本地 IDE，而且 AI 辅助需求更高 |
| 新功能开发 | 完全在本地，这是 Cursor 的主战场 |
| 架构重构 | 本地，需要人做判断 |

服务端自动化是在**减少开发者要处理的总量**，不是在取代开发者处理复杂问题的方式。

---
## 本地 IDE 工作流约束的真实价值

你设计的工作流约束（cursor-rule-guard、phase gating），核心价值不是"让 AI 跑通流程"，而是：

> **让 AI 在处理复杂问题时，不跳步、不省略证据、不绕过验证。**

这在服务端全自动场景里同样适用——LangGraph 的 validation gate 就是同一回事。

区别只是：服务端用代码写死约束，本地 IDE 用 hook + SKILL 来约束自主 AI。

---
## 所以

本地 IDE 工作流约束不是"过时的方向"，而是：

- 目前**比服务端自动化更早能落地**（不需要搭独立服务）
- 长期是**服务端自动化处理不了的复杂问题的补充**
- 你在探索的，恰好是业界还没有成熟方案的那个空白区

Turn:1～14

Model: Claude Sonnet 4.6

<!-- DDM:TURN_SEP:v1 -->
## User（Turn 15）

确实对于一些复杂问题，AI目前看还无法解决，例如：涉及到的多系统，端到端的BUG。可能需要手机上操作某个流程触发，服务端都还在DEV环境。 不过，未来会不会直接有一套仿真环境，解决这个环境问题。

<!-- DDM:TURN_SEP:v1 -->
## AI

Turn15

你描述的这个问题，业界已经有人在做了。

---
## 仿真环境的方向：确实存在，而且在快速发展

### 已有的实现

**端到端测试仿真**（相对成熟）
- Maestro、Detox：移动端 UI 自动化，可以模拟手机操作流程
- Playwright、Cypress：Web 端 E2E，可以模拟浏览器行为
- 但这些还是**脚本驱动**，不是 AI 理解并自主触发

**AI 驱动的移动端仿真**（2025-2026 新兴）
- Apple Intelligence 的 on-device agent、Android AI Core：AI 可以理解 UI 语义并操作
- Browser Use、Computer Use（Anthropic）：AI 直接操控浏览器/桌面，不需要写脚本
- 这些能力已经可以"理解一个用户操作流程并复现它"

**全链路沙箱**（还在早期）
- 你描述的"手机操作 → 触发 → 服务端 DEV 环境"这个完整链路的仿真，目前还没有成熟的开箱即用方案
- 但各大云厂商（AWS、Google）都在做 AI-driven integration testing 环境

---
## 这个方向的核心难题

不是技术不够，而是**环境状态的确定性**：

```
仿真环境难以解决的：
  - 用户账号的历史状态（数据差异）
  - 第三方依赖的异常行为
  - 时序问题（race condition）
  - 设备型号 / OS 版本的差异
```

这些问题在真实 bug 里占很大比例，仿真环境在这里的覆盖率天然有限。

---
## 我的判断

**短期（1-2 年）**：AI 能自主操作手机/浏览器复现 bug 的能力会成熟，但"全链路沙箱"还是工程重点，不是通用方案。

**中期（3-5 年）**：AI 的 E2E 仿真能力会覆盖 60-70% 的常见路径，真正的长尾复杂 bug（多系统、状态依赖、时序）仍需人工介入。

**长期**：如果 AI 能维护一个持续同步的数字孪生（生产环境的实时镜像），这个问题从根本上解决。这已经有人在做（Digital Twin + AI），但距离 coding workflow 的实用还有距离。

---

你描述的这类 bug（手机 → 服务端联调），**目前仍然是 AI 的硬边界**，也是人类开发者短期内最难被替代的场景。

Turn:1～15

Model: Claude Sonnet 4.6

<!-- DDM:TURN_SEP:v1 -->
## User（Turn 16）

相对于服务端开发，流程、日志都是清晰的。之需要Mock掉和外部系统 or 终端设备的交互，就可以完全AI设计测试了？那这么看，是否手机移动端开发，被完全自动化的可能更低呢？

<!-- DDM:TURN_SEP:v1 -->
## AI

Turn16

你的推断方向对，但结论要分层。

---
## 服务端确实更容易被自动化，原因就是你说的

服务端的天然优势：

```
输入/输出 → 明确（HTTP 请求、数据库状态、日志）
依赖 → 可 Mock（外部 API、消息队列、第三方服务）
状态 → 可快照（DB dump、seed data）
复现 → 确定性（同样的输入 → 同样的输出）
测试 → 可完全自动化（单测、集成测试、合约测试）
```

Mock 掉外部依赖之后，服务端 bug 的复现和验证是**封闭系统**，AI 可以完全在这个封闭系统里工作。

---
## 移动端的额外复杂度

| 维度 | 服务端 | 移动端 |
|------|------|------|
| 输入来源 | 网络请求（结构化） | 手势、传感器、相机、NFC（物理世界） |
| UI 状态 | 无 | 屏幕渲染、动画、布局（像素级） |
| 设备碎片化 | 无 | iOS / Android 版本 × 设备型号 × 屏幕尺寸 |
| 系统权限 | 无 | 通知、定位、蓝牙（用户授权流程） |
| 测试执行 | CI 服务器直接跑 | 需要真机或模拟器 |

这些不是"Mock 一下就解决"的问题——UI 渲染 bug、手势交互 bug、特定设备上的兼容问题，Mock 根本触达不到。

---
## 但结论要分层，不是"移动端更难被自动化"

| 移动端子领域 | 被自动化难度 |
|------|------|
| 业务逻辑层（ViewModel、数据处理） | 低，和服务端一样容易 |
| 网络层（API 调用、错误处理） | 低，可 Mock |
| UI 渲染、动画、布局 | 高，视觉验证难 |
| 手势、物理交互 | 高，仿真器和真机有差距 |
| 设备兼容性 | 极高，组合爆炸 |

**移动端 = 服务端逻辑 + UI 层 + 物理世界接口**

前两层的自动化程度和服务端一样，第三层是移动端特有的硬问题。

---
## 你的判断成立，但要精确一点

> 移动端业务逻辑和网络层，被自动化的程度和服务端一样高。  
> 移动端 UI 交互、设备兼容性，被完全自动化的可能性比服务端低得多。  
> 这部分恰好是移动端 bug 里最难复现、最难 Mock 的那类。

所以移动端开发者的不可替代性，集中在**UI 感知、设备调试、用户体验判断**这几件事上——这些都需要"眼睛和手"，AI 目前没有。

Turn:1～16

Model: Claude Sonnet 4.6

<!-- DDM:TURN_SEP:v1 -->
## User（Turn 17）

我们回到 本地 IDE 工作流约束，Turn10你说的： hooks.json + SKILL.md 的组合是目前可行性最高的路径。 可以解释一下大概思路吗？

<!-- DDM:TURN_SEP:v1 -->
## AI

Turn17

用户意图：理解 hooks.json + SKILL.md 组合实现本地 IDE 工作流约束的具体思路。

---
## 两个组件各自的角色

**SKILL.md = 工作流的定义层**

告诉 AI "应该怎么走"——流程顺序、每步做什么、完成条件、失败怎么处理。是指令，AI 会尽量遵守，但没有强制力。

**hooks.json = 工作流的执行守卫层**

在 AI 调用工具的时候插入检查，返回 allow 或 deny。是机械约束，AI 无法绕过。

---
## 组合思路

两者分工互补：

```
SKILL.md          →  定义"应该做什么"
hooks.json        →  强制"没做就不能继续"
```

以你的 Bug 修复流程为例：

```
开发者触发 systematic-debugging 技能
    ↓
AI 读取 phase0-repro.md（SKILL 指令）
    → 确认复现步骤
    → 写入 .workflow-state/{cid}/phase0.done（checkpoint）
         ↑
         hook 检查：写这个文件前，复现步骤文档必须已经 Read
         没读过 → deny，提示"先执行复现流程"
    ↓
AI 读取 phase1-analysis.md（SKILL 指令）
    → 分析根因
    → 写入 phase1.done
         ↑
         hook 检查：phase0.done 必须存在
         不存在 → deny，提示"必须先完成流程零"
    ↓
继续...
```

---
## checkpoint 文件是关键设计

每个阶段完成后写一个 checkpoint 文件，这是 hook 能感知到"当前处于哪个阶段"的唯一机械手段：

```
.workflow-state/{conversation_id}/
  phase0.done      ← 复现步骤确认完成
  phase1.done      ← 根因分析完成
  phase2.done      ← 影响面分析完成
  ...
```

guard 脚本在每次写操作前检查：

- 写 `phase1.done` → 前置要求 `phase0.done` 存在
- 写任何代码文件 → 前置要求 `phase3.done`（方案已获授权）存在

---
## 对你最初问题的回答

这就是"A 文件执行到某个阶段，必须加载 B 文件"在本地 IDE 里的完整落地形态：

- SKILL A 的指令：第二阶段开始前，必须读取 `phase1-spec.md`
- Hook：在写 `phase1.done` 这个 checkpoint 之前，检查 `phase1-spec.md` 是否已被 Read

**SKILL.md 定义意图，checkpoint write 是触发点，hook 是机械执行者。** 三者合一，构成可审计、可阻断的工作流约束。

Turn:1～17

Model: Claude Sonnet 4.6

<!-- DDM:TURN_SEP:v1 -->
## User（Turn 18）

怎么确保 AI 是按照 SKILL.md 的意图（流程）执行呢？  hook只是规定了必须做完A，才能做B；checkpoint write规定了做完了A要写入done。

<!-- DDM:TURN_SEP:v1 -->
## AI

Turn18

这是整个设计里最核心的问题，也是最诚实的地方。

---
## 先给一个直接的答案

**你不能完全确保。**

hook 能保证的是：
- AI 有没有 Read 过某个文件（行为可审计）
- checkpoint 文件有没有写入（动作可拦截）

hook 保证不了的是：
- AI 在流程零里有没有真的找到复现路径
- 还是只是随便写了一些文字就写入了 `phase0.done`

---
## 但可以把"作弊的成本"提高到高于"认真做的成本"

有几层机制可以叠加：

**第一层：输出结构化，hook 做 schema 验证**

checkpoint 不是空文件，而是包含结构化输出：

```json
// phase0.done
{
  "repro_steps": ["step1...", "step2..."],
  "min_repro_case": "...",
  "evidence": {"log_file": "...", "line": 42}
}
```

hook 脚本在写入前检查：required 字段是否存在，是否非空。AI 要绕过，必须构造一个符合 schema 的假输出——这比认真做还费劲。

**第二层：Read 审计作为前置条件**

这就是 cursor-rule-guard 的核心——写 `phase0.done` 之前，必须已经 Read 过 `phase0-spec.md`（明确定义"复现步骤"的标准是什么）。没读过 spec，连写入都不被允许。

**第三层：关键节点人工确认**

对于流程三（方案授权）这类高风险节点，hook 返回 `ask`，暂停并要求人工显式确认：

```json
{"permission": "ask", "user_message": "流程三完成，准备进入代码修改阶段，请确认方案"}
```

人工看了 AI 的输出之后，手动允许继续。

---
## 这和 cursor-rule-guard 的哲学是一致的

cursor-rule-guard 的设计文档里有一句话：

> 本系统只检测加载，不检测合规。合规验证由其他层负责。

同样的分层在这里适用：

| 层 | 能做什么 | 不能做什么 |
|------|------|------|
| hook + checkpoint | 检测步骤是否执行、结构是否完整 | 判断内容是否正确 |
| schema 验证 | 检测输出字段是否存在 | 判断分析是否准确 |
| 人工审查 | 判断 AI 的结论是否可信 | 不可能完全自动化 |
| spec 文档 | 定义"完成"的标准 | 强制 AI 达到这个标准 |

---
## 最诚实的表述

这套机制的目标不是**证明 AI 做了高质量的工作**，而是：

1. **防止 AI 无意识跳步**（最常见的失败模式）
2. **留下可审计的证据链**（出了问题能回溯）
3. **在关键节点强制人工介入**（把质量判断还给人）

和 TDD 的哲学一样——测试不保证代码正确，但让写正确代码比写错误代码更容易。

Turn:1～18

Model: Claude Sonnet 4.6

<!-- DDM:TURN_SEP:v1 -->
## User（Turn 19）

那退一步讲，如果我只使用SKILL实现呢？ 不做任何 checkpoint 和 hook， 这个BUG分析流程程度的执行链条，会有什么问题吗？

<!-- DDM:TURN_SEP:v1 -->
## AI

Turn19

用户意图：评估纯 SKILL.md 方案（无 hook、无 checkpoint）的实际效果和问题。

---
## 纯 SKILL.md 能跑通，但有三个真实风险

### 风险一：AI 会在压力下跳步

SKILL.md 是指令，AI 在执行时有一个内在偏向——**推进任务**。

具体表现：
- 流程零没找到清晰复现路径，但 AI 觉得"差不多了"，直接进流程一
- 流程一的根因"感觉合理"但没有代码证据，AI 仍然继续
- 用户说了一句"快点"，AI 把后面几步合并执行

这不是 AI 故意绕过，是它的注意力分配机制在长上下文里天然退化。**SKILL.md 越长，越后面的指令越容易被稀释。**

---

### 风险二：没有状态记录，恢复困难

纯 SKILL.md 的所有流程状态都在对话上下文里。

- 中途退出对话，再开新 Chat——状态全丢，必须从头开始
- 上下文太长被截断——之前的结论可能已经被 AI 遗忘
- 多个 Bug 同时处理——没有隔离，容易混淆

没有 checkpoint 文件，就没有持久化的"走到哪里了"。

---

### 风险三：无法审计，出了问题无法溯源

如果 AI 修复了 bug 但修复有问题，你无法回答：
- 它有没有真正做过影响面分析？
- 方案是否经过了授权步骤？
- 哪一步的输出质量有问题？

纯 SKILL.md 留不下可机械查询的证据链。

---
## 但纯 SKILL.md 也有真实价值

| 维度 | 纯 SKILL.md |
|------|------|
| 上手成本 | 极低，写一个 md 文件就行 |
| 对简单 bug 的效果 | 好，流程短，AI 跳步概率低 |
| 对复杂 bug 的效果 | 一般，步骤多时 AI 容易漂移 |
| 团队推广难度 | 低，不依赖任何额外工具 |

---
## 结论

纯 SKILL.md 方案**能跑，值得先做**。

它的问题不是"跑不通"，而是**可靠性随 bug 复杂度下降**——简单 bug 大概率顺畅，复杂 bug 的执行质量不稳定，出了问题难以溯源。

**实用建议**：先用纯 SKILL.md 跑几个真实 bug，观察 AI 在哪些步骤容易漂移，再针对这些步骤补 checkpoint + hook。不需要一开始就做完整的约束体系。

Turn:1～19

Model: Claude Sonnet 4.6

<!-- DDM:TURN_SEP:v1 -->
## User（Turn 20）

那再退一步讲，我直接对话式分析BUG呢？最大的问题是什么？

<!-- DDM:TURN_SEP:v1 -->
## AI

Turn20

用户意图：评估最原始的"直接对话分析 bug"方式的核心问题。

---
## 最大的问题只有一个

**没有流程纪律，AI 会跳到它觉得"最可能正确"的结论，跳过中间的验证步骤。**

---
## 具体表现

你描述一个 bug，AI 通常会：

1. 快速给出一个"最可能的根因"假设
2. 直接建议修复方案
3. 如果你接受了，开始改代码

它**跳过**的是：
- 有没有最小复现路径？
- 根因假设有没有代码证据支撑？
- 修复方案有没有评估影响面？
- 改动有没有对应的失败测试先行？

这些步骤在对话里不会自动发生，除非你主动问。

---
## 为什么这是问题

AI 给出的第一个假设，大约有 40-60% 的概率是错的（取决于 bug 复杂度）。

在对话模式里：
- 你可能接受了错误的根因 → 修了一个不相关的地方
- 修复了表面现象 → 根因还在，bug 复现
- 影响面没评估 → 修复引入了新问题

这不是 AI 能力的问题，是**对话模式天然没有"先证伪，再结论"的约束**。

---
## 对比三种方式

| 方式 | 跳步风险 | 结论可信度 | 出问题可溯源 |
|------|------|------|------|
| 直接对话 | 高 | 低 | 无 |
| 纯 SKILL.md | 中 | 中 | 弱 |
| SKILL + checkpoint + hook | 低 | 高 | 有 |

---

对话模式适合**探索和澄清**（"这个 bug 大概是什么方向"），不适合**严肃的修复执行**（"我要确保根因正确再动代码"）。

两者的区别就是：有没有一个流程在约束"先证明，再行动"。

Turn:1～20

Model: Claude Sonnet 4.6

<!-- DDM:TURN_SEP:v1 -->
## User（Turn 21）

执行：dtd_distill_overview，不写文件。

<!-- DDM:TURN_SEP:v1 -->
## AI

用户未提供 raw 文件路径，直接从当前对话生成。模式：只输出（不写文件）。

---

# AI 工作流约束与 Bug 修复流程设计

> 创建时间：2026年5月15日 19:12

---

**cursor-rule-guard 扩展与 Skill 依赖约束**｜Turn 1–5

- Turn 1：提问现有 cursor-rule-guard 架构能否承接"A 执行中必须加载 B"的场景，分析两条扩展路径（Checkpoint Write / Skill Active Guard）
- Turn 2：澄清需求——不是声明加载，而是机械验证 B 是否真正被 Read
- Turn 3：确认 hook 的独立审计轨迹（skills-state）是解决"不知道是否真加载"的正确机制
- Turn 4：进一步澄清——A 有多个分支（B1/B2/B3），不同分支对应不同文件，guard 需要能识别当前分支
- Turn 5：命题扩大为通用状态机工作流 A→B→C，要求机械固化执行路径；暴露"AI 动态生成分支"与"外部约束"的设计张力

---

**Bug 修复工作流定性与业界框架对比**｜Turn 6–8

- Turn 6：以8阶段 Bug 修复流程为具体场景，确认其与 LangGraph 状态机的结构对应关系，揭示"外部编排 vs Harness 约束"的根本差异
- Turn 7：调查业界 AI 工作流框架现状——LangGraph/CrewAI/AutoGen 三足鼎立，以及 2026 年 TDD 治理论文和 Cursor hook 生态实践
- Turn 8：厘清多 Agent 协作在框架版图中的位置——它是正交维度（WHO 做），不是编排维度（WHEN/HOW）

---

**多 Agent 的本质驱动**｜Turn 9

- Turn 9：多 Agent 的核心价值是上下文隔离、工作模式切换、并行加速、最小权限工具配置——四个驱动力各有侧重

---

**Cursor 原生工作流方案全景**｜Turn 10–12

- Turn 10：梳理 Cursor 原生环境下4种工作流方案（SKILL+Subagent / cursor-prompt-queue / hooks守卫 / MCP工作流引擎）及各自普及程度
- Turn 11：LangGraph 类比 GitHub Actions 成立，但关键差异在于时机（post-hoc vs in-process）和执行主体（脚本 vs LLM）
- Turn 12：确立场景切分——LangGraph 适合服务端自动化，Cursor 适合本地开发修复，两者可衔接为完整闭环

---

**AI 辅助研发流程的未来方向**｜Turn 13–16

- Turn 13：描述完整闭环愿景——测试自动触发→AI 分析→自动提 PR→复杂问题转人工；确认这是业界 2026 年正在落地的方向
- Turn 14：辨析服务端自动化不会消灭本地 IDE 工作流约束——两者服务不同问题域，本地约束是业界空白区
- Turn 15：探讨仿真环境解决多系统端到端 Bug 的可能性——短期能力正在成熟，长期依赖数字孪生，当前仍是 AI 硬边界
- Turn 16：服务端 vs 移动端被自动化程度对比——移动端业务逻辑层同样容易自动化，UI/手势/设备碎片化是移动端特有硬问题

---

**hooks.json + SKILL.md 工作流约束机制**｜Turn 17–20

- Turn 17：具体拆解 SKILL.md（定义意图）+ checkpoint write（触发点）+ hook（机械执行）三者如何协同，形成可审计工作流
- Turn 18：正面回答"如何确保 AI 按意图执行"——诚实承认无法完全机械保证，叠加 schema 验证 + 人工审查 + Read 审计提高可靠性下限
- Turn 19：评估纯 SKILL.md 方案（无 hook/checkpoint）——能跑通，但跳步风险随复杂度上升，建议先跑再针对性补约束
- Turn 20：评估直接对话分析 Bug——最大问题是无流程纪律，AI 会跳到"最可能正确的结论"跳过中间验证步骤

---

Turn:1～20

Model: Claude Sonnet 4.6

<!-- DDM:TURN_SEP:v1 -->
## User（Turn 22）

执行：dtd_normalize ， 主题： BUG分析工作流：发散式探讨

<!-- DDM:TURN_SEP:v1 -->
