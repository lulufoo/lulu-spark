# Harness Engineering Loop 设计分析

> 创建时间：2026年（草稿 `prompt-engineering-harness-engineering-loop-design_Version2.md`，稿内无日期）

> 来源：https://github.com/lulufoo/ai-software-dev/blob/main/harness-engineering/draft/prompt-engineering-harness-engineering-loop-design_Version2.md

> 导航：[distilled](../../../distilled/ai-software-dev/prompt-harness-engineering-loop-v2/202605081532-prompt-harness-engineering-loop-v2.md) · [digest](../../../digest/ai-software-dev/prompt-harness-engineering-loop-v2/202605081532-prompt-harness-engineering-loop-v2.md) · [trace](../../../trace/ai-software-dev/prompt-harness-engineering-loop-v2/202605081532-prompt-harness-engineering-loop-v2.md)

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 1）

（体裁：单篇分析稿，非 Copilot 多轮导出；正文与仓库草稿逐字一致。稿末在 GitHub 上即为截断状态。）

<!-- DDM:TURN_SEP:v1 -->

## AI


# Prompt Engineering 与 Harness Engineering Loop 设计分析

---

## 一、观点评价：「LLM已经很聪明，Prompt Engineering已经不重要」

### 观点定性：半对半错，但整体方向误判

这个观点捕捉到了一个真实现象——框架式提示词（CO-STAR、Magic Word）的门槛价值在下降。但它犯了一个**以局部替代整体**的错误。

---

### 正向论据（支持「PE不重要」的方向）

| 论据 | 说明 |
|---|---|
| 模型泛化能力提升 | GPT-4o/Claude 3.5+ 对自然语言理解已很鲁棒，简单任务不需要精心设计 Prompt |
| Agent 层封装 | 大量 Prompt 已被系统级 Agent 内化，普通用户无需感知（如 GitHub Copilot、Cursor 的内置 Prompt） |
| 结构化框架边际收益下降 | CO-STAR、RISEN 等框架在新模型上收益已不显著 |
| 工具链替代 | Function Calling、RAG、Tool Use 等工程化手段部分替代了 Prompt 技巧 |

---

### 反向论据（支持「PE 仍然重要/更重要」的方向）

| 论据 | 说明 |
|---|---|
| **模型倾向（Bias）的放大效应** | Loop 放大了模型的倾向，精准把握倾向比以前更关键 |
| **不可量化验收标准的设计** | 「UI 要高级」→ Claude 默认输出紫色渐变。Prompt 的精度决定了结果空间 |
| **幻觉控制仍高度依赖 Prompt** | 在医疗、法律、金融等高风险领域，Prompt 结构直接影响幻觉率 |
| **多步骤 Agent 链的漂移** | 单轮对话模型健壮，但在 5 步以上的 Agent 链中，每一步 Prompt 的模糊性都会积累误差 |
| **跨模型一致性** | 在多模型协作系统中，同一语义的 Prompt 在不同模型上产出差异极大，需要精准校准 |

### 结论

Prompt Engineering 的重心从「写出能让模型理解的 Prompt」转移到了「精准控制模型输出倾向的 Prompt」。这是一次**能力形态的升级，而非衰退**。

---

## 二、Harness Engineering Loop 深度分析

原文 Loop 结构：

```
design → spec → plan → TDD → review → 完成
                               ↑
                     review 不通过 → 回退到 plan 或 TDD
```

---

### Q1：这个 Loop 是谁在驱动？

**当前实践中：是 Agent 驱动，但人类是隐性的「导轨设计者」。**

- **表面上**：Orchestrator Agent 负责状态机流转——判断当前在哪个环节，调用对应的 Sub-Agent 执行
- **实质上**：驱动 Loop 的能量来自于**人在设计阶段预设的验收标准**
- 人的作用从「逐步指令下达者」转变为「规则与标准的设计者」

> 类比：不是司机，而是铁路的建造者——你建好轨道，火车（Agent）自己跑。

---

### Q2：每个环节是需要验收才算通过吗？

是的，但验收的性质不同：

| 环节 | 验收类型 | 验收方式 |
|---|---|---|
| design | 不可量化 | AI 自评 or 人工介入 |
| spec | 半可量化 | 结构完整性检查、关键字段覆盖率 |
| plan | 半可量化 | 任务分解合理性（可用另一个 Agent 评估） |
| TDD | **可量化** | 测试用例是否通过（自动化） |
| review | 混合 | Linter + 语义质量评分 |

---

### Q3：验收标准是什么？

**可量化验收标准（硬门槛）：**
- 测试用例通过率（如：≥95%）
- Linter 零报错
- 性能指标达标（接口响应时间、内存占用等）
- 代码覆盖率阈值

**不可量化验收标准（软门槛）——这是 PE 最关键的战场：**

- 错误示范：`"UI 要高级"`
- 正确示范：`"UI 遵循 Material Design 3 规范，色彩系统使用中性色为主色调，强调色不超过两种，组件间距符合 8px 网格系统，避免渐变和装饰性阴影"`

**→ 这正是 Prompt Engineering 价值的核心战场。**

---

### Q4：Loop 过程是自动执行、自动验收的吗？

理想状态是自动，现实是分层的：

```
完全自动层：TDD（单元测试）、Linter、性能测试
半自动层：  AI 自评 review（用 Judge LLM 打分）
人工介入层：design 方向确认、不可量化验收的边界校准
```

完全无人监督的 Loop 存在「自洽陷阱」——模型可能自己说自己通过了验收，但实际质量不达标。

---

### Q5：验收失败即停止执行吗？

**不应该是「停止」，而应该是「受控回退」：**

- 停止 = 浪费已有上下文，且无法收敛
- 受控回退 = 携带失败原因，返回指定步骤重试，设置最大重试次数

```
验收失败处理策略：
  if review 失败:
      携带失败原因 → 回退到 plan 或 TDD（由失败类型决定）
      重试计数 += 1
      if 重试计数 > MAX_RETRY:
          → 暂停，等待人工介入，输出当前状态快照
```

**无限循环是最危险的风险**，必须有熔断机制。

---

### Q6：专家级 Loop 设计方案

#### 核心设计原则

1. **验收标准先于执行**：每个环节开始前，验收标准必须已被明确定义
2. **失败携带上下文**：回退时必须附带结构化的失败原因，不能裸回退
3. **熔断优于无限循环**：设置每环节最大重试次数，超限转人工
4. **软标准硬描述**：不可量化的验收标准必须被转化为可被模型稳定理解的精确描述
5. **Judge 与 Executor 分离**：执行 Agent 和评估 Agent 不能是同一个，避免自洽陷阱

---

#### Loop 状态机设计

```
┌─────────┐    通过     ┌──────┐    通过     ┌──────┐
│ design  │ ──────────►│ spec │ ──────────►│ plan │
└─────────┘            └──────┘            └──────┘
     ▲                                          │ 通过
     │ 熔断→人工                                 ▼
     │                                      ┌──────┐
┌─────────┐   失败（逻辑问题）               │  TDD │
│ review  │ ◄──────────────────────────── └──────┘
└─────────┘                                    │ 通过
     │ 通过                                     ▼
     ▼                                      ┌──────┐
  完成 ✅                                   │review│◄─┐
                                           └──────┘  │
                                               │ 失败   │ 代码质量问题
                                               └───────┘（回退 TDD）
```

---

#### 各环节设计规格

**design**
- 执行者：Design Agent（输入：原始需求自然语言）
- 产出物：结构化设计文档（用户故事、边界条件、技术约束）
- 硬门槛：包含`用户故事`、`成功指标`、`排除项`三个必填字段
- 软门槛 Judge Prompt 示例：`"评估以下设计文档是否清晰定义了系统边界，是否每个用户故事都有可观测的完成状态，输出 JSON：{pass: bool, issues: [...]}"`
- 最大重试：2 次，超限人工介入

**spec**
- 执行者：Spec Agent（输入：design 文档）
- 产出物：技术规格文档（API 契约、数据模型、接口定义）
- 硬门槛：JSON Schema 格式校验通过，所有 API 端点有入参/出参定义
- 软门槛：Judge LLM 评估 spec 与 design 的覆盖率（≥90% 用户故事有对应 spec）
- 最大重试：2 次

**plan**
- 执行者：Planning Agent（输入：spec 文档）
- 产出物：任务分解列表（每个任务有明确的文件路径、函数签名、预期行为）
- 硬门槛：每个任务 item 有 `file_path`、`function_signature`、`expected_behavior` 字段
- 软门槛：Judge LLM 评估任务间依赖关系是否有循环依赖
- 最���重试：2 次

**TDD**
- 执行者：TDD Agent（输入：plan 的单个任务）
- 产出物：先写测试用例，再写实现代码
- 硬门槛：测试通过率 100%（核心路径）、Linter 零 error、覆盖率 ≥80%
- 最大重试：3 次，每次回退携带测试失败日志
- 注意：TDD Agent 失败时，**不回退到 plan**，除非 Judge 判定是任务定义本身有歧义

**review**
- 执行者：Judge LLM（与 Executor 完全分离，建议使用不同模型）
- 评估维度：代码质量、与 spec 的一致性、安全风险扫描、不可量化维度
- 回退路由规则：
  - 代码实现问题 → 回退 TDD
  - 任务分解问题 → 回退 plan
  - 需求理解问题 → 回退 spec（罕见，需人工确认）
- 最大重试：3 次（含跨环节回退计数）

---

#### 熔断与人工介入协议

```
触发条件：
  - 任意环节重试次数超限
  - review 连续失败且回退路由不确定
  - Judge LLM 置信度 < 0.6

触发动作：
  1. 暂停 Loop 执行
  2. 输出当前状态快照（已完成环节、当前失败环节、失败原因）
  3. 通知人工，提供三个选项：
     a. 修改验收标准后继续
     b. 指定回退目标环节后继续
     c. 终止并输出当前成果
```

---

#### Prompt Engineering 在 Loop 中的核心位置

| 环节 | Prompt 的关键作用 |
|---|---|
| design | 控制「需求理解倾向」——同样一句需求，不同 Prompt 会让模型关注不同侧面 |
| spec | 控制「规格粒度倾向」——太粗 or 太细，由 Prompt 中的示例样本决定 |
| TDD | 控制「测试覆盖倾向」——模型倾向于写 Happy Path，需要 Prompt 强制引导 Edge Case |
| review | **最关键**——软验收标准的精确度直接决定 Loop 收敛到什么质量 |

> **核心结论**：Loop 放大了模型对 Prompt 的倾向。Prompt 的精度，决定了 Loop 收敛后的质量上限。

---

#### 适用技术栈参考

- Orchestrator：LangGraph / AutoGen / CrewAI
- Judge LLM：独立部署，建议与 Executor 模型族不同（如 Executor 用 Claude，Judge 用 GPT-4o）
- 测试执行：pytest / Jest，容器化隔离运行
- 状态持久化：Redis / 数据库记录每个 Loop 状态，支持断点续跑

---

## 三、对原文的一处合理质疑

原文说「回退到 plan 或 TDD」，但没有说明**回退路由的决策逻辑是什么**。

这是实践中非常关键的设计缺口——如果 review

<!-- DDM:TURN_SEP:v1 -->
