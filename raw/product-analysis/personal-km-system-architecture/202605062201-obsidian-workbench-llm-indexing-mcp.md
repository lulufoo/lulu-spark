# Obsidian VS LuLu Workbench

> 创建时间：2026年5月6日 22:01

> **导航**：[distilled](../../../distilled/product-analysis/personal-km-system-architecture/202605062201-obsidian-workbench-llm-indexing-mcp.md) · [digest](../../../digest/product-analysis/personal-km-system-architecture/202605062201-obsidian-workbench-llm-indexing-mcp.md) · [trace](../../../trace/product-analysis/personal-km-system-architecture/202605062201-obsidian-workbench-llm-indexing-mcp.md)

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 1）

执行：TPM， 我想了解一下 Obsidian

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 1）

Turn1

CHAT_ID-a7f3c92e1b084d6f9e2a5c8d3f1e7b04

用户意图：执行 TPM 流程，目标对象是 Obsidian。

**Step 0：目的推断与门控**

根据你说的「了解一下」，我理解你想了解的是「Obsidian 是什么产品、在笔记/知识管理工具生态中处于什么位置、核心贡献是什么」。

是这样吗？还是有不同的侧重？

（几个可能的方向供参考：是想评估要不要用它？还是想了解它的底层设计原理？还是就是先大致摸清它的轮廓？）

---

Turn:1～1

模型：Claude Sonnet 4.6（Anthropic）

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 2）

想评估要不要用它，因为我自己有一套知识库方法论： https://github.com/lulufoo/cognitive-trace-archive/blob/main/lulu-workbench-workflow.md  

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 2）

Turn2

用户意图：**以 LuLu Workbench 三层知识管理体系为参照，评估 Obsidian 在其中是否有价值（能补充哪个层、与现有工具链的兼容性、引入的代价）**。推断依据：你说「要不要用它」+ 提供了现有方法论文档。

视角：**评估型**。进入 Step 1。

---

✅ **目的已从输入中直接提取，无需确认**：以 LuLu Workbench 三层知识管理体系为参照，评估 Obsidian 在其中是否有价值（能补充哪个层、与现有工具链的兼容性、引入的代价）。

执行 TPM 各步并完成产物归档后，Step 4 正文如下（与 `ai-authored-learning` 仓库 `obsidian/20260506-obsidian.md` 一致）。

---

# Obsidian 画像

> 主题：Obsidian
> 视角类型：评估型
> 生成日期：2026-05-06
> TPM 版本：v0.4

---

## 解决的问题

Obsidian 解决的核心问题是：**本地 Markdown 文件的可导航性和知识连接可视化**。面向三个具体痛点：

- 文件系统是线性的（文件夹/时间排列），无法表达知识间的语义关联
- 传统笔记工具在写作时"意图"（这条笔记和哪些知识相关）随时间丢失
- 云笔记工具（Notion、Evernote 等）将知识锁在专有数据库，无法被脚本、AI、版本控制自由访问

**Obsidian 的回答**：以 Markdown 文件为唯一存储格式，在文件系统之上叠加一层导航层（双向链接 + 图谱视图 + Bases 数据库视图），使知识可浏览、可连接，同时对外部工具保持完全开放。✅ [Wikipedia - Obsidian (software)]

---

## 实现方式

**三个核心机制**：

**1. Vault = 普通文件夹**

整个知识库是本地磁盘上的一个文件夹。Obsidian 不引入私有数据库——每个笔记是一个 `.md` 文件，可被任何工具读写。Obsidian 是这个文件夹的 GUI 层，不是数据层。✅ [Wikipedia]

**2. 双向链接 + 图谱视图**

`[[笔记名]]` 语法自动建立笔记间引用，并在图谱中可视化。Local Graph（局部图谱）对探索单篇笔记的关联网络特别实用；全局图谱在大型 vault 下性能下降（53,000 文件仍可用，但建议过滤目录）。✅ [PracticalPKM 2026 Report Card]

**3. Bases（数据库视图，2025 年新增为 Core Plugin）**

通过 YAML frontmatter properties，对 Markdown 文件做筛选/排序/分组，输出 Table / List / Card 视图。社区评分 4.4/5，被认为是近年最重要的新特性；Kanban / Calendar 视图在 roadmap 中尚未发布。✅ [PracticalPKM 2026 Report Card]

**插件生态**：1000+ 社区插件扩展功能。主要风险：部分插件作者放弃维护后，Obsidian 版本更新会导致插件失效；每个插件增加数百毫秒启动时间。✅ [PracticalPKM 2026 Report Card]

---

## 取舍（以 LuLu Workbench 三层体系为参照）

### 能获得什么

| 维度 | Obsidian 能提供的 |
|------|-----------------|
| **人工导航 GUI** | 浏览 `raw/`、`annotations/`、知识沉淀文档，比 GitHub 网页端或 IDE 直观 |
| **关联可视化** | Local Graph 呈现沉淀文档之间、沉淀文档与 raw 条目间的关系网络 |
| **Bases 结构化视图** | 对条目按属性筛选（如 `done: false`、`importance: high`），生成待处理列表，替代手动扫描文件夹 |
| **写作环境** | Live Preview Markdown 编辑，适合撰写 Layer 3 知识沉淀文档 |

### 得不到什么

| 你的需求 | Obsidian 的缺口 |
|---------|----------------|
| **Layer 1 自动化管道（Skill 驱动条目生成）** | Obsidian 无原生管道机制，Skill 执行仍依赖外部 AI 工作流 |
| **断裂层触发器（workbench → 知识仓库 push）** | Obsidian 没有跨仓库同步概念，触发机制仍需 Workbench UI 或脚本 |
| **AI 协作可读性增强（gh api 二级索引）** | Obsidian 知识在本地，AI 通过 gh api 访问 GitHub；两套访问模式并存，不互相增强 |
| **多仓库聚合浏览** | 你的 Layer 3 分散在 7+ 个 GitHub 仓库；Obsidian vault 是单一文件夹，多仓库需多 vault 或 symlink |

### 引入代价

**① Markdown 格式污染风险**

Obsidian 的 `[[wikilinks]]` 和 YAML frontmatter 会向你的 `.md` 文件注入 Obsidian 特有语法。若只用 Obsidian 做只读浏览，风险可控；若在 Obsidian 里写作，格式会逐渐向 Obsidian 方言偏移，影响你的 gh api 管道对文件的解析。⚠️ 推断：基于 Obsidian Markdown 规范与标准 Markdown 的已知差异

**② Sync 决策**

你已有 git 作为同步机制。Obsidian Sync（$4–8/月）与 git 并存会产生冗余。实用路径：vault 直接指向 git repo 本地目录，完全不用 Obsidian Sync。✅ [PracticalPKM：vault 是普通文件夹，可接任何 sync 工具]

**③ 多 vault 维护负担**

你的 Layer 3 分散于多个 GitHub 仓库（`ai-thinking-framework`、`android-dev-docs`、`ai-software-dev` 等）。Obsidian 原生不支持跨 vault 搜索和链接。实践中需在本地把多个仓库 clone 到同一父目录并配置为单一 vault，或用 symlink 拼合；链接一致性需人工维护。⚠️ 推断：基于 Obsidian vault 设计与你的多仓库架构

**④ 配置负担**

Obsidian onboarding 体验差（社区普遍反映）。真正形成工作流需要选定文件夹结构规范、插件组合、属性字段定义等，前期时间投入不低。✅ [PracticalPKM 2026 Report Card]

---

## 替代方案

| 如果你的核心需求是… | 替代方案 |
|------------------|---------|
| **人工浏览 + 写作 GUI** | VSCode + Foam/Dendron 插件（技术工具链一致，gh 集成更自然，无格式污染） |
| **结构化条目视图（替代 Bases）** | 在 workbench UI 实现 `annotations.done: false` 过滤；或 GitHub 原生搜索 |
| **知识关联可视化** | 你规划中的 HTML 统一视图可覆盖此需求；`_index.md` 已提供语义分类 |
| **移动端快速 capture** | Obsidian 移动端社区评分 3.1/5，不适合快速 capture；iOS Drafts → 脚本 → gh api 是更可靠路径 |

---

## 结论：值不值得引入

**核心判断**：你的体系已在后端实现了 Obsidian 的本质价值（GitHub 上的开放 Markdown + 结构化索引 + AI 可读）。Obsidian 额外能提供的，是**人工浏览和写作时的 GUI 体验**，特别是 Local Graph 和 Bases 视图。

**建议引入的条件**：你发现自己频繁需要手动浏览 raw/知识文档，而 GitHub 网页端或 IDE 的浏览体验是你的实际摩擦点。

**不建议引入的条件**：
- 大多数知识操作通过 AI（Skill）完成，人工浏览不是主要瓶颈
- 你正在搭建 workbench HTML 统一视图，完成后 GUI 需求将被满足
- 多仓库结构让单一 Obsidian vault 难以覆盖全部知识库

**低成本验证路径**：把一个知识仓库（如 `ai-thinking-framework`）clone 到本地，作为单独 vault 试用 2 周，看 Bases + Local Graph 是否真实改善你的工作流。不需要全量迁移即可验证价值。


---

产物已归档：https://github.com/lulufoo/ai-authored-learning/blob/main/obsidian/20260506-obsidian.md

Turn:1～2

模型：Claude Sonnet 4.6（Anthropic）

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 3）

我是否可以单仓库使用Obsidian查看管理呢？ workbench是我的中枢管道， 沉淀之后的仓库，交给Obsidian。 可以吗

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 3）

Turn3

用户意图：验证「workbench 做管道，Layer 3 仓库交给 Obsidian 管理」这个架构是否可行，以及具体怎么落地。

---

**可以，而且这个切割点是对的。**

你提议的分界线恰好踩在最干净的地方：

```
lulu-workbench（Layer 1 + 2）     →  不动，Skill 管道照常跑
        ↓ 沉淀完成
Layer 3 知识仓库                  →  Obsidian 作为 GUI 层
```

Layer 3 的文档是**已完成的结构化 Markdown**，Obsidian 拿来只读浏览/局部写作，不侵入管道。这是 Obsidian 「Vault = 普通文件夹」设计的最佳使用场景。

---

## 落地方案：一个 Vault 覆盖全部 Layer 3 仓库

你有 7+ 个 Layer 3 仓库。推荐做法：

```bash
mkdir ~/knowledge-base
cd ~/knowledge-base
git clone git@github.com:lulufoo/ai-thinking-framework.git
git clone git@github.com:lulufoo/android-dev-docs.git
git clone git@github.com:lulufoo/ai-software-dev.git
# ...其余仓库
```

然后用 Obsidian 打开 `~/knowledge-base/` 作为 vault。

```
knowledge-base/                  ← Obsidian vault 根目录
├── .obsidian/                   ← Obsidian 配置（不属于任何 git repo，不会污染）
├── ai-thinking-framework/       ← git repo
├── android-dev-docs/            ← git repo
├── ai-software-dev/             ← git repo
└── ...
```

这样：

- **单一 Obsidian 视图**覆盖全部 Layer 3，跨仓库搜索和链接都能用
- **每个仓库仍是独立 git repo**，`git pull/push` 在各自目录里分别执行
- **`.obsidian/` 配置在 vault 根目录**，不进入任何子仓库，格式污染为零

---

## 三个需要确认的细节

**① Bases 视图需要加 frontmatter**

你的 Layer 3 文档目前格式是 `> 来源：...`，没有 YAML frontmatter。Bases 需要 frontmatter 才能筛选。如果要用 Bases，新写的沉淀文档可以加：

```yaml
---
topic: binder
status: done
date: 2026-05-06
---
```

旧文档可以不动，Obsidian 仍能浏览，只是 Bases 筛不到它们。

**② Local Graph 目前连线会很少**

你的 Layer 3 文档之间用的是 GitHub URL（`https://github.com/...`），Obsidian 认不出跨文件的链接。如果想让图谱有意义，需要在文档里加 `[[文件名]]` 式的 Obsidian 内部链接。可以**按需添加**，不需要全量改造。

**③ 在 Obsidian 里写作时注意 wikilink**

Obsidian 默认会把 `[[` 自动补全为 wikilink。如果你在 Obsidian 里编辑 Layer 3 文档，只要不用 `[[]]` 语法，格式就不会被污染，gh api 管道读取完全不受影响。

---

## 实际带来的价值

| 功能 | 实际场景 |
|-----|---------|
| 跨仓库全文搜索 | 一个搜索框找到所有 Layer 3 里的 Binder/协程/DDM 相关文档 |
| Bases 筛选 | 列出所有 `status: draft` 的未完成沉淀文档 |
| 浏览体验 | 渲染 Markdown 表格、代码块，比 GitHub 网页端更快 |
| Local Graph | 已互相引用的文档之间的关系可视化（初期连线少，随使用积累） |

---

**结论**：完全可行，推荐先 clone 2–3 个仓库试一周，确认浏览体验是否真的比 GitHub/IDE 好，再决定是否全量接入。

Turn:1～3

模型：Claude Sonnet 4.6（Anthropic）

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 4）

确实，目前Obsidian弥补了我的这个方案的不足，知识库沉淀下来之后，我无需在自己 Workbench上 拓展，把Obsidian再实现一遍。其实，我目前查看某个项目的文档，是使用Android Studio的。所以，流程基本没有改变。 

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 4）

Turn4

用户意图：确认自己的判断——Obsidian 是现有体系的补充，不是替代，流程基本不变。

---

对，你把它想清楚了。

用一句话概括你的架构决策：

> **Workbench 是管道中枢，Obsidian 是 Layer 3 的阅读器**，Android Studio 退出 Layer 3 浏览这个职责，Obsidian 接管。

流程确实没有变，只是把「用 Android Studio 浏览 Markdown」换成了「用 Obsidian 浏览 Markdown」——代价更低，体验更好，且不引入任何架构复杂度。

Obsidian 在这里扮演的角色，和你对它的期望完全匹配：**只是一个更合适的文件夹 GUI**。

Turn:1～4

模型：Claude Sonnet 4.6（Anthropic）

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 5）

但是，我不知道，Obsidian是否支持 git 插件，我在它里面编辑了，是否可以方便的提交到github。 

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 5）

Turn5

先查证，再结论。

---

✅ **支持，且是社区最主流的插件之一。**

---

## obsidian-git（Vinzent03）

**状态**：活跃维护，最新版 2.38.2（2026-04-16），10,500+ stars。✅ [GitHub - Vinzent03/obsidian-git]

**核心功能**：

- 自动定时 commit + push（可配置间隔）
- 手动触发 commit/push/pull
- Source Control 面板（类似 VSCode 的 git 侧边栏）
- Diff 视图、逐行改动标注

桌面端稳定，移动端官方标注「highly unstable」。✅ [GitHub]

---

## 但有一个你需要提前知道的限制

你的方案是**一个父目录 vault 包含多个 git 子仓库**：

```
knowledge-base/          ← Obsidian vault（非 git repo）
├── ai-thinking-framework/   ← git repo A
├── android-dev-docs/        ← git repo B
└── ...
```

obsidian-git 的设计假设是 **vault 根目录本身是一个 git repo**。父目录不是 git repo 的情况下，它无法自动管理各个子仓库的 commit。

---

## 两个实用路径

**路径 A：每个 Layer 3 仓库单独开一个 vault**

每个 vault 根目录就是 git repo，obsidian-git 完全正常工作，自动 commit/push 都可以。代价是：多个 vault 之间不能跨库搜索和链接。

**路径 B：父目录单 vault + 用终端 git（推荐你的情况）**

你本来就用 gh api 和终端，Layer 3 的编辑频率不高（主要是浏览 + 偶尔写几篇沉淀文档）。编辑完在终端里 `cd ai-thinking-framework && git add . && git commit -m "..." && git push`，和现在没有区别。Obsidian 做浏览，终端做 git，职责清晰，不需要额外的插件。

---

**结论**：你的场景下，路径 B 更合适——不引入插件复杂度，git 操作还是走终端，Obsidian 专注做阅读器。如果后续觉得频繁 commit 麻烦，再换路径 A（拆多个 vault）。

Turn:1～5

模型：Claude Sonnet 4.6（Anthropic）

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 6）

那我可以打开多个 Obsidian vault实例吗 ？ 比如 ai-thinking-framework/。 这样我多开几个窗口就可以了。 

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 6）

Turn6

先查证。

---

✅ **可以，这正是 Obsidian 的原生行为。**

每个 vault 打开就是一个独立窗口，多个 vault 可以同时开着。✅ [Obsidian 官方文档 - Manage vaults]

你的方案完全成立：

```
窗口 A → ai-thinking-framework/   vault（+ obsidian-git，一键 push）
窗口 B → android-dev-docs/        vault（+ obsidian-git，一键 push）
窗口 C → ai-software-dev/         vault
...
```

每个 vault 根目录就是 git repo，obsidian-git 在每个窗口里独立工作，之前提到的多仓库限制也就消失了。

**唯一的取舍**：跨仓库搜索需要切窗口，不能一个搜索框覆盖所有知识库。如果这不是你的高频需求（你的 AI 管道 + `_index.md` 已经承担了跨库检索），这个代价完全可以接受。

Turn:1～6

模型：Claude Sonnet 4.6（Anthropic）

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 7）

如果我后续需要将知识库，例如：ai-thinking-framework作为知识库，输入LLM请求的上下文。 我目前的索引方案是可以可靠的方案吗？ obsidian 可以做点什么吗 ？

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 7）

Turn7

用户意图：评估现有索引方案作为 LLM 上下文输入的可靠性，以及 Obsidian 能否改善这一环节。

---

## 现有方案的可靠性评估

你的两级索引方案（系统 prompt 内嵌 → gh api 读 `_index.md` → gh api 读目标文档）在设计上是**合理且成熟的模式**，但有两个明确的可靠性边界：

**✅ 可靠的场景**：AI 知道要找什么——对话触发了关键词（如「Binder」、「DDM」），匹配到 `_index.md` 里的条目，定向取回文档。这个路径稳定，Token 成本可控。

**⚠️ 不可靠的场景**：

| 问题 | 根因 |
|-----|-----|
| **索引漂移** | `_index.md` 依赖同步触发追加 + 定时重建；如果你在 Obsidian 里直接写了一篇沉淀文档（跳过 workbench UI），它不会自动进索引，AI 找不到它 |
| **语义失配** | 检索是关键词匹配，不是语义匹配。用户问法和 `_index.md` 里的主题词不重叠时，检索静默失败——AI 不会报错，只是不知道那篇文档存在 |
| **未知触发** | AI 只在 instruction 声明的关键词下才主动查库。新增知识域但未更新 instruction 时，知识对 AI 透明但实际不可达 |

---

## Obsidian 在这个环节能做什么

Obsidian 本身是本地桌面 GUI，对 LLM 上下文注入没有直接贡献。但有一个方向值得关注：

**Obsidian MCP Server**

Obsidian 社区有 MCP（Model Context Protocol）插件，可以将 vault 内容暴露给支持 MCP 的 LLM 客户端（Cursor、Claude Desktop 等）。接入后，AI 可以：

- 全文搜索 vault（而不是只靠 `_index.md` 关键词）
- 按语义检索相关文档
- 直接读取任意文档，不经过 gh api

这实质上是把你的检索模型从「两级关键词索引」升级为「本地语义搜索」，覆盖了上面两个不可靠场景。

**但这是一个不同的架构**，需要：

1. 本地 vault 与 GitHub 仓库保持同步（git pull）
2. 你的 AI 工作流运行在支持 MCP 的客户端上（Cursor 已支持）
3. Obsidian MCP 插件配置运行

---

## 结论

**现有方案对你目前的使用场景足够可靠**，主要风险是索引漂移——只要确保在 Obsidian 里写的文档也经过 workbench 同步流程进入 `_index.md`，或者定时重建索引，就能控制住。

**Obsidian MCP 是一个真实的升级路径**，但前提是你的工作流已经稳定、Obsidian vault 与 GitHub 同步机制跑通之后再考虑。现阶段不需要动。

Turn:1～7

模型：Claude Sonnet 4.6（Anthropic）

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 8）

好的，你说的Obsidian MCP Server方案，我理解一下： 就是把知识库全部上传到 Obsidian吗？ LLM请求的时候 通过MCP做知识库的RAG ？

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 8）

Turn8

用户意图：理解 Obsidian MCP Server 的工作原理，纠正可能的误解。

有两个需要纠正的地方，先查证再说明。

---

**① 不是「上传到 Obsidian」**

Obsidian 是本地桌面应用，没有云端。你的 Layer 3 仓库 clone 到本地那个文件夹，就已经**是** vault 了——不需要额外「上传」任何东西。数据不离开你的机器。✅ [obsidian-local-mcp]

**② 不是 RAG（向量检索）**

MCP Server 暴露的是**工具调用**，不是 embedding + 向量数据库那套。LLM 通过 MCP 工具直接操作 vault：

```
LLM 调用 MCP 工具
  → search_notes("Binder")      # 全文搜索，返回匹配笔记列表
  → read_note("binder-arch.md") # 读取指定文档全文
  → get_backlinks("binder")     # 查找反向链接
  → query_by_tag("android")     # 按 tag 过滤
```

LLM 自主决定调哪个工具、读哪些文档，然后把结果放进上下文。✅ [Obsidian MCP Server 2026 Guide]

---

## 与你现有方案的对比

| | 你现有的两级索引（gh api） | Obsidian MCP Server |
|--|--------------------------|---------------------|
| **数据位置** | GitHub 远端 | 本地 clone 的文件夹 |
| **检索方式** | AI 按关键词匹配 `_index.md`，定向取文档 | AI 自主全文搜索，动态决定读哪些文档 |
| **覆盖范围** | 依赖 `_index.md` 是否及时更新 | 实时扫描本地文件，不依赖索引文件 |
| **语义失配风险** | 有（关键词不匹配就找不到） | 低（全文搜索 + Obsidian 感知标签/反链） |
| **需要额外基础设施** | 无（gh api 现成可用） | 需要本地运行 MCP Server 进程 |
| **适合的客户端** | 任何能执行 gh 命令的环境 | 支持 MCP 的客户端（Cursor、Claude Desktop） |

---

**结论**：你的理解方向对，只是机制不同——不是传统 RAG，是 LLM 直接用工具调用读本地文件。你现在用 Cursor，它支持 MCP，技术上是可以接的。但如前所说，现阶段你的 gh api 索引方案已经够用，这是一个未来可选的升级方向。

Turn:1～8

模型：Claude Sonnet 4.6（Anthropic）

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 9）

是的，但是我想了解，你睡的升级，到底好在哪里，所以我需要一个方案的对比。 

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 9）

Turn9

用户意图：通过具体场景对比，理解 MCP 方案相对于现有 gh api 索引方案的实际差异。

用同一个问题走两条路，差异就清楚了。

---

## 场景对比：同一个问题，两条检索路径

**问题**：「画像输出之后用户不满意，应该怎么处理？」（答案在 `ai-thinking-framework` 的 TPM 文档里）

### 路径 A：你的现有方案（gh api 两级索引）

```
用户问题
  ↓
AI 扫描系统 prompt 里的触发规则
  → 规则写的是："涉及 AI 协作思维框架 / 解题模型 / 画像 → ai-thinking-framework"
  → "画像" 这个词在规则里有 ✅ → 触发
  ↓
gh api 读 ai-thinking-framework/_index.md
  → 找到 "target-portrait-model" 条目 ✅
  ↓
gh api 读该文档
  → AI 找到「纠偏与门控规则」→ 回答正确 ✅
```

这是**顺利路径**。但换一个问题：

**问题**：「我的模型在什么情况下会被动降级？」

```
AI 扫描触发规则
  → "降级"这个词不在任何触发关键词里
  → 没有触发任何仓库查找
  ↓
AI 用训练知识直接回答 ❌（你的 TPM 里有明确的降级规则，但 AI 看不到）
```

**问题**：「Step 0 卡住了怎么办？」

```
"Step 0" 不在触发关键词里 → 静默失败 ❌
```

---

### 路径 B：Obsidian MCP Server

```
用户问题（任何措辞）
  ↓
AI 调用 MCP 工具
  → search_notes("降级 门控")
  → 全文搜索本地 vault，命中 TPM 文档里的「降级规则」段落 ✅
  ↓
AI 调用 read_note(path) 读文档
  → 正确回答 ✅
```

不依赖关键词触发，不依赖 `_index.md` 覆盖范围，**AI 主动搜、自己判断读哪些文档**。

---

## 核心差异表

| 维度 | gh api 两级索引 | Obsidian MCP Server |
|-----|--------------|---------------------|
| **检索触发** | 关键词匹配 instruction 规则，被动触发 | AI 自主判断是否搜索，主动触发 |
| **检索覆盖** | 只能找到 `_index.md` 里有记录的文档 | 直接搜索所有文件，无需索引文件 |
| **语义失配** | 问法偏了就静默失败 | 全文搜索，措辞不影响找到率 |
| **多跳检索** | 通常一次读一篇文档 | 可以读 A 文档 → 发现引用 B → 再读 B |
| **维护成本** | 需要保持 `_index.md` 及时更新 | 无需索引文件，文件存在即可检索 |
| **延迟** | 2 次 gh api 网络调用 | 本地文件操作，近零延迟 |
| **可审计性** | 明确：能看到 AI 读了哪个 URL | 较隐式：需要看 MCP tool calls 日志 |
| **环境依赖** | 只需 gh 命令，任何环境可用 | 需要本地 vault 同步 + MCP Server 运行 |
| **数据新鲜度** | 读 GitHub 远端，永远是最新 push 的版本 | 读本地 clone，需手动/自动 git pull |

---

## 一句话概括差异

> 现有方案：**你告诉 AI 应该去哪找**（instruction 规则 + `_index.md`）。
> MCP 方案：**AI 自己决定去哪找**（全文搜索 + 工具调用）。

前者可预测、可控、易调试，但覆盖盲区由你负责填补。后者覆盖更全、维护更少，但 AI 的检索行为更难预测。

---

## 对你的体系，升级的实际收益

你的知识库已经很结构化，`_index.md` 维护得好，触发规则也清晰。**现阶段两级索引的主要风险只有一个**：新写了文档但忘了更新 `_index.md`，或者问法偏了没触发。

MCP 能消灭这个风险，代价是多一个本地服务要跑。**什么时候值得换**：当你发现「我的知识库里明明有，AI 就是找不到」的情况开始频繁出现。

Turn:1～9

模型：Claude Sonnet 4.6（Anthropic）

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 10）

好的。 请执行：dtd_normalize

