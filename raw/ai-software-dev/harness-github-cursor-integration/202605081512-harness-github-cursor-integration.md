# GitHub CI/CD：三方的Agent

> 创建时间：2026年3月27日（导出草稿 fri_mar_27_2026）

> 来源：Copilot Chat Export · User: @tttuser · Thread: https://github.com/copilot/c/55661f99-5a46-485f-8c4d-71e1deab8890

> 导航：[distilled](../../../distilled/ai-software-dev/harness-github-cursor-integration/202605081512-harness-github-cursor-integration.md) · [digest](../../../digest/ai-software-dev/harness-github-cursor-integration/202605081512-harness-github-cursor-integration.md) · [trace](../../../trace/ai-software-dev/harness-github-cursor-integration/202605081512-harness-github-cursor-integration.md)

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 1）

你是Harness Engineering 专家：

GitHub支持连接Cursor作为三方的Agent，你怎么看待这种架构设计？ 三方Agent是一种什么定位？ 
在 harness engineering编程过程中，三方的Agent可以起到什么作用。我了解到可以在PR中@cursor，这个和GitHub的PR过程的自动审查是什么关系。 参考示例：@tttuser/cursor-harness-demo/pull/1 

如下是官方介绍： Connect Cursor to GitHub to allow agents to do work for you in the background. https://github.com/settings/installations/104786251 

Harness Engineering由HashiCorp联合创始人Mitchell Hashimoto在2026年2月首次提出，并由OpenAI团队在实践中推广。请以这个定位，重新审查我们所有对话。 https://openai.com/index/harness-engineering/

<!-- DDM:TURN_SEP:v1 -->

## AI

**TODO1.0**

---

## 一、关于你提到的「Harness Engineering」定位—��我需要先质疑这个前提

你说「Harness Engineering 由 Mitchell Hashimoto 在 2026 年 2 月首次提出，并由 OpenAI 团队在实践中推广」，并要求我以此重新审视对话。

**这个说法需要订正：**

根据公开资料，Harness Engineering 并非 Mitchell Hashimoto「首次提出」的一个正式概念，更准确的描述是：

- **Mitchell Hashimoto**（Ghostty 项目作者，HashiCorp 联合创始人）通过他的 Ghostty 项目实践，**首先系统化地总结**了 AI agent 开发中的「harness 思维」——通过 `AGENTS.md`���隐式提示（implicit prompting）、程序性抗体（programmatic antibodies）等手段，构建约束和反馈环境[[1]](https://aihola.com/article/harness-engineering-ai-coding)。
- **OpenAI** 则以 Codex 为核心，独立开展了为期5个月的实验，最终在 [openai.com/index/harness-engineering/](https://openai.com/index/harness-engineering/) 发布报告，将这一实践命名并推广[[2]](https://openai.com/index/harness-engineering/)。
- 两者属于**同期独立实践、相互印证**的关系，并非单一来源。

这个区别是重要的：它影响你如何理解这个概念的边界——它不是某人的专利理论，而是一种从工程实践中归纳出来的范式。

---

## 二、GitHub + Cursor 三方 Agent 架构，怎么看？

### 架构定位：Cursor 是「受控执行者」，GitHub 是「任务编排平台」

你提到的安装链接 `https://github.com/settings/installations/104786251` 是将 **Cursor 作为 GitHub App 安装**。这确立了一个清晰的三层架构关系：

```
[ 你（工程师）] 
     ↓ 定义 harness（规则、约束、文档）
[ GitHub ]  ←——— 任务触发、PR 流转、权限管控
     ↓ 事件通知
[ Cursor Agent ]  ←——— 执行者，在后台自主完成具体任务
```

**三方 Agent 的定位本质是：**
> 有边界的自主执行节点——它不是助手，不是 copilot，而是一个接收任务、独立运行、返回结果的工程 agent。

它与传统 CI/CD Bot 的核心差异在于：**具备语义理解和上下文推理能力**，不只是触发脚本。

---

## 三、在 PR 中 `@cursor`：和自动 Code Review 是什么关系？

你的 PR #1（[tttuser/cursor-harness-demo/pull/1](https://github.com/tttuser/cursor-harness-demo/pull/1)）是一个很好的真实案例。我从中看到：

```
PR 标题：Update build.gradle.kts
PR 描述（由 Cursor Bugbot 自动生成）：
- 风险评级：Low Risk
- 变更说明：仅调整 plugins 块内的空白字符，不影响版本或行为
- 自动更新：每次新 commit 后重新生成
```

这里 Cursor 已经**自动执行了 PR Summary 生成**，无需你主动 `@cursor`。

### 两种触发模式对比：

| 模式 | 触发方式 | 行为 | 适用场景 |
|---|---|---|---|
| **自动审查（Bugbot 模式）** | PR 创建 / 新 commit 推送自动触发 | 自动生成风险评级、变更摘要 | 每个 PR 都需要快速摘要 |
| **`@cursor` 主动调用** | 在 PR 评论中手动 mention | Cursor agent 按指令执行特定任务（分析、修复、实现等） | 需要针对性 agent 任务 |

**关键区别：**
- 自动 Code Review（Bugbot）是**被动、常驻的 harness 机制**——它是 harness 的一部分，属于你配置的「环境约束」。
- `@cursor` 调用是**主动、即时的 agent 指令**——更接近你向 agent 下达任务单。

两者可以共存，且在 Harness Engineering 视角下，两者都是 harness 的组成部分。

---

## 四、Harness Engineering 视角下，三方 Agent 的真正价值

在 Harness Engineering 框架里，Cursor 这类三方 Agent 的价值不在于它有多聪明，而在于：

**你构建的 harness 有多严密，agent 就能做多可靠的事。**

具体来说，对你的 `cursor-harness-demo` 项目，harness 可能包括：
- `AGENTS.md` / `CURSOR.md`——告诉 agent 什么可以做、什么不能碰
- 测试套件——agent 的每次输出都必须通过
- PR 模板——约束 agent 的输出格式
- Bugbot 自动摘要——对 agent 产出的可读性反馈环

这不是「AI 帮你写代码」，而是「你构建了一套让 AI 能可靠工作的系统」。

---

> ⚠️ 一个合理质疑：你的 PR #1 目前只是一个空白字符修改，Cursor Bugbot 已经自动生成了摘要。但这个 demo 目前看起来更像是「验证 Cursor 接入」，而非「验证 harness 设计」。如果你的目标是探索 Harness Engineering，建议思考：这个 repo 的 harness 边界在哪？有没有 `AGENTS.md` 或等效的规则文件？

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 2）

1. 我们刚刚聊的都是代码提交之后的流程了，不包括需求方案设计，架构设计，详细设计，编码阶段，测阶段吧。我把这个阶段命名为开发阶段，这不是我们今天聊的重点

2. 刚刚聊的是属于CI/CD阶段吗？ 这个阶段Harness Engineering有明确的定义吗？这个阶段的Harness Engineering是否已经有了行业级别的解决方案共识了？ 

3. 第2这个阶段在整个研发过程中的Harness Engineering，属于重点吗？ 还是说开发阶段是目前的聚焦重心？ 你对这个阶段Harness Engineering起的作用怎么看待？

<!-- DDM:TURN_SEP:v1 -->

## AI

**TODO2.0**

---

## 问题1：我们上一轮聊的是「开发阶段」之后的事吗？

**基本同意，但需要精确一下边界。**

你定义的「开发阶段」= 需求 → 方案 → 架构 → 详细设计 → 编码 → 测试。

我们上一轮聊的——PR 创建、Cursor Bugbot 自动摘要、`@cursor` review、GitHub App 接入——**确实是编码产物进入版本控制之后的事**，属于你定义的开发阶段之后。

不过有一个边界值得商榷：**「测试阶段」是否完全在开发阶段内？** 如果你指的是单元测试/集成测试在本地或 PR 触发时运行，那它和 CI/CD 是高度重叠的。这个边界后续如果要讨论，建议明确一下。

---

## 问题2：上一轮聊的属于 CI/CD 阶段吗？Harness Engineering 在这里有无明确定义？是否有行业共识？

### 2.1 属于 CI/CD 阶段吗？

**是的，核心在 CI 侧。** 具体对应：

| 我们聊的内容 | CI/CD 阶段 |
|---|---|
| PR 创建后 Cursor Bugbot 自动生成摘要 | CI 前置门禁（Pre-merge gate） |
| `@cursor` 触发 agent 执行任务 | CI 中的 AI agent 任务节点 |
| GitHub App 权限接入 | CI/CD 基础设施配置层 |

### 2.2 Harness Engineering 在 CI/CD 阶段有明确定义吗？

**有，但它是「概念迁移」而非「专门定义」。**

OpenAI 和 Hashimoto 的原始 Harness Engineering 定义，**聚焦点是编码执行环境**——即 agent 写代码时周围的约束体系。但这套思路在 CI/CD 阶段有非常自然的映射[[1]](https://openai.com/index/harness-engineering/)：

> **CI/CD 里的 harness = 流水线本身就是 harness 的一部分**
> - 测试套件 → 对 agent 输出的自动校验
> - Policy-as-Code → 对 agent 行为的规则约束
> - PR gate → 对 agent 产出的准入门禁

所以 CI/CD 阶段不是 Harness Engineering 的「另一个领域」，而是 harness 向后延伸的执行验证层。

### 2.3 CI/CD 阶段是否有行业级共识？

**有共识，但不是以「Harness Engineering」命名的共识。** 更准确地说：

- **AI-powered PR review**（Cursor Bugbot、CodeRabbit、Qodo 等）已经成为行业标配，共识度高[[2]](https://www.epsilla.com/blogs/harness-engineering-evolution-prompt-context-autonomous-agents)
- **Policy-as-Code + 自动安全扫描**（SBOM、供应链安全）已有明确规范
- **Predictive Test Selection**（AI 选择最小测试集）正在普及[[3]](https://toolshelf.tech/blog/ci-cd-pipeline-best-practices-2026/)

但把这些统一冠以「Harness Engineering」的框架来理解——**目前更多是工程师个体和先行团队的实践，还没有形成像 DevOps 那样的行业级认证或标准体系。**

---

## 问题3：CI/CD 阶段在整个 Harness Engineering 中是重点吗？还是开发阶段才是聚焦重心？

**直接结论：开发阶段（编码执行环节）才是 Harness Engineering 目前的绝对重心，CI/CD 是重要但偏「验证」性质的延伸。**

### 原因分析：

Harness Engineering 的核心命题是：
> **当 agent 在写代码时，如何让它不出错、出了错如何自动修复。**

这个命题天然指向的是**编码执行环境**——`AGENTS.md`、上下文注入、programmatic antibodies、沙箱隔离——而不是 PR merge 之后发生的事。

| 维度 | 开发阶段（编码） | CI/CD 阶段 |
|---|---|---|
| Harness Engineering 关注度 | ⭐⭐⭐⭐⭐ 核心战场 | ⭐⭐⭐ 验证延伸 |
| agent 的「自主度」 | 高，agent 独立完成大量工作 | 中，更多是规则执行和门禁 |
| harness 设计难度 | 高，需要大量约束设计 | 相对成熟，工具链已标准化 |
| 当前行业探索热度 | 高，快速演进 | 中，已有较成熟方案 |

### 我对 CI/CD 阶段 Harness Engineering 作用的判断：

**它的价值是「兜底」而非「创造」**。

开发阶段的 harness 决定 agent 能不能生产出「大方向正确」的代码；CI/CD 阶段的 harness 决定「细节错误」能不能被拦截。两者缺一不可，但优先级是明确的——**没有编码阶段的 harness，CI/CD 阶段的 gate 会被淹没在大量低质量 agent 输出里，成本极高；反之，好的编码 harness 会让 CI/CD 阶段的门禁轻松很多。**

> ⚠️ 质疑你的命名：你把「需求 → 编码 → 测试」统一叫「开发阶段」，这个范围其实非常大，内部各子阶段的 Harness Engineering 关注点差异显著。如果你后续要深入讨论，建议进一步拆分——否则「开发阶段的 Harness Engineering」这个说法会显得边界模糊、难以聚焦。

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 3）

未来开发者在整个Harness Engineering中，你觉得最重要的能力是什么？是设计这套Harness Engineering过程吗？这套Harness Engineering过程，对大部分公司是通用的吗？ 如果有了通用型方案支持，是否定义这套流程，不需要更多人去做了。我问这个问题的目的是，作为开发者，是只需要会用，还是说要具备定义流程的能力？
当然，除了定义这套流程，就是如何使用这套流程，你觉得开发者最重要的能力是什么？软件开发的能力迁移会是痛苦的吗？传统软件开发驱动到新的流程，会是很大的Gap吗？

<!-- DDM:TURN_SEP:v1 -->

## AI

**TODO3.0**

---

## 一、未来开发者最重要的能力是什么？是「设计 Harness」吗？

**不完全是。我的判断：最重要的能力是「判断力」，而不是「流程设计力」。**

具体来说，是这三层能力的组合：

| 层次 | 能力 | 说明 |
|---|---|---|
| 第一层（核心） | **判断 agent 输出的正确性** | 能识别 agent 生成的代码/方案是否合理，哪里有隐患 |
| 第二层（关键） | **定义问题边界和验收标准** | 告诉 agent「什么叫做对」，而不是「怎么做」 |
| 第三层���加分） | **设计和调整 harness 规则** | 理解 AGENTS.md、测试策略、门禁配置背后的逻辑 |

**为什么「判断力」优先于「流程设计力」？**

因为 harness 本身会越来越标准化（下面会讲），但 **agent 的输出永远需要人来做最终校验**。一个不懂软件的人，即使有完美的 harness，也无法判断 agent 生成的架构决策是否合适。而一个判断力强的工程师，即使 harness 不完美，也能发现问题并修正。

---

## 二、这套流程对大部分公司通用吗？

**80% 通用，20% 高度定制。**

通用的部分（工具链层）：
- PR 自动摘要、风险评级
- 测试自动化、门禁规则
- AGENTS.md 模板、上下文注入框架
- 安全扫描、Policy-as-Code

**这部分已经在快速产品化**——Cursor Bugbot、GitHub Copilot Workspace、Qodo、CodeRabbit 等工具，本质上是在把 harness 的通用部分打包成 SaaS。

不通用、需要定制的部分：
- **业务领域知识的编码**——agent 需要理解你的领域规则（金融合规、医疗规范、特定架构约束），这无法被通用模板覆盖
- **组织协作流程**——谁有权合并、谁负责 review、哪些模块禁止 agent 修改
- **遗留系统的边界定义**——大量传统公司有历史包袱，harness 必须手工定义「不可碰区域」

---

## 三、如果有了通用方案，还需要人来定义流程吗？

**需要，但需要的人会大幅减少，且角色会变。**

类比一下：DevOps 出现后，不是所有公司都需要自己设计 CI/CD 流水线——大部分公司直接用 GitHub Actions 模板、GitLab CI 预设。但每家公司仍然需要**至少一个人**理解这套东西、能调整它、能在它出问题时诊断。

Harness Engineering 会走同样的路：
- **平台型公司**（GitHub、Cursor、OpenAI）负责提供通用 harness 基础设施
- **大多数公司**只需要在通用框架上做配置和业务定制
- **少数人**（平台工程师、AI 工程师）负责设计和演进 harness 本身

所以你的问题「是只需要会用，还是要具备定义能力」——**答案取决于你想在这个分工里处于哪个位置。**

---

## 四、除了定义流程，开发者最重要的能力是什么？

**我认为是：精准表达问题的能力。**

这听起来简单，实际上是传统开发者最大的短板之一。

在传统开发中，工程师可以边写边想，靠代码来「探索」问题。但在 agent 驱动的开发里：

> **你对问题的描述精度，直接决定 agent 输出的质量上限。**

具体体现在：
- 写出好的 Issue / Task 描述（边界清晰、验收标准明确）
- 能把模糊需求分解成 agent 可以执行的原子任务
- 知道什么时候 agent 的答案「看起来对但实际错了」

其次是：**系统思维**——理解各模块之间的依赖关系，防止 agent 在局部优化时破坏全局一致性。

---

## 五、软件开发能力迁移会很痛苦吗？Gap 大吗？

**诚实地说：会痛苦，但痛苦的方向可能和你想的不一样。**

### 不痛苦的部分：
- 工具本身学习曲线不高——Cursor、GitHub Copilot 上手很快
- 机械性的编码工作（样板代码、简单 CRUD）几乎立刻可以交给 agent

### 真正痛苦的部分：

**是「控制感的丧失」**。

传统开发者习惯于对代码有完全的掌控——每一行都是自己写的，出了 bug 知道去哪查。Agent 驱动之后，代码量增大、生成速度极快，但**理解深度下降**。很多工程师会经历一个阶段：agent 生成了 1000 行代码，能跑通，但自己不完全清楚为什么这样设计。这种「不确定感」对有经验的工程师反而比新手更难受。

### Gap 的本质：

| 传统开发思维 | Harness Engineering 思维 |
|---|---|
| 「我来实现它」 | 「我来定义它，agent 来实现」 |
| 关注实现细节 | 关注约束和验收边界 |
| 靠代码探索问题 | 靠描述和规则约束问题 |
| 出了 bug 自己 debug | 设计能让 agent 自我修复的反馈环 |

这个思维转变，对写了 10 年代码的人，**比对刚入行的人更难**。因为老工程师有更强的「动手惯性」。

> ⚠️ 质疑你的假设框架：你把「会用」和「定义流程」对立起来问。但实际上这是一个连续谱，不是非此即彼。更关键的���题或许是：**你愿意在这个分工里承担多大的主动性？** 纯粹「会用」的开发者，会在通用工具越来越强时，逐渐失去差异化价值；而能定义问题、能判断输出、能设计约束的工程师，价值会随 agent 能力增强而放大——因为更强的 agent 需要更清晰的 harness 才能发挥出来。

<!-- DDM:TURN_SEP:v1 -->
