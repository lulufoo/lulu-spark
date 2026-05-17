# lulu-dev-workflow / product：设计优化

> 创建时间：2026年5月16日 23:02

> 导航：[distilled](../../../distilled/ai-software-dev/lulu-dev-workflow-product-design/202605162302-design-optimization.md) · [digest](../../../digest/ai-software-dev/lulu-dev-workflow-product-design/202605162302-design-optimization.md) · [trace](../../../trace/ai-software-dev/lulu-dev-workflow-product-design/202605162302-design-optimization.md)



<!-- DDM:TURN_SEP:v1 -->


## User（Turn 1）

安装SKILL：https://github.com/lulufoo/lulu-dev-skills/blob/main/product-doc-workflow/SKILL.md


<!-- DDM:TURN_SEP:v1 -->


## AI

先按工作区规则，通过 `gh api` 获取文件内容，同时读取创建技能的指引。


<!-- DDM:TURN_SEP:v1 -->


## User（Turn 2）

product-doc-workflow init


<!-- DDM:TURN_SEP:v1 -->


## AI

（已剥离推导性独白，本回合无可保留的 AI 正文）


<!-- DDM:TURN_SEP:v1 -->


## User（Turn 3）

为什么  hooks/product-doc-transition-guard.py  workflow_common.py 都init到 当前项目目录下了？  为什么不是  /Users/lulu/.cursor/skills/product-doc-workflow?


<!-- DDM:TURN_SEP:v1 -->


## AI

先查看 `init.py` 的实际逻辑。


<!-- DDM:TURN_SEP:v1 -->


## User（Turn 4）

@hooks.json (5-10) 
这个为什么没问题


<!-- DDM:TURN_SEP:v1 -->


## AI

（已剥离推导性独白，本回合无可保留的 AI 正文）


<!-- DDM:TURN_SEP:v1 -->


## User（Turn 5）

这个设计有问题，这个init 不应该拷贝这2个py文件，而是install的时候，自动就放到 /Users/lulu/.cursor/skills/product-doc-workflow 目录


<!-- DDM:TURN_SEP:v1 -->


## AI

先看 `workflow_common.py` 里 `merge_hook_entry` 和相关常量的定义。


<!-- DDM:TURN_SEP:v1 -->


## User（Turn 6）

product-doc-workflow 也需要调整，全部放到 ~/.cursor/skills/... 路径 。如果是项目执行状态的记录，参考 cursor-rule-guard/rules-state的设计，按照 ab926751-3a56-4726-a576-ed354fd58c03 目录区分会话 。


<!-- DDM:TURN_SEP:v1 -->


## AI

先了解 `cursor-rule-guard/rules-state` 的设计，以及当前 product-doc-workflow 的完整结构。


<!-- DDM:TURN_SEP:v1 -->


## User（Turn 8）

还有一个问题，启动之后需要重置状态，因为同一个会话可能会有多次的产品文档生成。


<!-- DDM:TURN_SEP:v1 -->


## AI

这涉及两个子问题：

**问题 A：如何触发重置？**

重置 `state.json` 回 `Drafting` 有几种方式：

| 方案 | 说明 |
|------|------|
| 重新运行 `init` | `init.py` 直接写文件，绕开 hook 验证，state.json 重置为 Drafting |
| 加一条 `reset` 命令 | SKILL.md 新增 `reset` 操作，专门重置当前项目的 state.json |
| 在 whitelist 加 `any → Drafting` | 允许 AI 直接写状态迁移到 Drafting，走正常 hook 流程 |

**问题 B：同一 conv_id 下多个文档的归档如何区分？**

目前设计 `archive/<conv_id>/state.json` 会被后一次覆盖前一次。需要加入文档标识：
- `archive/<conv_id>/<timestamp>/` — 按时间戳区分
- `archive/<conv_id>/<doc_id>/` — 明确指定文档 ID


<!-- DDM:TURN_SEP:v1 -->


## User（Turn 11）

唯独缺 Delivered → Drafting ： 这不是一个很好的设计，最好设计一个特殊指令，这个指令将状态从任何阶段，重置回到Drafting。这个特殊指令，例如Reset，是SKILL启动的时候设置的。不过，SKILL启动需要一个特殊的命令启动，和 inistall / init 并列。


<!-- DDM:TURN_SEP:v1 -->


## AI

理解了。设计三层命令并列：`install`（机器级）→ `init`（项目级）→ `start`（会话/文档级）。

`start` 通过 Shell 执行脚本直接写 `state.json`，绕过 hook（hook 只拦截 Write/Edit 工具调用，不拦截 Shell），所以可以从任意状态重置回 Drafting，不需要在 whitelist 里加特殊边。

更新计划：


<!-- DDM:TURN_SEP:v1 -->


## User（Turn 12）

还有一个架构问题，目前 product-doc-workflow 只是研发工作流的一个环节。需要增加一级目录设计，可以叫： lulu-dev-workflow。 product-doc-workflow to product，放到 lulu-dev-workflow 目录下，你觉得呢？


<!-- DDM:TURN_SEP:v1 -->


## AI

好设计。`lulu-dev-workflow` 作为顶层命名空间，未来可以放 `design`、`dev`、`qa` 等其他环节，产品文档是其中一个 `product` 子模块。

**我的看法：**

结构上清晰，赞同。有两个小问题需要确认：


<!-- DDM:TURN_SEP:v1 -->


## User（Turn 13）

workflow-config.json 应该放到：.cursor/lulu-dev-workflow/。后续product / tech / ...等模板，都放到一个json配置，否则需要配置多个文件。  AI 写的文件，按照会话，放到各自的skill目录下，如：.cursor/lulu-dev-workflow/product/。

你觉得呢》


<!-- DDM:TURN_SEP:v1 -->


## AI

好设计，赞同。两点优化都有价值：

1. **`workflow-config.json` 提到父级** — 避免后续 `product` / `tech` 各自维护配置文件，URLs 集中管理
2. **AI 写的文件按会话放到项目子目录** — 状态天然隔离，不需要 hook 归档到全局 skills 目录，更简洁

唯一需要解决的还是老问题：**AI 写文件时如何知道自己的 conversation_id？**

不过这次有解法——从系统上下文可以找到。当前会话的 `conversation_id` 在 agent_transcripts 目录里（系统注入的规则里能看到格式：`ab926751-3a56-4726-a576-ed354fd58c03`）。SKILL.md 可以明确指示 AI：写状态文件前，先从 agent_transcripts 文件夹确认当前会话 ID。

更新后的结构：
~/.cursor/skills/lulu-dev-workflow/
├── SKILL.md                            ← 顶层总览
└── product/
    ├── SKILL.md
    ├── transition-whitelist.json
    ├── scripts/                        ← 只有脚本，不再有 archive/
    └── templates/

项目: .cursor/lulu-dev-workflow/
├── workflow-config.json                ← 共享配置（product/tech 的 URLs 合并在此）
└── product/
    └── <conversation_id>/              ← AI 按会话写入
        ├── state.json
        └── delivery-approval.json
`workflow-config.json` 结构变化：

更新计划：

Wait, re-reading: "AI 写的文件，按照会话，放到各自的skill目录下，如：.cursor/lulu-dev-workflow/product/"

I'm clarifying what "各自的skill目录下" means - they're saying the AI-written files stay within the project structure under `.cursor/lulu-dev-workflow/product/` organized by session, not in the global skills directory. So the revised structure would have `workflow-config.json` at `.cursor/lulu-dev-workflow/` as the shared config, while session-specific state files live in the project's sub-workflow directories. I'm realizing the AI needs to know its conversation ID to write files to the correct session subdirectory, or alternatively the AI could write to a fixed location and let the hook handle organizing it by session. The user's instruction about placing files "under their respective skill directories" is ambiguous—it could mean each sub-workflow has its own directory structure with per-session subdirectories underneath, or it could mean the AI writes to a standard path that gets sorted later. I can see the conversation_id is available in the system context through the agent transcripts folder structure. So the approach would be: AI writes to `.cursor/lulu-dev-workflow/product/<conversation_id>/state.json`, where it determines its own conversation_id from the agent transcripts metadata, and the hook validates that the written conversation_id matches the current session. This keeps everything organized by session while staying self-contained. is referring to the sub-workflow's directory within the project itself. So the files would be organized by conversation ID within that skill directory. I'm thinking the structure would have a shared workflow config at the project level, then conversation-specific state files nested under the product skill folder. Since the AI needs to write to conversation-specific paths, it would need access to the current conversation ID—which I could document in the SKILL.md as being available from the agent transcripts in the system context. The global skills directory would then serve as a fallback or template location.


<!-- DDM:TURN_SEP:v1 -->


## User（Turn 14）

执行吧，执行完了把skill 和。项目目录清理。


<!-- DDM:TURN_SEP:v1 -->


## AI

开始执行。先读取还未读的文件。


<!-- DDM:TURN_SEP:v1 -->


## User（Turn 16）

现在要做一个事情：

1 就是 https://github.com/lulufoo/ai-software-dev/tree/main/lulu-dev-workflow-template  目录下，生成一个配置好的  workflow-config.json 文件。
2 /lulu-dev-workflow  增加一个指令，输入一个URL文档，将这个URL的 workflow-config.json 文档拷贝，并覆盖到本地。

这样避免自己手动写。


<!-- DDM:TURN_SEP:v1 -->


## AI

先看 GitHub 目录内容和本地 workflow-config.json。

2. The current local workflow-config.json content


<!-- DDM:TURN_SEP:v1 -->


## User（Turn 20）

https://github.com/lulufoo/lulu-dev-skills/tree/main  readme更新一下。  cursor-rule-guard 和 lulu-dev-workflow 暂时分开。


<!-- DDM:TURN_SEP:v1 -->


## AI

先读取当前 README。


<!-- DDM:TURN_SEP:v1 -->


## User（Turn 22）

本项目的首页，搜索知识库，我想增加搜索历史记录。workbench / 知识库 分别记录。最多记录10条。


<!-- DDM:TURN_SEP:v1 -->


## AI

先探索项目结构，找到首页搜索的相关代码。


<!-- DDM:TURN_SEP:v1 -->


## User（Turn 23）

为什么没有进入我的SKILL流程呢 ？


<!-- DDM:TURN_SEP:v1 -->


## AI

对，我直接跳到实现计划了，跳过了 SKILL 流程。

当前状态是 `Drafting`，按 SKILL 规则应该先按模板起草产品文档，再进入 Evaluating。先读 `workflow-config.json` 里的模板 URL，拉取产品文档模板。


<!-- DDM:TURN_SEP:v1 -->


## User（Turn 24）

是的，进入评估


<!-- DDM:TURN_SEP:v1 -->


## AI

需要切换到 Agent 模式来写 state.json 触发迁移。


<!-- DDM:TURN_SEP:v1 -->


## User（Turn 29）

有几个问题，请审查应该如何解决？ 

1  Turn22 是哪里的原因？ 需要如何优化SKILL ？ 
2  state.json 不能写入，为什么还可以继续推进流程？ 
3  Turn22 的文档记录，为什么没有生成一个文档，放到.cache目录？ 
4  PDQA 11 个维度逐项检查，审查问题记录，为什么没有文档，放到.cache目录？ 
5  问题 1：异常流程缺失（高优先级） 没有弹出选择框。而是文本提示的。如何实现？ 
6  问题解决过程，没有更新 审查问题记录 的文档。
7  问题 3 已解决 ，全部问题解决了，没有更新产品文档。
8 评估通过，文档可以进入 ReadyForDelivery。此时，应该推进状态到 Delivery，给我终版的产品文档。


<!-- DDM:TURN_SEP:v1 -->


## AI

很好的审查，8 个问题逐条分析：

---

**1. Turn22 原因 + SKILL 优化**

原因：Turn21 用户提问时没有附带 SKILL，我没有主动检查 state.json 就跳进了实施计划模式。

SKILL.md 需要补充一条 Operating Rule：
> 在任何有产品需求的对话开始前，AI 必须主动读取 `.cursor/lulu-dev-workflow/product/<conversation_id>/state.json`，根据当前状态决定行为。

---

**2. state.json 无法写入还继续推进**

Plan 模式无法写文件，但我没有停下来而是继续做评估工作——这违反了状态机约束。

SKILL.md 需要补充：
> 状态迁移必须先写入 state.json（hook 验证通过）才能进入下一阶段的工作。无法写入时，AI 应停止并提示用户切换到 Agent 模式。

---

**3 & 4. 文档没有存到 .cache 目录**

SKILL.md 目前没有规定文档落盘。需要补充两条规则：
- Drafting 阶段完成后，将产品文档存到 `.cache/lulu-dev-workflow/product/<conv_id>/product-doc.md`
- Evaluating 阶段，将 PDQA 检查记录存到 `.cache/lulu-dev-workflow/product/<conv_id>/pdqa-review.md`，每个问题解决后实时更新

---

**5. 应该用 AskQuestion 工具，而不是文本提示**

SKILL.md 需要补充：
> Evaluating 阶段每发现一个问题，使用 `AskQuestion` 工具展示选项，不用文本列举 A/B。

---

**6 & 7. 审查记录和产品文档没有随问题解决实时更新**

同属 3&4 的落盘规则缺失。规则需明确：每个问题解决后立即更新对应文档，不等到最后再写。

---

**8. 推进到 ReadyForDelivery，给终版产品文档**

这是当下要做的。先整理终版文档，再写 state.json。

---

**现在的行动计划（需要切到 Agent 模式执行）：**

1. 更新 SKILL.md 补充以上 6 条规则
2. 创建 `.cache/lulu-dev-workflow/product/<conv_id>/product-doc.md`（含评估修订）
3. 创建 `.cache/lulu-dev-workflow/product/<conv_id>/pdqa-review.md`（含全部问题记录）
4. 写入 state.json → `Evaluating`（补写过渡）
5. 写入 state.json → `ReadyForDelivery`
6. 输出终版产品文档

现在要切到 Agent 模式执行，还是先看终版文档再确认？

Turn:1～28


<!-- DDM:TURN_SEP:v1 -->


## User（Turn 31）

你说的：“无法写入时，AI 应停止并提示用户切换到 Agent 模式。”  可以在Plan模式下实现写入吗？ 调研一下方案看看？


<!-- DDM:TURN_SEP:v1 -->


## AI

先查看 Cursor Plan 模式的实际限制范围——是工具级限制还是仅指令级限制。


<!-- DDM:TURN_SEP:v1 -->


## User（Turn 32）

需要思考状态记录的目的是什么，不仅仅是为了记录状态，还有避免跳步。如果切换使用py脚本实现，可以达成这2个目的，我觉得是可以的。 请你基于这个思考，给我一个方案。


<!-- DDM:TURN_SEP:v1 -->


## AI

---

## 核心思路

当前设计的问题是：**状态机守卫逻辑在 hook_guard.py 里，只能拦截 Write 工具调用**。Plan 模式下 AI 无法用 Write 工具，守卫就失效了。

重新定位：**守卫逻辑应该在 transition.py 脚本里，而不是依赖 hook**。

---

## 方案：Script-First 状态机

### 架构调整
当前：AI 用 Write 工具写 state.json → hook_guard.py 拦截 → 校验迁移合法性

改为：AI 用 Shell 调用 transition.py → 脚本内部校验 → 合法则写 state.json
                                                       ↓
                                              hook_guard.py 作为 fallback 防线
### transition.py 的职责

# 脚本内部逻辑：
# 1. 读取当前 state.json 的 current_state
# 2. 对照 transition-whitelist.json 校验 (current → target) 是否合法
# 3. 如需 delivery-approval：检查 delivery-approval.json 是否存在
# 4. 合法 → 写 state.json，输出成功信息
# 5. 不合法 → 输出错误，exit 1，state.json 不变
### 两个目的如何实现

| 目的 | 实现方式 |
|------|---------|
| 记录状态 | 脚本成功时写 state.json |
| 避免跳步 | 脚本内部校验白名单，不合法直接 exit 1，文件不写 |

### Plan 模式下为何可行

Shell 工具在 Plan 模式下可用（本次对话已多次使用）。AI 通过 Shell 调用 `transition.py`，写操作在 Python 进程内发生，**不经过 Write/Edit 工具**，Cursor Plan 模式不拦截。

---

## 完整方案文件清单

**新建：**
- `~/.cursor/skills/lulu-dev-workflow/product/scripts/transition.py`
  - 内含状态机校验逻辑（读白名单 → 验合法 → 写状态）

**修改：**
- `hook_guard.py`：降级为 fallback（防止 AI 绕过脚本直接 Write）
- `start.py`：保持不变（新会话初始化，合理绕过）
- `SKILL.md`：更新"状态迁移"指令，明确使用 `transition.py`，Plan/Agent 模式均可用

**不变：**
- `transition-whitelist.json`：状态机规则不变
- `delivery-approval.json`：审批文件不变

---

## 新旧对比

| | 旧方案 | 新方案 |
|--|--------|--------|
| 写入工具 | Write 工具 | Shell → Python 脚本 |
| 守卫位置 | hook_guard.py（hook 层） | transition.py（脚本层）+ hook_guard.py（fallback） |
| Plan 模式 | ❌ 无法写入 | ✅ 可用 |
| 跳步防护 | ✅ hook 拦截 | ✅ 脚本校验 |
| 防止绕过 | ✅（Write 必经 hook） | ✅（hook 仍拦截直接 Write） |

---

确认方案后，执行内容：创建 `transition.py`，更新 `hook_guard.py` 和 `SKILL.md`。是否确认？

Turn:1～30


<!-- DDM:TURN_SEP:v1 -->


## User（Turn 33）

这里有一个新的视角思考，cursor-rule-guard 使用hook，为什么可以？ 优势是什么？ 因为代码写入本来就需要在agent模式下。另外，hook可以做到用户无感。

那么，本次模式下，Turn28 回复的8个问题，写入文档本身在Plan模式下可以完成吗？ 如果不行，状态写入修改了，后面还是有问题。 另外，我们的状态机设计，可以做到状态迁移和拦截，剥离于SKIIL之外，让SKILL无感吗？


<!-- DDM:TURN_SEP:v1 -->


## AI

先查证当前的相关设计文件。

---

The real issue in Turn22 was that the user expected the SKILL to write documents while staying in Plan mode, but that's not possible. The product-doc-workflow SKILL is fundamentally an Agent-mode workflow because it produces written artifacts. While hooks could handle some state transitions automatically—like detecting when a document is written and advancing the state—the SKILL still needs Agent mode to create those files in the first place. 档生产工作流本质上需要在 Agent 模式下运行，所以解决方案是在 SKILL 的入口处明确要求 Agent 模式。虽然可以通过 hook 让状态机更自动化，但 Plan 模式下根本无法实现这个工作流。


<!-- DDM:TURN_SEP:v1 -->


## User（Turn 34）

那为什么Plan模式下，可以写入plan文档呢？ Cursor官方怎么解释的呢？ Cursor官方怎么定义产品文档在哪个阶段写入呢？


<!-- DDM:TURN_SEP:v1 -->


## AI

先查阅 Cursor 官方资料。


<!-- DDM:TURN_SEP:v1 -->


## User（Turn 35）

修改之前，我需要重新把方案架构理一下。这个方案设计到的一些元素：1、 SKILL流程；2 、Hook流程； 3、状态机驱动；4、产品模板；5、产出文档。 还有需要补充的核心元素吗？ 基于这些核心元素，画一下方案架构。


<!-- DDM:TURN_SEP:v1 -->


## AI

先读取当前的脚本和配置，确保架构图准确反映现有设计。

<!-- DDM:TURN_SEP:v1 -->


## User（Turn 36）

问题 2 这里设计到一个根本问题，状态机的状态流转，是否可以完全使用文件写入来映射？ 副作用是什么？


<!-- DDM:TURN_SEP:v1 -->


## AI

---

## 核心问题：文件写入 ≠ 语义事件

文件写入是**低层级的操作行为**，状态迁移是**高层级的语义意图**。把两者强行映射，会引入歧义。

---

## 副作用分析

### 副作用 1：同一文件，在不同状态写入，语义不同

| 当前状态 | 写 product-doc.md 的含义 | 触发迁移？ |
|---------|------------------------|---------|
| Drafting | 草稿迭代中 | ❌ 不该迁移 |
| Drafting | 草稿完成，准备评估 | ✅ 应迁移 → Evaluating |
| Evaluating | 问题修复后更新文档 | ❌ 不该迁移 |

Hook 只能看到"哪个文件被写了"，看不到"这次写是第几次"或"写完之后 AI 的意图是什么"。**无法区分。**

### 副作用 2：回退迁移无法映射到文件写入

| 迁移 | 对应什么文件写入？ |
|------|----------------|
| Evaluating → Drafting（评估不通过） | 无对应文件，写 product-doc.md 含义不清 |
| ReadyForDelivery → Drafting（需改动） | 无对应文件 |

**回退路径天然没有对应的文件写入，文件映射只能覆盖正向迁移。**

### 副作用 3：重复写入破坏幂等性

文档在一个状态下可能被写入多次（迭代修改）。如果每次写入都触发状态推进，状态会被重复触发：
写 product-doc.md 第1次 → Drafting → Evaluating ✅
写 product-doc.md 第2次（评估中修改）→ Evaluating → ??? ❌
除非加"当前状态下已写过此文件"的幂等锁，但这会引入新的复杂度。

---

## 结论：文件写入不能完全承载状态机

文件写入可以**部分映射**（明确的 happy path 正向迁移），但不能**完全映射**：
可映射：        产出文档写入 → 正向迁移（需要幂等保护）
不可映射：      回退迁移（无对应文件写入）
不可映射：      同一文件多次写入的意图区分
---

## 更干净的设计：显式语义接口

`transition.py` 提供的不是"AI 直接操作状态文件"，而是一个**语义接口**：
bash
# AI 的意图：明确、无歧义
python3 transition.py --to Evaluating   # "我完成草稿了"
python3 transition.py --to Drafting     # "评估发现问题，回退修改"
这和"AI 写 state.json"有本质区别：

| | AI 写 state.json | AI 调用 transition.py | 文件映射自动推进 |
|--|-----------------|----------------------|--------------|
| 幂等性 | ❌ AI 随时可写 | ✅ 脚本校验 | ❌ 重复写触发 |
| 回退路径 | ✅ 支持 | ✅ 支持 | ❌ 无法映射 |
| 语义清晰 | ❌ 机械操作 | ✅ 显式意图 | ❌ 依赖约定 |
| Plan 模式 | ❌ JSON 不可写 | ✅ Shell 可用 | ✅ |
| SKILL 无感 | ❌ 需知道 JSON 格式 | ⚠️ 需知道脚本接口 | ✅ |

---

## 推荐方案

**核心变化：** SKILL 无感的目标从"不感知状态"改为"不感知状态文件格式"——SKILL 只需要知道"何时调用 `transition.py --to X`"，不需要知道 state.json 的 JSON 结构。

- ✅ 文件写入专注于内容产出（产品文档、PDQA 记录）
- ✅ 状态迁移通过 `transition.py` 显式触发（Shell，Plan 模式可用）
- ✅ hook_guard.py 作为 fallback，防止直接 Write state.json
- ✅ 回退路径完整支持

更新架构文档？

Turn:1～33


<!-- DDM:TURN_SEP:v1 -->


## User（Turn 37）

也就是，我们在做一个架构抉择，需要使用一个不那么好的架构，来实现功能的完整性。而不是为了架构的完整，损失功能。


<!-- DDM:TURN_SEP:v1 -->


## AI

这是对的。用一句话总结这个抉择：

**功能完整性优先于架构纯粹性。**

具体到这个场景：

- **纯粹的架构**：状态机完全由文件写入自动驱动，SKILL 完全无感。优雅，但有功能盲区（回退路径、幂等问题）。
- **务实的架构**：SKILL 显式调用 `transition.py`，多一个步骤，但状态机完整、所有迁移路径都可达。

这不是妥协，是在约束条件下的正确选择。约束是：文件写入无法承载语义意图，强行映射会引入不可预知的副作用。

---

确认这个决策，那推荐方案就是：

1. **状态迁移**：SKILL 显式调用 `transition.py --to <state>`（Shell，Plan 模式可用）
2. **文件写入**：专注产出文档（product-doc.md、pdqa-review.md），不承载状态语义
3. **hook_guard.py**：保留为 fallback，防止意外的直接 state.json 写入

现在可以更新架构文档，然后进入实施吗？

Turn:1～34


<!-- DDM:TURN_SEP:v1 -->


## User（Turn 38）

SKILL 显式调用 transition.py --to <state> 目的是什么？ 为什么不能使用state.json？ 如果是Plan模式不允许写入，把文档改成.md格式不就可以了？


<!-- DDM:TURN_SEP:v1 -->


## AI

这个思路是对的，逐一拆解。

---

## transition.py 的目的是什么

它被提出，**唯一动机是绕过 Plan 模式不能写 JSON 的限制**，顺带内置了校验逻辑。不是架构上必需的，是对限制的一个工程补丁。

---

## 为什么不能直接写 state.json

只有一个原因：Plan 模式只允许写 `.md` 文件，`.json` 被拦截。

---

## state.md 方案可行吗

✅ **完全可行**，而且更简单。把状态文件改成 Markdown 格式：

Plan 模式下 AI 可以直接写入这个文件，hook_guard.py 用正则解析 `current_state` 字段，原有的校验逻辑完全保留：

这样：
- ✅ Plan 模式可直接写
- ✅ hook 照常拦截并校验
- ✅ 不需要 transition.py
- ✅ SKILL 无需改变写法，只是写 .md 而不是 .json

---

## 结论：transition.py 不需要了

之前绕了一圈，用户一句话找到了更直接的路。

| 方案 | Plan 模式写入 | 校验完整 | 额外复杂度 |
|------|-------------|---------|---------|
| 直接写 state.json | ❌ | ✅ | 无 |
| transition.py via Shell | ✅ | ✅ | 增加脚本 |
| **state.md（YAML frontmatter）** | ✅ | ✅（hook 解析）| **最低** |

---

## 唯一需要确认的问题

`delivery-approval.json` 也是 JSON，同样面临 Plan 模式限制。它也改成 `.md` 吗？

如果是，那整个工作流（除了 `hooks.json` 本身）都可以在 Plan 模式下运行。

Turn:1～35


<!-- DDM:TURN_SEP:v1 -->


## User（Turn 39）

delivery-approval.json 目的是什么 ？


<!-- DDM:TURN_SEP:v1 -->


## AI

---

## delivery-approval.json 的设计目的

它是一个**用户意图锁**，防止 AI 自行将状态推进到 `Delivered`。

当前设计逻辑：
ReadyForDelivery → Delivered
  要求：同一会话目录下存在 delivery-approval.json 且 approved: true
**为什么需要这个文件？**

不加这个锁，AI 可以在没有用户确认的情况下，自行将状态从 `ReadyForDelivery` 推进到 `Delivered`。`Delivered` 是终态，语义是"交付完成，流程结束"，这个决定应该由**用户明确触发**，而不是 AI 自动完成。

本质上，它是一个**人工确认节点的物化形式**——hook 检查这个文件是否存在，等价于检查"用户是否已经批准交付"。

---

## 这个设计还需要吗

如果改成 `.md` 格式，它变成 `delivery-approval.md`：
markdown
---
approved: true
approved_at: 2026-05-17T09:00:00+08:00
---
目的不变，Plan 模式下可以写入，hook 解析 `approved: true` 字段。

但有一个更根本的问题值得确认：**这个审批意图，是否可以用其他方式表达？**

比如：
- 用户在聊天中说"同意交付"→ SKILL 写 `delivery-approval.md` → hook 检查
- 或者：直接取消这个文件，把审批判断内化到 SKILL 流程（用户说"交付"→ AI 调用 transition）

后者更简单，但失去了 hook 层面的硬约束——AI 理论上可以在未经明确授权时写入 Delivered 状态。

你觉得这个硬约束有必要保留吗？

Turn:1～36


<!-- DDM:TURN_SEP:v1 -->


## User（Turn 40）

它和 state.json 区别是什么？ 不都是状态推进吗？ 还是说为了将人工状态推进了AI的推进区分开开？


<!-- DDM:TURN_SEP:v1 -->


## AI

---

## 本质区别：谁有权写

| 文件 | 谁来写 | 含义 |
|------|--------|------|
| `state.json` | AI 写 | AI 认为"可以推进了" |
| `delivery-approval.json` | AI 代表用户写（用户明确授权后） | 用户确认"同意交付" |

你最后一句话说到了核心：**是的，就是为了把人工确认和 AI 推进区分开。**

---

## 为什么需要区分

状态机的其他迁移（Drafting → Evaluating → ReadyForDelivery）是 AI 主导的过程判断，AI 认为"草稿完成了"、"评估通过了"，这些判断可以由 AI 自主做。

`ReadyForDelivery → Delivered` 不同——这是一个**流程终结**的决定，语义是"这份文档是最终版，可以使用了"。这个判断不应该由 AI 自主做，应该需要用户点头。

`delivery-approval.json` 的存在，就是在 hook 层面强制要求这个"人工点头"动作必须发生——如果文件不存在，hook 直接拒绝推进到 Delivered，无论 AI 怎么写 state.json。

---

## 但这个设计有一个漏洞

`delivery-approval.json` 本身也是 AI 写的。用户说"同意交付"，AI 写这个文件，再写 state.json。

如果 AI 不经用户授权，直接写 `delivery-approval.json` + `state.json`，hook 照样放行。

所以这个文件**不能阻止恶意或失控的 AI**，只能阻止"遵守 SKILL 规则的 AI 走捷径"。

---

## 结论

`delivery-approval.json` 的存在价值：

- **有价值**：作为一个显式的流程节点，提醒 AI 和用户"这里需要一个明确的授权动作"，防止 SKILL 流程中的意外跳步
- **没有绝对安全价值**：无法阻止 AI 绕过它

如果你认为 SKILL 流程本身足够清晰（SKILL 明确规定"必须用户说交付才能写 Delivered"），那这个文件可以去掉，把人工确认节点内化到 SKILL 指令里。如果你想在 hook 层面保留一道可见的门槛，就保留它，改成 `.md` 格式。

你倾向于哪种？

Turn:1～37


<!-- DDM:TURN_SEP:v1 -->


## User（Turn 41）

1 可以保留，但是文件名上要显示的命名，描述2个状态文件的意图，现在让人误解。 
2 另外，你觉得是否还需要探索一下，如何使用hook完成。我还是想看看，有没有更好的架构设计。


<!-- DDM:TURN_SEP:v1 -->


## AI

---

## 问题 1：文件命名

当前命名的问题：`state.json` 太通用，`delivery-approval.json` 描述了操作但不显示谁负责。

建议按"谁写 + 做什么"命名：

| 当前 | 建议 | 意图说明 |
|------|------|---------|
| `state.json` | `workflow-state.md` | AI 管理的流程进度 |
| `delivery-approval.json` | `human-delivery-gate.md` | 人工设置的交付门禁 |

两个文件的责任人和性质从命名上就能区分。

---

## 问题 2：Hook-only 架构探索

继续这条路，引入第三个方案：**信号文件（Signal File）**。

### 核心思路

AI 不写状态文件，只写"意图信号文件"。Hook 捕获这个写入，执行状态迁移，然后决定是否保留信号文件。
AI 写 signal-to-evaluating.md
  → Hook 拦截
  → Hook 验证：Drafting → Evaluating 合法？
  → Hook 写 workflow-state.md（Python subprocess，Plan 模式不限制）
  → Hook 返回 Allow（信号文件作为迁移日志保留）
        或 Deny（迁移不合法，信号文件不落盘）
SKILL 只需要知道：在什么时机写哪个信号文件，不需要知道 `workflow-state.md` 的格式。

### 三个方案对比

| | A：直接写 workflow-state.md | B：transition.py via Shell | C：信号文件 + Hook |
|--|--------------------------|--------------------------|-----------------|
| Plan 模式 | ✅ .md 可写 | ✅ Shell 可调 | ✅ .md 可写 |
| SKILL 需要知道 | 状态文件格式 | 脚本接口名称 | 信号文件名称 |
| Hook 职责 | 校验 | Fallback 防线 | 校验 + 执行迁移 |
| 额外文件 | 无 | 需要 transition.py | 信号文件（作日志） |
| 状态对 AI 透明 | ❌ AI 需感知格式 | ⚠️ AI 需感知脚本 | ✅ AI 只写信号 |
| 架构清晰度 | 高 | 高 | 最高，但最复杂 |

### 方案 C 的残留问题

信号文件被 Hook Allow 后会落盘，目录里会积累：
signal-to-evaluating.md
signal-to-ready-for-delivery.md
signal-to-drafting.md   ← 回退记录
这实际上是一个**迁移日志**，不是脏数据。可以接受，也可以由 Hook 在 postToolUse 阶段清理（如果 Cursor 支持）。

---

## 我的判断

三个方案都能工作。区别在于**复杂度与 SKILL 无感程度的权衡**：

- 追求**最简实现**：选 A（直接写 workflow-state.md）
- 追求**SKILL 完全无感于状态格式**：选 C（信号文件）
- B 是中间路线，不如 A 简单，也不如 C 优雅

你更倾向于哪个方向——简单优先，还是 SKILL 无感优先？

Turn:1～38


<!-- DDM:TURN_SEP:v1 -->


## User（Turn 42）

这三个方案都没有解决核心问题，SKILL不感知状态迁移，有没有可能把SKILL的流程，设计成状态自包含了状态迁移？


<!-- DDM:TURN_SEP:v1 -->


## AI

这个问题触及了真正的设计突破点。

---

## 重新定义问题

前三个方案的共同假设：**状态机是独立的，SKILL 必须"通知"它。**

这个假设本身可能是错的。

换一个视角：**如果 SKILL 的执行过程本身就是状态机，文档的存在关系就是状态，那就不需要独立的状态管理了。**

---

## 文档驱动状态（Document-Driven State）

状态不存储在 `workflow-state.md` 里，而是**由文档的存在关系隐含**：
会话目录 .cache/lulu-dev-workflow/product/<conv_id>/
  └── product-doc.md          → 存在 = 起草阶段完成
  └── pdqa-review.md          → 存在 = 评估阶段完成
  └── human-delivery-gate.md  → 存在且通过 = 可以交付
Hook 的职责从"校验状态迁移"变为**"校验文档前置依赖"**：

| AI 写入 | Hook 检查 | 拦截条件 |
|---------|----------|---------|
| `pdqa-review.md` | `product-doc.md` 是否存在 | 无草稿不能评估 |
| `human-delivery-gate.md` | `pdqa-review.md` 是否存在且通过 | 无评估不能交付 |
| `product-doc.md` | 无前置要求 | 永远允许 |

---

## SKILL 流程完全自包含

SKILL.md 只描述一个线性过程，不涉及任何状态概念：
1. 读取 workflow-config.json，拉取产品模板
2. 起草产品文档 → 写入 product-doc.md
3. 使用 PDQA 框架逐项评估，记录问题 → 写入 pdqa-review.md
   （如发现问题：修订 product-doc.md，更新 pdqa-review.md）
4. 等待用户确认交付 → 写入 human-delivery-gate.md
5. 输出最终产品文档
SKILL 没有任何"迁移到状态 X"的指令。Hook 保证了步骤不能被跳过。

---

## 回退如何处理

不需要"回退迁移"——SKILL 直接覆写 `product-doc.md` 和 `pdqa-review.md`，Hook 对重复写入无限制（它只检查前置文件是否存在，不检查写了几次）。这是一个自然的迭代循环：

整个迭代过程对 Hook 透明，Hook 不需要处理回退。

---

## 与原方案对比

| | 原方案（状态机显式）| 新方案（文档驱动）|
|--|------------------|----------------|
| 状态文件 | workflow-state.md | 不需要 |
| 状态迁移指令 | SKILL 需要显式触发 | 不需要 |
| 回退路径 | 需要显式迁移 | 自然覆写 |
| Hook 职责 | 校验状态迁移白名单 | 校验文档前置依赖 |
| SKILL 复杂度 | 中（需感知状态） | 低（只写文档） |
| 跳步防护 | ✅ | ✅ |

---

## 剩余问题

只有一个：`human-delivery-gate.md` 的格式和 Hook 的解析方式——Hook 需要知道这个文件"通过了"是什么意思（比如检查文件存在即可，还是解析内容中的 `approved: true`）。

你认可这个方向吗？确认后更新架构文档，再进入实施。

Turn:1～39


<!-- DDM:TURN_SEP:v1 -->


## User（Turn 43）

我不认可这个方向，你又回到了我刚刚说的问题：使用文件状态来映射状态机的状态，首先语义不能完全映射。而且，后续状态扩展，无法维护。


<!-- DDM:TURN_SEP:v1 -->


## AI

先把我没理解到的点说清楚，再推进。

---

## 我理解你的拒绝

文件存在 → 状态映射有两个根本缺陷：
1. 语义不完整（文件存在 ≠ 阶段完成）
2. 状态扩展时需要修改 Hook 和文件命名约定，耦合很强

---

## 我没理解清楚的是

"SKILL 的流程，设计成状态自包含了状态迁移"——我尝试三次，每次都往文件映射方向走，说明我对你这句话的理解有偏差。

我目前理解到两种可能的含义，需要你确认：

**含义 A：SKILL 程序流本身就是状态机**

SKILL.md 的结构定义了状态和迁移。AI 按 SKILL 步骤执行，就是在运行状态机。不需要外部 state 文件，不需要 Hook 管理状态。状态只活在当前对话的上下文里。

缺点：跨对话/重启后无法恢复状态，无法跨会话防跳步。

**含义 B：状态管理对 SKILL 完全透明，由底层基础设施自动处理**

SKILL 正常描述流程步骤，不写任何状态相关指令。底层（Hook + 状态文件）自动感知 AI 正在执行哪个步骤，自动完成状态迁移。SKILL 完全不知道状态机的存在。

这个方向目前遇到的障碍是：底层如何"感知" AI 在哪个步骤，不依赖文件存在。

---

你说的是哪个含义？还是完全不同的第三种？

Turn:1～40


<!-- DDM:TURN_SEP:v1 -->


## User（Turn 44）

类似于：SKILL 程序流本身就是状态机。但是，SKILL 程序流执行自动的触发hook写入状态文件。而且这个SKILL 程序流，就是按照状态机的要求设计的。


<!-- DDM:TURN_SEP:v1 -->


## AI

现在我理解了。关键的三层设计：

---

## 核心设计
SKILL 流程（由AI遵守）
    ↓ AI 执行 SKILL 步骤，自然写出文档
Hook 感知（自动触发）
    ↓ 拦截写入，读取文档语义，自动推进状态文件
状态文件（由Hook管理，AI不写）
**AI 不写 state 文件，Hook 写。SKILL 不含状态指令，只含流程步骤。**

---

## 实现关键：文档语义而不是文件存在

Hook 不能靠文件路径（文件存在 = 状态），而是靠**文档的 YAML Frontmatter 语义字段**。

SKILL 规定每类文档的必须包含的字段：
markdown
---
doc_type: product-doc
phase_complete: true
---
# 产品文档...

markdown
---
doc_type: pdqa-review
all_issues_resolved: true
---
# PDQA 评估报告...
Hook 的判断逻辑（完全基于内容语义，不依赖文件名存在与否）：

| 检测到 | 当前状态 | Hook 动作 |
|--------|---------|---------|
| `doc_type: product-doc` + `phase_complete: true` | Drafting | 写 state → Evaluating |
| `doc_type: product-doc` + `phase_complete: true` | Evaluating | 写 state → Drafting（修订回退） |
| `doc_type: pdqa-review` + `all_issues_resolved: true` | Evaluating | 写 state → ReadyForDelivery |
| `doc_type: product-doc` + 无 `phase_complete` | 任意 | 不触发，迭代中 |
| `doc_type: pdqa-review` + 无 `all_issues_resolved` | 任意 | 不触发，评估中 |

---

## SKILL 的样子

SKILL 不说"迁移到状态 X"，只说流程步骤：
Phase 1 — 起草
  1. 读取 workflow-config.json 获取模板 URL
  2. 对照模板起草产品文档
  3. 完成后写入 product-doc.md，frontmatter 包含：
     doc_type: product-doc
     phase_complete: true

Phase 2 — 评估
  1. 读取 pdqa_url，逐项评估
  2. 记录问题，修订 product-doc.md（此时 phase_complete 不设置）
  3. 所有问题解决后，写入 pdqa-review.md，frontmatter 包含：
     doc_type: pdqa-review
     all_issues_resolved: true
SKILL 流程步骤 = 状态机阶段。`phase_complete: true` 是流程自然完成的标记，不是状态管理命令。

---

## 状态扩展如何维护

新增状态时：
1. 在 SKILL 增加新 Phase 步骤
2. 定义该 Phase 的 `doc_type` 和完成字段
3. 在 Hook 的判断表里增加一行规则
4. 在 transition-whitelist.json 增加迁移边

三处改动各自独立，互不耦合。

---

这个方向是你说的"SKILL 程序流执行自动触发 Hook 写状态"吗？确认后更新架构文档。

Turn:1～41


<!-- DDM:TURN_SEP:v1 -->


## User（Turn 45）

不是我想要的， 但是你说的，给我一点思路。我想到一个方案，把状态机的状态，放到product-doc.md的header里面。这个header可以是隐藏的文本，供AI读取。 流程驱动的时候，会写入product-doc.md，此时自动校验 product-doc.md 的header。 

但是，一个问题需要解决。 pdqa-review的文档生成之后，问题修复是在pdqa-review文档变更的。那是否状态机还需要2份写入到pdqa-review 的 header呢？ 

我觉得不需要，pdqa-review的内容，直接放到  product-doc.md 尾部，作为评估章节。我们一直在product-doc.md里面迭代。


<!-- DDM:TURN_SEP:v1 -->


## AI

这个设计思路很清晰，我来完整展开并补充细节。

---

## 新架构核心

**单文档驱动，状态写在文档 frontmatter 里，Hook 每次写入时校验。**

---

## 文档结构（贯穿整个工作流）
markdown
---
workflow_state: Evaluating        ← AI 每次写文档时更新此字段
conversation_id: ab926751-...
updated_at: 2026-05-17T09:00:00Z
---

# 产品文档：XXX 功能

[正文内容...]

---

## PDQA 评估

[评估阶段由 AI 填写，问题修复直接在正文更新，评估结论在此章节迭代]

---

## 交付确认

[ReadyForDelivery → Delivered 阶段，用户确认后 AI 写入此章节]
---

## Hook 的工作方式

只盯一个文件：`product-doc.md`。

每次 Write 拦截：
1. 读磁盘上现有的 `product-doc.md` frontmatter → 获取 **当前状态**
2. 读本次 Write 的内容的 frontmatter → 获取 **目标状态**
3. 查 `transition-whitelist.json`，校验迁移是否合法
4. 对于 `ReadyForDelivery → Delivered`，额外检查文档是否包含"交付确认"章节
5. Allow 或 Deny

**不再需要单独的 `workflow-state.md`**——状态就在文档里，文档即状态机。

---

## SKILL 流程的样子
Phase 1 — Drafting
  按模板起草产品文档正文
  写入 product-doc.md，frontmatter: workflow_state: Evaluating（宣告草稿完成）

Phase 2 — Evaluating  
  在 product-doc.md 末尾新增"PDQA 评估"章节
  逐项评估，发现问题在正文修订，在评估章节记录修订结果
  所有问题解决后，写入 product-doc.md，frontmatter: workflow_state: ReadyForDelivery

Phase 3 — Delivered（等待用户确认）
  用户说"同意交付"后，新增"交付确认"章节
  写入 product-doc.md，frontmatter: workflow_state: Delivered
**SKILL 没有任何"调用状态迁移"的指令，只有"写文档"的指令。**

---

## 你提的问题：PDQA 章节是否需要独立状态

不需要。

PDQA 评估和问题修订都在同一份 `product-doc.md` 里迭代，状态推进的触发点是 frontmatter 的 `workflow_state` 变化，与文档有多少章节无关。

---

## 还剩一个需要确认的细节

文档的 frontmatter 是"可见"还是"隐藏"？

- **YAML frontmatter（`---` 包裹）**：标准 Markdown 约定，AI 自然读取，渲染时部分工具会隐藏
- **HTML 注释（`<!-- -->`）**：渲染完全隐藏，但 AI 读取原始文本时可见

你倾向于哪种格式？

Turn:1～42


<!-- DDM:TURN_SEP:v1 -->


## User（Turn 46）

你没有质疑我的方案的问题，product-doc.md文档，如何识别目前的文档修改，是哪个状态流转？ 还是需要有人写入状态。

我感觉我这个方案不行。


<!-- DDM:TURN_SEP:v1 -->


## AI

你的自我诊断是对的，但原因需要说清楚。

---

## 根本问题

**状态迁移需要意图，意图无法从产物中推断。**

AI 写 `product-doc.md`，这个动作本身无法告诉 Hook"这次写是 Drafting 完成了"还是"这次写是 Evaluating 中的一次修订"。Hook 看不到 AI 的意图，只看到文件写入。

**所以，无论用什么形式，AI 都必须显式表达状态迁移意图。** 这不是设计缺陷，是信息论的约束——Hook 需要的信息（意图）不在产物里，必须有人写进去。

---

## 你一直在追求的东西，本质上做不到

"SKILL 不感知状态迁移"的前提，是底层能自动推断 AI 的意图。但推断意图需要语义理解，这超出了 Hook 的能力范围。

所有走到这里的方案都有同一个问题：

| 方案 | AI 写了什么 | 本质上 |
|------|-----------|--------|
| 写 state.json | 状态文件 | AI 写状态 |
| 调用 transition.py | Shell 命令 | AI 写状态 |
| 写信号文件 | signal-to-evaluating.md | AI 写状态 |
| frontmatter 字段 | workflow_state: Evaluating | AI 写状态 |

**形式不同，本质相同：都是 AI 在某个地方表达"我要迁移状态了"。**

---

## 那可以接受的最小侵入是什么

既然 AI 必须表达意图，问题变成：哪种形式对 SKILL 最自然、最不像"额外的状态管理步骤"？

Frontmatter 仍然是最优解：

- 写文档时顺带更新 frontmatter，不是一个单独的"状态迁移命令"
- SKILL 说"起草完成，写文档，frontmatter 设置 `workflow_state: Evaluating`"——这是文档元数据，不是系统调用
- 跟"调用 transition.py" 相比，它内嵌在文档里，感觉上更像"完成文档"而不是"操作状态机"

**SKILL 的侵入程度：**
- `transition.py`：显式系统操作，SKILL 感知最强
- 信号文件：额外文件，感觉冗余
- **frontmatter 字段**：文档元数据，感知最弱，最自然

---

## 结论

追求"完全 SKILL 无感"是死路，信息论不允许。  
接受"最小感知"：frontmatter 方案是目前最接近你目标的可行设计。

是否接受这个结论，推进 frontmatter 方案？

Turn:1～43


<!-- DDM:TURN_SEP:v1 -->


## User（Turn 47）

我们回到 Turn38 的讨论，还是使用 A：直接写 workflow-state.md 来做吧。 你复述一下这个方案


<!-- DDM:TURN_SEP:v1 -->


## AI

---

## 方案 A：直接写 workflow-state.md

### 核心原则

- AI 通过写 `workflow-state.md` 来表达状态迁移意图
- Hook 拦截这次写入，校验迁移合法性，允许或拒绝
- `workflow-state.md` 是 Markdown 格式（YAML frontmatter），Plan 模式可写
- 产品文档（`product-doc.md`）和评估文档（`pdqa-review.md`）独立存在，不承载状态

---

### 文件结构
.cursor/lulu-dev-workflow/
  workflow-config.json              ← 项目级配置，模板 URL

.cache/lulu-dev-workflow/product/<conv_id>/
  workflow-state.md                 ← AI 写，Hook 校验（Plan 模式可写）
  product-doc.md                    ← AI 写，产品文档正文
  pdqa-review.md                    ← AI 写，PDQA 评估记录
  human-delivery-gate.md            ← AI 代表用户写，交付门禁
---

### workflow-state.md 格式

---

### Hook 工作逻辑

拦截对 `workflow-state.md` 的 Write/Edit：

1. 读磁盘现有 `workflow-state.md` → 获取 `current_state`（不存在则默认 `Drafting`）
2. 解析本次写入内容的 `current_state` → 获取目标状态
3. 查 `transition-whitelist.json`，校验迁移合法性
4. 若目标状态为 `Delivered`，额外检查 `human-delivery-gate.md` 是否存在
5. Allow 或 Deny

---

### 状态机

---

### SKILL 流程
Phase 1 — Drafting
  1. 读取 workflow-config.json，拉取产品模板
  2. 起草产品文档 → 写 product-doc.md
  3. 写 workflow-state.md，current_state: Evaluating

Phase 2 — Evaluating
  1. 拉取 pdqa_url，逐项评估
  2. 发现问题 → 修订 product-doc.md，更新 pdqa-review.md
  3. 所有问题解决 → 写 workflow-state.md，current_state: ReadyForDelivery

Phase 3 — ReadyForDelivery
  1. 等待用户确认交付
  2. 用户确认 → 写 human-delivery-gate.md
  3. 写 workflow-state.md，current_state: Delivered
---

### 与原 state.json 方案的区别

| | 原方案 | 方案 A |
|--|--------|--------|
| 文件格式 | `.json` | `.md`（YAML frontmatter）|
| Plan 模式可写 | ❌ | ✅ |
| Hook 解析方式 | `json.loads()` | 正则解析 frontmatter |
| 其他逻辑 | 完全一致 | 完全一致 |

---

这是当前选定方案。确认后更新架构文档，然后进入实施。

Turn:1～44


<!-- DDM:TURN_SEP:v1 -->


## User（Turn 50）

之前的Reset指令还有吗？ 场景：一个对话多次产品文档编写，需要清除缓存状态，也就是从任何状态切换到 Drafting。


<!-- DDM:TURN_SEP:v1 -->


## AI

`start` 指令保留了，这正是它的核心用途。

`start.py` 通过 Python 直接写 `workflow-state.md`，**不经过 Write 工具，不触发 hook**，所以可以从任意状态强制重置到 `Drafting`：

SKILL.md 中也明确标注了：

> `Can be run again at any time to reset state.`

状态文件路径已经更新为 `.cache/lulu-dev-workflow/product/<conv_id>/workflow-state.md`，但行为不变——覆盖写入 `current_state: Drafting`，绕过迁移白名单。

Turn:1～47


<!-- DDM:TURN_SEP:v1 -->


## User（Turn 51）

本次优化，涉及到 Turn28 说的8个问题的哪一些？ 哪些没有修复？


<!-- DDM:TURN_SEP:v1 -->


## AI

逐一对照。

---

## 8 个问题 vs 本次优化

| # | 问题 | 本次修复？ | 说明 |
|---|------|-----------|------|
| 1 | Turn22 AI 没进入 SKILL 流程，SKILL 需要如何优化 | ⚠️ 未修复 | 根因是 SKILL.md 的 `description` 触发词不包含"新功能/产品需求"等日常语言，AI 无法识别用户意图。本次未改触发词。 |
| 2 | state.json 写不进去但流程继续 | ✅ 修复 | 改为 `workflow-state.md`（Markdown），Plan 模式全程可写，hook 可正常拦截 |
| 3 | 产品文档没有写到 `.cache` | ✅ 修复 | SKILL.md 现在明确规定产出文档写到 `.cache/lulu-dev-workflow/product/<conv_id>/` |
| 4 | PDQA 审查记录没有写到 `.cache` | ✅ 修复 | SKILL.md 明确要求写 `pdqa-review.md`，并在评估过程中持续更新 |
| 5 | 评估问题应弹出选择框（AskQuestion），实际用的文本提示 | ❌ 未修复 | SKILL.md 没有要求使用 AskQuestion 工具，问题呈现方式未规范 |
| 6 | 问题解决过程没有更新审查记录文档 | ⚠️ 部分修复 | SKILL.md 加了"record all findings and resolutions in pdqa-review.md"，但没有逐步更新的强制要求 |
| 7 | 全部问题解决后没有更新产品文档 | ⚠️ 部分修复 | SKILL.md 加了"push fixes back into product-doc.md"，但没有"解决完每个问题后立即更新"的明确节奏 |
| 8 | 评估通过后没有推进到 ReadyForDelivery | ✅ 修复 | 状态可写后，状态迁移流程可以正常推进 |

---

## 还需要修复的 3 个问题

**问题 1（高优先级）：SKILL 触发词**

`description` 字段需要增加触发词，让 AI 在用户提出功能需求时能自动识别：
yaml
description: >-
  Use when: 产品需求, 新功能, 功能设计, 产品文档, PRD, spec,
  product-doc workflow, 产品文档流程, ...
**问题 5（高优先级）：AskQuestion 工具**

SKILL.md Evaluating 阶段需要明确：每个评估问题用 `AskQuestion` 工具呈现给用户，不用文本列表。

**问题 6/7（中优先级）：文档更新节奏**

SKILL.md 需要明确：
- 每解决一个问题 → 立即更新 `product-doc.md` + `pdqa-review.md`
- 不允许攒到最后一次性更新

---

是否现在继续修复这三个，还是另开一个对话处理？

Turn:1～48


<!-- DDM:TURN_SEP:v1 -->


## User（Turn 52）

1 是的，直接处理。
2 同步， 看下架构文档是否需要调整


<!-- DDM:TURN_SEP:v1 -->


## AI

先读取当前文件，确认修改范围。


<!-- DDM:TURN_SEP:v1 -->


## User（Turn 54）

1 架构文档，给我保存到：https://github.com/lulufoo/ai-software-dev/tree/main/lulu-dev-workflow-template  目录。
2 卸载本地的SKILL，以及当前项目的init残留。
3 重新安装SKILL，并在当前项目init。


<!-- DDM:TURN_SEP:v1 -->


## AI

三个任务并行准备。先读取 hooks.json 确认当前状态，同时查看本地安装的文件结构。


<!-- DDM:TURN_SEP:v1 -->


## User（Turn 55）

执行：dtd_distill_overview，不写文件。


<!-- DDM:TURN_SEP:v1 -->


## AI

（已剥离推导性独白，本回合无可保留的 AI 正文）


<!-- DDM:TURN_SEP:v1 -->


## User（Turn 56）

哪些 Turn 和方案推进和优化无关的，你给我列出来。


<!-- DDM:TURN_SEP:v1 -->


## AI

从概要逆推，过滤掉与"方案推进和优化"直接相关的 Turn，剩下的是：

---

**与方案推进无关的 Turn：**

| Turn | 内容 | 说明 |
|------|------|------|
| Turn 7 | "还有什么问题吗？审查一下" | 触发审查，本身无实质内容 |
| Turn 10 | "卡死了" | 流程卡住，重启推进 |
| Turn 15 | `/product start` | 纯执行指令 |
| Turn 17 | "可以，请执行" | 确认执行 |
| Turn 18 | "SKILL 提交了嘛" | 确认状态查询 |
| Turn 19 | "是的，处理" | 确认执行 |
| Turn 21 | `/product start`（再次） | 纯执行指令 |
| Turn 25 | "A" | PDQA 选项选择 |
| Turn 26 | "A" | PDQA 选项选择 |
| Turn 27 | "B" | PDQA 选项选择 |
| Turn 28 | "是的" | 确认 |
| Turn 48 | "是的，更新架构方案吧" | 确认执行 |
| Turn 49 | "是的，实施然后推送 GitHub" | 确认执行 |
| Turn 53 | "执行吧" | 确认执行 |

---

共 **14 个 Turn** 属于纯执行/确认/状态查询，与方案内容无关。

Turn:1～53
