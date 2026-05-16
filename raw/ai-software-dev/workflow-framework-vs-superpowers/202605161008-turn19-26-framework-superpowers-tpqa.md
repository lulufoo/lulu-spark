# ai-dev-workflow-framework：对比 Superpowers

> 创建时间：2026年5月16日 10:08

> 主题：【ai-dev-workflow-framework：对比 Superpowers】（归档范围 Turn 19～26；来源会话 transcript `3659af6a-f5db-4275-b31e-21c011d883fe`）

> 导航：[distilled](../../../distilled/ai-software-dev/workflow-framework-vs-superpowers/202605161008-turn19-26-framework-superpowers-tpqa.md) · [digest](../../../digest/ai-software-dev/workflow-framework-vs-superpowers/202605161008-turn19-26-framework-superpowers-tpqa.md) · [trace](../../../trace/ai-software-dev/workflow-framework-vs-superpowers/202605161008-turn19-26-framework-superpowers-tpqa.md)

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 19）

你觉得当前的 技术实现计划评估框架（TPQA），是否可以做成 工作流 ？

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 19）

Turn19

**用户意图：** 判断 **TPQA（技术实现计划评估框架）** 是否适合做成可重复的 **工作流**（而不仅是静态检查表）。

**可以，而且文档本身已经偏「流程」而不是纯概念。** ✅ 依据来自你已引用的框架结构：四层按顺序检查、十维清单、修复优先级表、以及与 PDQA / SPCA 的先后顺序（规格 → 计划 → TPQA → 编码）。这些天然对应「阶段 + 门禁 + 可重复执行」。

**适合做成工作流的含义（建议这样界定）：**

| 形态 | 说明 |
|------|------|
| **人 / Agent 工作流** | 最贴合：对每份 Plan 按层 1→4 过表，输出「通过 / 风险 / GAP」，再按优先级表安排修补顺序。可直接嵌进 PR 模板、计划评审会或 Cursor 里「计划 → 审一遍 → 再写代码」的仪式。 |
| **半自动工作流** | 能自动化的主要是 **机械规则**：例如 Plan 里出现 `TBD`/`TODO`、未完成占位、禁止词等可由脚本或 CI 扫 Markdown；**依赖图是否 DAG** 若计划用结构化 YAML 表达也可部分校验。 |
| **全自动「质量门禁」** | **不适合全自动化**：维度 1（目标是否产出导向）、5（粒度）、6 中「是否需想象」等，需要读者判断，除非你愿意把 Plan 强制写成高度结构化 DSL（成本高、灵活性降）。 |

**若要做成「工作流」，建议显式固定的只有三件事：**
1）**触发时机**：计划定稿后、写实现代码前（与原文「计划写完后，编码前」一致）。
2）**产出物**：每次 TPQA 留一份**简短记录**（维度结论 + GAP 编号即可），避免只「在脑子里过一遍」。
3）**与 SPCA 的关系**：规格 ↔ 计划对齐（SPCA）和 TPQA 可串联为同一 MR/checklist 里的两步，而不是两套互不关联的流程。

**结论：** TPQA **很适合**做成「可重复的人机协作工作流」；若期待的是 **无人参与的流水线门禁**，只能覆盖**一部分**维度，剩余必须保留人工或 LLM 评审环节。

Turn:19～19

本回复由 **Auto**（Cursor 中的 agent 路由器/助手接口）驱动，底层为 **GPT-5.2**。

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 20）

superpower 提供的也是开发工作流吗？ 和 目前的 规格 → 计划 → TPQA → 编码，是否有重叠？

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 20）

Turn20

**用户意图：** 弄清 Superpowers 算不算「开发工作流」，以及它和 **规格 → 计划 → TPQA → 编码** 这条线有没有重叠。

**简短结论：** Superpowers 更像是 **一组可编排的开发行为技能（技能库 + 触发顺序）**，不是单独一份叫「Superpowers 工作流」的固定文档；它和 **规格/计划/TPQA** 在「先想清楚再写代码、计划要可执行」上 **有明显重叠**，但 **覆盖面和目的** 并不相同。

---

### Superpowers 提供的是什么？✅ 已对照 `writing-plans` / `using-superpowers`  skill 摘要

- **不是**一条写死在仓库里的「PDQA→SPCA→TPQA」式三框架，而是多个 **Skill**（如 `brainstorming`、`writing-plans`、`test-driven-development`、`executing-plans`、`verification-before-completion` 等）。
- `using-superpowers` 里用流程图表达的是 **先判断是否该加载技能、创意类是否先 brainstorming**，属于 **代理行为纪律**，不是产品规格流水线。
- `writing-plans` 明确要求：**在有 spec/requirements 的前提下**，把实现拆成带文件路径、测试、小步提交的计划——这和「写计划」阶段 **高度同构**。

所以更准确的说法是：**Superpowers = 开发侧的行为与工作方式规范**；若你愿意，可以把它 **编排成你们团队自己的内部工作流**，但它本身 **不等于** ai-software-dev 里的 **PDQA/SPCA/TPQA 文档质量体系**。

---

### 和「规格 → 计划 → TPQA → 编码」的重叠关系

| 维度 | ai-software-dev 链 | Superpowers |
|------|-------------------|-------------|
| **焦点** | **文档质量闸门**：规格好不好、计划能不能独立执行（PDQA / SPCA / TPQA） | **代理怎么干活**：先脑暴、怎么写计划、TDD、复审、验收证据等 |
| **计划** | TPQA 评估计划 **可执行性**（十维） | `writing-plans` 教你怎么 **写出** 那份计划（结构、Task、文件边界） |
| **编码前** | 强调「计划过 TPQA 再编码」 | 强调「有计划 / 有测试步骤再动代码」等技能组合 |

**重叠部分（实质类似）：**

- 都要求 **先有计划、再实现**；计划里要有 **任务、文件、验收/测试** 一类信息。
- `writing-plans` 产出的内容，本身就是 **TPQA 里最容易打高分的材料**（尤其是依赖、文件边界、步骤可执行）。

**不重叠 / 互补部分：**

- **PDQA / SPCA / TPQA**：对付的是 **规格与计划文档** 的质量与对齐，有 **显式审核表**。
- **Superpowers**：对付的是 **全过程行为**——例如 brainstorming、debugging、完成前 verification，这些在 **三框架示意图里不会单独成框**，但很影响落地质量。

---

### 怎么放在一起用（避免重复劳动）

- 不必二选一：可以约定 **Superpowers 负责「怎么写计划、怎么执行」**，**TPQA 负责「计划写成后过一遍审计表」** —— 前者偏 **生成与纪律**，后者偏 **审计与门禁**。
- 若发现 **写计划** 和 **TPQA 审计划** 内容重复，可以把 TPQA 收成 **轻量 checklist**（只盯十维里你们最容易翻车的那几维），其余交给 `writing-plans` 的结构自带覆盖。

---

**事实标注：** Superpowers 技能正文基于当前工作区 [`writing-plans/SKILL.md`](/Users/lulu/.cursor/skills/superpowers/skills/writing-plans/SKILL.md)、[`using-superpowers/SKILL.md`](/Users/lulu/.cursor/skills/superpowers/skills/using-superpowers/SKILL.md) 的阅读；三框架顺序表述与 **你之前用的 ai-software-dev `21-tpqa` / 概述中的三框架** 一致（此前会话内 `gh api` 已读过）。

Turn:20～20

本回复由 **Auto**（Cursor 中的 agent 路由器/助手接口）驱动，底层为 **GPT-5.2**。

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 21）

还是不太理解，我研发的工作过程，应该使用 Superpowers 驱动吗？ 他和 https://github.com/lulufoo/ai-software-dev/blob/main/ai-dev-workflow-framework/00-overview.md  过程怎么协作呢 ？

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 21）

Turn21

**用户意图：** 弄明白日常研发到底要不要「以 Superpowers 为驱动」，以及它和 [lulufoo/ai-software-dev `00-overview.md`](https://github.com/lulufoo/ai-software-dev/blob/main/ai-dev-workflow-framework/00-overview.md) 里那条总流程 **怎么一起用**。

**✅ 出处：** 上文对总览图与表格的复述，均来自该文件（经 `gh api` 拉取的当前内容）。

---

### 1. 要不要「只用 Superpowers 驱动」？

**不用二选一。** `00-overview` **已经把你关心的两条线写进同一张总览图里了**：

| 阶段 | 总览里写的「谁驱动」 |
|------|----------------------|
| 阶段 1 探索与设计 | **Superpowers：`brainstorming`** + AI IDE **Plan Mode** |
| 阶段 2 实现计划 | **Superpowers：`writing-plans`** + Plan Mode → 用户确认后再执行 |
| 阶段 3 施工单（可选） | **`writing-plans`（Task 级）** + Plan Mode |
| 阶段 4 执行 | **`subagent-driven-development` / `executing-plans`** + **Agent Mode** |
| 阶段 5 验证与审查 | **`verification-before-completion` / `requesting-code-review`** + Agent Mode |

所以：**在「用 AI 做研发」这件事上，Superpowers 本来就是总览里认定的执行侧习惯法**（什么时候脑暴、什么时候写计划、怎么用子代理、怎么收工前验证）。
**但** 单靠 Superpowers，并不会自动帮你完成 **Spec / Plan 之后该做哪几种评审、顺序是什么**——那是 **同一套流程里另一根轴：评估与交叉检测框架**。

---

### 2. Superpowers 和 `ai-dev-workflow-framework` 各自管什么？

用小比喻：

- **Superpowers**：教 **AI（和跟着 AI 的你）怎么动**——在哪个模式、用哪个 skill、计划怎么拆、编码怎么执行、何时要证据再收工。
- **`ai-dev-workflow-framework`（总览 + PDQA/TPQA/SPCA…）**：规定 **路上要经过哪些站、每站产出什么文档、文档质量用什么表审一遍**。

总览里的 **审查触发顺序** 写明的是 **文档侧**（例如 Plan 做完 → SPCA → TPQA），这和 Superpowers **并行不冲突**：一边是 **产出与审计 Plan 文本**，一边是 **写 Plan 时用 `writing-plans`、执行时用 `executing-plans`、收尾用 `verification-before-completion`**。

---

### 3. 和你说的「规格 → 计划 → TPQA → 编码」怎么对齐？

总览比这句话 **多几格**，但 **你关心的核心**可以这么叠上去：

```
用户需求
  → （阶段1）脑暴 + Spec
  → 【PDQA】审 Spec
  → （阶段2）writing-plans 产出 Plan（对齐 20-plan-template）
  → 【SPCA】Spec↔Plan 【TPQA】Plan 可执行性
  → （可选阶段3）复杂 Task → 施工单 + PWCA/WOQA
  → （阶段4）executing-plans / subagent 开发 → 代码
  → （阶段5）verification + code review
```

所以：
- **「规格 → 计划 → …」** ≈ 总览里的 **阶段 + 文档 + 评审**。
- **「TPQA」** 只是 **Plan 定稿后的一道门**；**编码** 在总览里是 **阶段 4**，前面还会强调用 **Superpowers 的执行类 skill**。

---

### 4. 建议你心里记一句「协作方式」

**研发过程 = `00-overview` 的阶段与评审（轨道） + 各阶段里标明的 Superpowers skill（驾驶方式）。**
不是要「选 Superpowers **还是** ai-dev-workflow」，而是 **官方总览已经设计成一起用**：框架图里 **每个阶段都并排写了 Superpowers 和 IDE 模式**，中间再插入 **PDQA / SPCA / TPQA** 等。

你若团队化落地，可以收成一张自己的检查表：**到了哪一阶段 → 开哪个 skill → 产出哪份文档 → 跑哪两个评估**——把总览图缩成你团队的一张 A4 就够。

Turn:21～21

本回复由 **Auto**（Cursor 中的 agent 路由器/助手接口）驱动，底层为 **GPT-5.2**。

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 22）

那 https://github.com/lulufoo/ai-software-dev/blob/main/ai-dev-workflow-framework/00-overview.md  可以拓展支持  Superpowers skill的能力吗？ 完全替代 Superpowers 

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 22）

Turn22

**用户意图：** `00-overview.md` 能否**扩展**成同时覆盖 Superpowers skill 的能力，并**完全替代** Superpowers。

**依据：** 当前 [`00-overview.md`](https://github.com/lulufoo/ai-software-dev/blob/main/ai-dev-workflow-framework/00-overview.md) 已在各阶段**点名**若干 Superpowers skill（如 `brainstorming`、`writing-plans`、`subagent-driven-development` / `executing-plans`、`verification-before-completion`、`requesting-code-review`）—这是「文档里引用技能名」，不是替代技能本体。

---

### 1. 「拓展支持 Superpowers skill」——可以，而且已经是半只脚踏进去了

可做的扩展无非是更「可执行」、更可维护，例如：

- 每个阶段：**Skill 全名 / 仓库路径 / 何时 Read**（你仓库里写清，和 Cursor/Claude Code 的加载方式对齐）。
- **与模板绑定**：阶段 2 ↔ `20-plan-template.md`，阶段 3 ↔ `30-work-order-template.md`，阶段 4 ↔ `executing-plans` 的勾选习惯。
- **质量门**：在总览图里把 PDQA/SPCA/TPQA 与「必须 invoke 的 skill」画成一条时间线（你现在已有审查顺序，可增强为「进入阶段 4 前 checklist」）。

这些扩展都仍是在 **ai-software-dev 仓库里维护「流程与规范文档」**，完全合理。

---

### 2. 「完全替代 Superpowers」——不能在同一件事物上成立

要分清两层：

| 层面 | `00-overview`（+ 整本 framework） | Superpowers（ obra 那套） |
|------|-------------------------------------|---------------------------|
| **是什么** | Markdown：**说明流程、模板、评审维度** | **技能包**：多份 `SKILL.md`，由 IDE/Agent **加载并执行**具体步骤、清单、纪律 |
| **能否被替代** | 不能替代「运行时行为」 | 文档不能「执行」自己 |

所以：

- **不能把「一份 overview」当成 Superpowers 的完全替代物**——除非你所谓的「替代」只是指「我们团队不再引用 obra 仓库」，那也还需要 **别的东西在运行时提供 skill 内容**（例如你们自维护的 skills、或内嵌在规则/AGENTS.md 里的长指令），否则「阶段 4 用 executing-plans」只是一句空话，终端里并没有那份 SKILL 的步骤与约束。

- **若在你们仓库里複製/改写 SKILL 同款内容**：那是 **分叉/自建技能库**，从组织上可减少对 obra/Superpowers 上游的依赖，但那是 **换一个技能来源**，不是「overview 代替了 Superpowers」；维护责任也在你们自己身上。

---

### 3. 推荐你怎么想「协作边界」

- **`ai-dev-workflow-framework`**：宜作为 **你家研发流程的单页真相（含评审门、模板、触发顺序）**，可尽情扩展映射到「该读哪个 SKILL」。
- **Superpowers（或等价物）**：是 **运行时如何约束 Agent 行为** 的载体；要么继续用 obra，要么用自己 repo 里的 skills **实现同一套意图**。

一句话：**overview 可以再扩展成「Superpowers（或等价 skills）的安装与使用说明书」；但要完全丢掉 Superpowers 这类 skill 运行时，必须用另一套等价机制补上，单凭 overview 扩写无法独立完成。**

Turn:22～22

本回复由 **Auto**（Cursor 中的 agent 路由器/助手接口）驱动，底层为 **GPT-5.2**。

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 23）

我的意思是，如果有一套 和  ai-dev-workflow-framework 无缝匹配的 类似Superpowers定位的skill，是否更合适？

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 23）

Turn23

**用户意图：** 判断是否存在一套 **定位类似 Superpowers、但与 `ai-dev-workflow-framework` 无缝对齐** 的 skill——是否 **更合适**。

**结论：** 对你这种 **已经把 `ai-dev-workflow-framework` 当作研发主流程真相** 的用法来说，一套 **为该框架量身定制、阶段与模板一一对应的 skill**，通常 **更合适**——不是因为它在抽象上优于 Superpowers，而是因为 **耦合成本更低、叙事一致、可被「流程 + 运行时」一体化治理**。

---

### 为什么说「更合适」（适用条件）

**当且仅当**你满足下面之一：

- 团队默认走 **Spec → PDQA → Plan → SPCA/TPQA →（可选 WO）→ 执行 → WOEF → 收尾评审**（与 `00-overview` 一致）；
- 你希望 **开一个阶段 = 开一个 skill**，而不是在执行时自行「从 Superpowers 里挑技能名再对齐总览」。

在这种前提下，「框架原生 skill」带来的好处很明确：

| 维度 | 与 overview 匹配的专用 skill |
|------|-------------------------------|
| **心智模型** | 阶段编号、门禁顺序、模板文件名可以和 skill 触发语 / 文档交叉引用完全一致 |
| **产出物契约** | 每个 skill 的「完成定义」可直接对应 PDQA/SPCA/TPQA 的检查对象 |
| **维护** | 流程改版时**同时改**框架 MD + 同名 skill，避免 obra 上游与业务框架两套节奏 |
| **落地** | 新人一句「按 ai-dev workflow 走」就够，不必再解释 Superpowers 子集 |

---

### Superpowers 仍有什么价值？

- **通用性强**：不写死在某一家的 Spec/Plan/施工单措辞上；社区会持续迭代。
- **你已在 `00-overview` 里借了名字**：本质是「行为纪律」可被任何 skill 实现——**更合适的是对齐，而非否定 Superpowers 这类形态**。

所以更准确的说法不是「Superpowers 不好」，而是：
**以 `ai-dev-workflow-framework` 为轴心时，「一套与它 1:1 映射的技能包」比「套用通用 obra 全集再人工对齐」更省力。**

---

### 「无缝匹配」一般指什么（避免空喊）

真要「无缝」，通常要约定清楚例如：

- 每个 **阶段 / 门禁** 是否 **强制** invoke 某一个 skill（或等价：Read 某一 SKILL.md）；
- skill 的 **产出路径**是否与模板一致（如 `docs/...`、`20-plan-template` 小节）；
- **PDQA/TPQA 等表格**是否与 skill 的 checklist **同构**（避免两套重复又不一致）。

否则只是「换个名字」，没有减少摩擦。

---

**一句话：** 若你的工作流主轴就是 `ai-dev-workflow-framework`，**做一套与它同构的 skill（或精简后的「框架版 Superpowers」）通常更合适**；Superpowers **仍是合理参照物**，可以完全由你们自维护技能内容来替代 obra 的实现，只要把 **运行时执行**留在 skill 层，而不是只扩写 Markdown。

Turn:23～23

本回复由 **Auto**（Cursor 中的 agent 路由器/助手接口）驱动，底层为 **GPT-5.2**。

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 24）

我说一个具体的例子，你思考一下我们上面讨论的内容，更有体感。

在 Plan执行过程中，我讨论完了，需要丢一个文档：20-plan-template.md，说按照这个模板整理。但是，实际上Plan讨论的过程，就应该按照这个模板执行，最后产出就是符合模板的设计。而不是最后说，按照模板整理，但是实际上，可能模板里面讨论的内容，并没有涉及，最后仓促的生成一个看似完备的文档。

这个Case，你感觉目前的 Superpowers 如何更好的无缝兼容呢？

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 24）

Turn24

**用户体感里的问题：** 讨论像在「散装聊天」，快结束时才来一句「按 `20-plan-template.md` 整理一下」——结果文档**长得像模板**，但很多块是**事后补窟窿**，没在讨论里真正被推演过。你要的是：**讨论过程和模板骨架同轨**，收口时只是**落定文字**，而不是**第一次才把模板当人看**。

下面按「Superpowers 怎么和它更贴合」来想，不涉及改你仓库的具体文件。

---

### 1. 问题本质：`writing-plans` 管的是「计划长什么样」，不自动等于「你的 20 号模板」

Superpowers 的 `writing-plans` 很会要求：**文件边界、Task、小步步骤、勾选框**——但它**不认识**你们在 `ai-software-dev` 里那份 **`20-plan-template.md` 的章节顺序与门禁表**。
所以会自然滑向你说的形态：**先有对话 → 再在事后把对话塞进模板**，除非人在流程上强制「从一开始就按章节推进」。

**要无缝，必须把「模板 = 讨论的骨架」，写进可被 Agent 强制执行的习惯里**，而不是只靠一句口头「最后整理」。

---

### 2. 更值得做的「贴合方式」（从省事到最重）

**A. 轻量、立刻有用（几乎不用Fork Superpowers）**

- **规则 / 会话约定**：只要一进入「做 Plan」，**第一段输出必须是**：从 `20-plan-template.md` **复制全套二级标题**，占满文档骨架；后面所有回复**只允许往对应标题下填空**，不允许长时间「无归属闲聊」。
- **门禁一句话**：没到「模板里每一块都有至少一条**讨论里出现过的论据/决策**」，就不算 Plan 结案（PDQA/TPQA 里「无占位」「验收可检验」也能打这条的后门）。

这已经是在用 **overview 的阶段 2 精神**对齐讨论过程，不一定要改 obra。

**B. 中等：`writing-plans` 的用法「绑死」到你的模板**

在团队自己的 **SKILL.md 包装一层**（或 AGENTS/Cursor Rule）写明：

> 使用 `writing-plans`，但 **Markdown 结构与 `20-plan-template.md` 逐项对齐**（不是 Superpowers 默认 header 一套就完）；每完成一节讨论，**当场写入**一节，不要等到「最后再整理」。

这样 **skill 仍然存在**，但你把 **输出的 schema** 从「通用 Superpowers 计划」**收紧成「20 号模板副本」**，事后「仓促套壳」的概率会明显下降。

**C. 最重、但和你前面说的「专属 skill」一致**

做 **`ai-dev-planning`（名字随意）**：
**触发即**打开模板壳 → **逐项用 checklist 挡住**「尚未在本会话中出现的章节」——和 `brainstorming` 的 HARD-GATE 同构：没填满关键块就不宣布设计通过。这叫 **真正把模板嵌进运行时**，而不是事后整形。

---

### 3. 和 TPQA/SPCA 的「体感闭环」

你抱怨的现象，正好是 **TPQA 维度里「占位 / 先有想象再填空」的反例**。若讨论过程始终对着 **模板同一套标题**，事后 TPQA **扫的不是「突然冒出来的工整文档」，而是「讨论轨迹是否真的能支撑每一节」**——体感和质量都会好很多。

---

### 4. 一句落地建议（针对你这个 Case）

**不要让「丢一个 20-plan-template」发生在讨论结束之后；要让「打开的空模板（或 SKILL 驱动的同构骨架）」出现在讨论第一句。**
Superpowers **本身可以做到「很会写 Plan」**，**无缝兼容 `20-plan-template` 靠的是**：要么 **规则强制「先骨架后叙述」**，要么 **薄包装 `writing-plans` + 固定章节映射**，要么 **专用 skill + 未完成章节不让过**——三选一就够改善你这个具体痛点。

Turn:24～24

本回复由 **Auto**（Cursor 中的 agent 路由器/助手接口）驱动，底层为 **GPT-5.2**。

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 25）

不仅如此，21-tpqa-plan-evaluation-framework.md 执行的时候，我希望按照上述执行的时候流程，审查生成报告。对报告逐项问答。然后升级plan。然后，再次执行 21-tpqa-plan-evaluation-framework.md 。直到plan的质量通过。 这个过程，其实需要一个流程约束去执行，而不是通过对话驱动。

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 25）

Turn25

**用户意图：** 把 **TPQA（`21-tpqa-plan-evaluation-framework.md`）** 从「聊天里顺便审一下」改成 **有流程约束的闭环**：先出**审查报告** → **对报告逐项确认/处置** → **升级 Plan** → **再跑 TPQA** → 直到通过；并且强调要 **流程驱动**，不是 **对话驱动**。

---

### 1. 你说的闭环，本质是什么

可以压成一条**有终止条件的状态机**：

```text
Plan_v0
  → [TPQA 执行] → 审查报告（十维 × 判定 + GAP 列表）
  → [逐项处置]   → 每条 GAP：采纳修复 / 接受风险 / 暂缓（需记录理由）
  → [升级 Plan]  → Plan_v(n+1)（只改与处置决议相关的部分）
  → [TPQA 复跑]  → 若仍有未关闭 GAP → 循环；若十维均达门槛 → 结束
```

这和「对话驱动」的差别在于：**每一步的输入/输出是规定好的产物**（报告、决议表、Plan 版本号），而不是「想到哪问到哪」。

---

### 2. 为什么「需要流程约束」——你已经在点出 Superpowers 的缺口

- **Superpowers** 擅长 **写计划、拆 Task、执行、验证**，但**没有**把你描述的 **「TPQA 报告 → 逐项签核 → 升版 Plan → 再 TPQA」** 做成**强制状态机**。
- **`21-tpqa` 本身是评估框架（读什么、判什么）**，**不是**「如何组织多轮评审会议」的运行手册——**循环与门禁**需要你们**在外层补一层「流程协议」**（或 skill / 工具）。

所以你的判断对：**要稳定质量，需要流程约束**，不能只依赖「这次对话里 agent 比较乖」。

---

### 3. 「不是对话驱动」在工程上通常长什么样

任选其一到混合使用即可，核心是 **固定产物 + 固定顺序 + 禁止跳步**：

| 形态 | 作用 |
|------|------|
| **状态文件** | 如 `plan-tpqa-state.yaml`：`round`, `plan_version`, `dimensions[]`, `gaps[]`, `resolutions[]`；未更新状态文件不得声明「TPQA 通过」。 |
| **审查报告模板** | 每轮 TPQA **必须**产出同结构 Markdown（十维表 + GAP 编号 + 证据引用 Plan 章节），禁止只口头说「过了」。 |
| **逐项签核表** | GAP 表增加列：`处置 / 责任人 / 对应 Plan 修改段 / 是否需再跑 TPQA`。每一项未 CLOSED 则 **不允许** merge「Plan 定稿」标签。 |
| **自动化辅助** | 脚本检查：Plan 里是否还有 `TBD`、是否缺 `## Task`、是否引用已删除章节等（只扫**机械项**，主观维仍人审）。 |
| **专用 Skill（你前面说的框架向 skill）** | 把上述步骤写进 `SKILL.md`：**Step 1 只许出报告，Step 2 只许过签核表，Step 3 只许改 Plan，Step 4 只许复跑 TPQA**——对话只是填表，不是绕流程。 |

「不是对话驱动」的意思，落地就是：**对话只能操作规定表格/状态**，不能替代表格。

---

### 4. 和你前面「讨论就要按 20 号模板」怎么拼在一起

- **阶段 2（写 Plan）**：讨论过程 **锚定 `20-plan-template.md` 骨架**（先壳后填），减少「假完备」。
- **TPQA 轮**：对 **Plan 的当前版本** 出报告，**逐项签核** 改 Plan。
- **再 TPQA**：对 **新版本** 再跑，直到 **十维 + 你们自定义的「必须零 GAP」政策** 满足。

这样 **模板负责「长对」**，**TPQA 循环负责「审对、改到对」**，二者都是**流程产物**，不是聊天氛围。

---

### 5. 一句结论

你的设想（**报告 → 逐项问答 → 升 Plan → 再 TPQA → 直到通过**）是 **TPQA 框架的自然延伸**，但需要 **外层流程协议（状态 + 模板 + 禁跳步）** 或 **专用 skill/工具** 来约束；**单靠 `21-tpqa`  markdown 本体 + 普通对话**，很容易退化成「形式上审过」。**Superpowers 要无缝兼容**：要么 **wrap 一层「ai-dev TPQA gate」skill** 固化上述步骤，要么 **在工作流引擎/PR 门禁里强制执行状态文件**——核心是 **运行机制**，不是再多聊几轮。

若你下一步要做的，是把这套东西 **写进 `ai-software-dev` 的一个新文档（如 `21b-tpqa-execution-protocol.md`）或一个 Cursor skill**，可以说明你希望 **纯文档**还是 **要带机器可读状态**，我可以按你仓库习惯拆一稿结构目录（仍不替你擅自改远端仓库 unless 你明确要求）。

Turn:25～25

本回复由 **Auto**（Cursor 中的 agent 路由器/助手接口）驱动，底层为 **GPT-5.2**。

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 26）

那你倾向 还是基于 Superpowers 去支持这个流程吗？

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 26）

Turn26

**用户意图：** 问我在「TPQA 报告 → 逐项处置 → 升版 Plan → 再跑 TPQA」这条流程里，是否**应该以 Superpowers 为底座**来做支持。

---

**倾向：** **把 Superpowers 当「执行纪律层」**，把 **TPQA 闭环自己当一层「门禁/编排协议」**；**不要为了「统一在 Superpowers 里」而硬塞**。两者是 **组合关系**，Superpowers **不能单独替代**你那套「多轮迭代 + 逐项签核 + 终止条件」，但它能 **接住**其中的若干步。

拆开说：

| 层级 | Superpowers 是否天然覆盖 | 说明 |
|------|-------------------------|------|
| 单次：写计划 / 拆解 / Debug / 「完成前要验证」 | 较贴切 | Skills 本来就是**按阶段约束行为**的工具。 |
| 多轮：报告 → Q&A → 改 Plan → 再 TPQA 直到过关 | ⚠️ 不自动 | 这是 **有状态编排**（版本、GAP 闭环、放行条件）；需要 **自建协议 + 产物模板**（或专用 skill/workflow），不能只指望「开一个 Superpowers conversation」就等价。 |

**因此我的倾向：**
- **若要「流程约束而非聊天驱动」**：主心骨应是 **可复制产物**（报告模板 + 逐项签核表 + `plan`/状态版本号 + 放行规则），写入 **文档或 Cursor skill**。
- **Superpowers**：继续用在 **verification-before-completion、debugging、写计划方法论**等与现有 skills 对齐的环节；有需要时在 **外层 skill** 里写一句「必须先过 TPQA 闭环再声明完成」，把 Superpowers **挂进编排图**里，而不是反过来说「一切都要 Superpowers」。

⚠️ 以上为架构取舍上的**推断/观点**，非对 obra/superpowers 仓库的逐项核对。

如果你的团队已经全员用 Superpowers：**「外层 TPQA orchestration skill + 内层 Superpowers 阶段 skills」** 往往最顺；若以仓库/PR 为准：**门禁 + Markdown 协议** 一样可以，不一定要绑 Superpowers。

Turn:26～26

本回复由 **Auto**（Cursor 中的 agent 路由器/助手接口）驱动；模型标识：**GPT‑5.2**。

<!-- DDM:TURN_SEP:v1 -->
