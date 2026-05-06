# Dialogue Insight Archive 模块化重构：从架构设计到完整执行

> 创建时间：2026年5月5日 08:36

<!-- DDM:TURN_SEP:v1 -->

## User

查一下 zarazhangrui/follow-builders 这个项目，我想把它的 feed 数据集成到 cognitive-trace-archive 的 index.html 里。

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 1）

通过 `gh api` 并行获取 README 和目录结构。

**follow-builders 项目概况**：追踪 25 位 AI 创作者的内容聚合工具，通过 GitHub Actions 定时爬取，生成三个 JSON 文件：

- `feed-x.json`：按人物分组的推文列表，每条含 text / timestamp / likes / retweets / replies / url
- `feed-podcasts.json`：播客列表，含 title / published / url / transcript（全文）
- `feed-blogs.json`：博客列表（当前 `blogs: []` 为空）

**当前 index.html 状态**：~2600 行单文件，包含所有 CSS + JS 内联。功能完整（卡片列表 / 文档 Viewer / annotations / 注解编辑），但长期堆叠导致代码难以维护。

<!-- DDM:TURN_SEP:v1 -->

## User

从长期迭代的角度，这个项目架构上有没有问题？有没有更好的方向？

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 2）

读取完整 index.html（~2600 行），识别出三条架构路径：

| 路径 | 描述 | 适合场景 |
|---|---|---|
| **A：继续堆叠** | 在现有文件里直接加功能 | 短期快速交付 |
| **B：ES Modules 拆分** | 不引入构建工具，用原生 module 拆文件 | 个人工具长期迭代 ✅ |
| **C：React/Vue + 构建** | 现代框架，CI/CD，TypeScript | 商业产品 |

**推荐 B**：零构建依赖（server.py 继续原样），拆分后每个 .js 文件职责清晰，测试可加可不加，迭代成本大幅降低。

<!-- DDM:TURN_SEP:v1 -->

## User

方案 B 可靠吗？重构有哪些风险？

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 3）

方案 B 可靠，但有三个隐藏耦合点需要在执行前明确：

1. **`loadAnnotationsSummary` 直接 mutate `indexData`**：这个函数在读取注解后直接修改主数据结构，迁移时必须显式化为 `mergeAnnotations()` 函数
2. **`entry._id` 反向引用**：条目通过 `entry._id` 在运行时反向引用自身，迁移时需要用 `Map<path, id>` 替代
3. **`currentEntry / currentLayer / currentAnnotation` 跨组件共享**：12 个全局变量分散各处，必须先统一收归 `state.js` 再迁移组件

风险可控，关键是要有分步执行计划。

<!-- DDM:TURN_SEP:v1 -->

## User

详细的计划可以降低风险吗？

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 4）

可以，但计划必须包含两个关键要素：**逐条功能回归验证清单** + **每阶段独立 Git commit**。

目标文件结构：
```
index.html / app.css / js/state.js / js/api.js / js/utils.js
js/components/sidebar.js, cards.js, viewer.js, comments.js, links-bar.js
js/components/modals/delete-dialog.js, commit-dialog.js, move-dialog.js
js/feed.js / js/main.js
tests/utils.test.js / tests/api.test.js
```

分阶段计划（Phase 0-5），总时长约 2 天，Phase 3 是最高风险阶段（三个耦合点必须在此阶段显式处理）。

<!-- DDM:TURN_SEP:v1 -->

## User

这个验证过程是人工验证吗？是否可以加入单元测试？加入之后可以降低验证成本吗？

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 5）

验证分两层，性质不同：

| 测试类型 | 覆盖的风险面 | 接入成本 | 适合本项目 |
|---|---|---|---|
| 纯函数单元测试（Vitest） | ~20% | 极低 | ✅ 值得 |
| API mock 测试 | ~15% | 低 | ✅ 顺手加 |
| jsdom 集成测试 | ~40% | 高 | ⚠️ 勉强 |
| Playwright E2E | ~80% | 很高 | ❌ 过度 |
| **人工验证清单** | **~95%** | 每次 15-20min | **✅ 不可替代** |

核心结论：**单元测试降低验证成本，但不能替代人工验证清单**。Vitest 把 5 秒内确认纯函数没退化，人工验证范围从 20min 压到 10-12min。

<!-- DDM:TURN_SEP:v1 -->

## User

如果这个项目是价值 200 万的商业项目，需要商业包装推广，从这个定位，重构方案应该怎么做？

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 6）

商业产品的架构要求与当前方案存在根本差距：

| 维度 | 方案 B（当前规划） | 200 万商业产品 |
|---|---|---|
| 开发协作 | 单人，ES Modules 足够 | 多人团队，需 TypeScript |
| 构建流程 | 无构建，浏览器直跑 | Vite + 打包 + CDN |
| 测试 | Vitest 纯函数 + 人工 | 单元 + 集成 + E2E + CI 门禁 |
| 后端 | server.py（同步单线程） | 生产级 API + 数据库 |

核心结论：**商业化的成本不在代码重构，在于数据模型（本地文件→数据库）、权限模型（零→多租户）、部署模型（本机→云服务）**。这三个变化会让前端代码几乎全部作废重写。

方案 B 对"个人工作台，长期迭代"是完全正确的选择。如果将来商业化，正确决策是以方案 B 作为原型参考，用现代技术栈重新立项。

<!-- DDM:TURN_SEP:v1 -->

## User

你说的对，我们重新定位到个人工作台，长期迭代。为了最小化重构风险，你觉得这个重构方案，从架构上，应该包含哪几部分？

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 7）

三部分，缺一不可：

**一、模块边界设计**：决定"哪些东西会频繁变化"。规则只有一条：业务层可以 import 稳定层，稳定层不能 import 业务层。

**二、状态管理契约**：消灭 12 个散落全局变量的隐式共享，所有模块通过 `import state from './state.js'` 读写同一个引用。

**三、验证门机制**：每个 Phase 对应一个 Git commit，commit 前必须 `npm test` 通过 + 人工走完对应功能区的验证清单。

三部分关系：
- 模块边界 → 决定"代码放在哪里"
- 状态契约 → 决定"数据怎么流动"
- 验证门 → 决定"怎么知道没出错"

<!-- DDM:TURN_SEP:v1 -->

## User

可以，基于这三部分更新一版完整计划。我希望这是一个分步骤的、每一个步骤风险可控的、可验证的计划。

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 8）

先通过 `grep_search` 确认所有全局变量的实际名称，然后更新 session plan，输出完整计划。

**状态契约**（12 个全局变量的归宿）：
```js
state = {
  index: { data, groupedByDate, titleCache, diffStatus, annotations, titleFetchCache },
  ui: { activeDate, archiveRoot },
  viewer: { entry, layer, rawText, annotation, commentEditCtx }
}
```

**分阶段执行**（Phase 0-5，每 Phase 一个 commit，验证门通过才继续）：
- Phase 0：提取 app.css（20min，零风险）
- Phase 1：utils.js + api.js + Vitest（2-3h，极低风险）
- Phase 2：state.js + main.js 骨架（1-2h，低风险）
- Phase 3：sidebar + cards 迁移（2-3h，**最高风险**，逐函数验证）
- Phase 4：viewer + dialogs（2-3h，中等风险）
- Phase 5：新增 feed.js（1-2h，零风险）

**验证清单**：25 个功能区验证点，每 Phase 逐条检查。

<!-- DDM:TURN_SEP:v1 -->

## User

我理解一下，三个架构原则，其实是我们希望达到的终态是吗？它其实是一个目标指引，引导我们的重构过程的方向，并且测量是否更加靠近目标了，是这样的吗？

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 9）

你的理解基本正确，但三个原则的性质并不相同：

- **状态契约** — 是一个**具体的终态目标**。12 个全局变量收归 `state.js` 有明确的完成时刻。
- **模块边界** — 是一条**约束规则（尺子）**，不是终态。重构完成后你仍然在用它判断新代码放哪里。
- **验证门** — 是一个**过程机制**，与终态无关。它描述"怎么确认没出错"，迭代中永远适用。

更精确的表述：
> 状态契约告诉你要去哪里，模块边界告诉你怎么走，验证门告诉你有没有走偏。

<!-- DDM:TURN_SEP:v1 -->

## User

如果需要提炼一个完整的重构框架，你觉得这三个原则够了吗？一个完整的重构框架，应该包含哪些部分？补充一点，重构框架的定位是引导、约束重构的过程，它也是从更高的抽象维度定义整个重构的过程，它有点类似系统架构方案。

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 10）

你的类比基本准确，但有一处值得精确：系统架构描述的是**目标态的结构**（空间描述），重构框架还需要约束**变换过程本身**（空间 + 路径描述）。

三个原则覆盖了目标态、决策规则和验证机制，但缺少两件事：

**缺口 1：变更边界（Scope）**：明确哪些东西在重构过程中**冻结不动**。比如 server.py API 接口、index.json 格式、HTML DOM 的 id/class 名称。没有这条，重构中会不断面临"顺手改一下"的诱惑，出问题时定位不到原因。

**缺口 2：迁移顺序规则**：框架层面需要一条"**一个模块的所有依赖迁移完成之前，该模块不能迁移**"的规则，才能在遇到任意判断题时独立作出决策。

完整的重构框架应包含四个部分：

| 部分 | 作用 |
|---|---|
| 目标态定义 | 描述终态结构（状态契约） |
| 决策规则 | 约束每步怎么做（模块边界 + 迁移顺序） |
| 变更边界 | 明确什么不能动 |
| 验证机制 | 保证每步不退步（验证门） |

<!-- DDM:TURN_SEP:v1 -->

## User

可以，我觉得可以把这个重构框架填充完整，然后记录到 refactor-1.0 目录下。

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 11）

两个空白还需要填充才算完整。

**变更边界**（5 类冻结对象）：

| 冻结对象 | 原因 |
|---|---|
| server.py 14 个 API 的 URL 和参数格式 | api.js 封装依赖这些契约 |
| index.json 数据结构（entries / common_path / layers 等） | state.js 数据模型依赖它 |
| HTML DOM 的 id / class 名称 | JS 通过 getElementById / querySelector 引用 |
| CDN 引用 marked.min.js | viewer.js 依赖 marked.parse() |
| 目录结构（raw / distilled / digest / trace） | api.js 路径拼接依赖它 |

**迁移顺序规则**（正式表述）：
> 一个模块的所有依赖迁移完成之前，该模块不能迁移。迁移方向：从叶子节点（无依赖的纯函数）向根节点（main.js 入口）推进。

然后询问 refactor-1.0 目录位置。

<!-- DDM:TURN_SEP:v1 -->

## User

根目录下。

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 12）

创建三个文件：

- **`refactor-1.0/framework.md`**：完整重构框架（4 部分：目标态定义 / 决策规则 / 变更边界 / 验证机制），含 state.js 状态契约、模块依赖层次图、冻结对象清单
- **`refactor-1.0/plan.md`**：分阶段执行计划（Phase 0-5），每 Phase 含"做什么 / 不做什么 / 验证门 / 回滚方式 / Commit 信息"
- **`refactor-1.0/checklist.md`**：功能回归验证清单，49 个验证点（比原讨论的 25 条细化，加入了更精确的步骤描述）+ 每 Phase 记录模板

<!-- DDM:TURN_SEP:v1 -->

## User

在开始之前，请审查一下 plan 和当前代码状态的匹配度。是否存在矛盾和冲突的地方，可以列出来。

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 13）

通过多次 `grep_search` 和 `read_file` 对 index.html 逐项验证。

发现 **6 处矛盾**：

🔴 **严重**（执行时会出错）：
1. **Phase 2/3 的 module 切换时机错误**：当前 index.html 的 `<script>` 不是 module，无法使用 `import`。plan.md 的 Step 3.1 写"在 `<script type="module">` 区域 import utils.js"，但原 script 标签就是普通 `<script>`，`import` 语句无法在此执行。
2. **`buildGroups` 有副作用被错误分类**：plan.md 说它"只操作数据，无 DOM 依赖"，但实际代码直接读写全局 `indexData` 和 `groupedByDate`，迁移时若不同步修改为纯函数会出现状态不同步。

🟡 **中等**（执行时需要额外决策）：
3. **`LAYERS` 和 `REPO` 常量归宿缺失**：这两个常量被多个组件使用，plan 的目标结构没有提到放哪里。
4. **`/api/commit` 被调用两次但语义不同**（单文件 L2154 vs 全量 L2300），api.js 的 `commitFiles` 封装需要明确区分。
5. **`loadTitles` 内有独立的 `./raw/` fetch**，未关联到 api.js 的 `fetchFileContent`。

🟢 **轻微**：
6. **行号标注偏差**（escHtml ~L1306 实际 L1304，importanceBadgeHtml ~L2104 实际 L2054 等）。

<!-- DDM:TURN_SEP:v1 -->

## User

可以，修正这些问题吧。

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 14）

使用 `multi_replace_string_in_file` 同时修正 framework.md 和 plan.md 的 6 处问题：

| # | 文件 | 修改内容 |
|---|---|---|
| 1 | framework.md | 文件结构中增加 `constants.js`（REPO / LAYERS / IMPORTANCE_CYCLE） |
| 2 | plan.md | Step 1.2 行号纠正（escHtml→L1304, importanceBadgeHtml→L2054, nowTs→L1643 等） |
| 3 | plan.md | `commitFiles` 签名改为 `files?`，说明单文件 L2154 vs 全量 L2300 的语义差异 |
| 4 | plan.md | 新增 **Step 1.6**：提取 `js/constants.js` |
| 5 | plan.md | **Step 3.1** 彻底重写：核心改为"将 `<script>` 升级为 `type="module"`"，注明这是关键切换点 |
| 6 | plan.md | **Step 3.3** 新增警告：buildGroups 必须改为纯函数（接收参数、返回结果），由调用方写 state |
| 7 | plan.md | **Step 3.8** 补充：loadTitles 内的 `./raw/` 硬编码 fetch 必须替换为 `api.fetchFileContent` |

<!-- DDM:TURN_SEP:v1 -->

## User

另外，我考虑 plan 按照 Phase 拆分到不同的文件中。这样避免执行的时候不同的 Phase 互相干扰。你觉得呢？

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 15）

这个想法合理，但需要权衡：

**支持**：执行 Phase 3 时只开 phase-3.md，不误改其他 Phase；每个 Phase 文件可加 `- [x]` 标记完成进度；git diff 更干净。

**反对**：跨 Phase 引用（如 Step 3.1 说"移除 Phase 2 添加的 `<script type="module">`"）变隐式，容易漏看。

**建议**：拆分，但保留轻量 `plan.md` 作为入口（只保留总览表 + 执行原则），每个 phase 文件开头加前置依赖声明：

```
refactor-1.0/
  plan.md        ← 总览 + 通用执行协议
  phase-0.md
  phase-1.md
  phase-2.md
  phase-3.md
  phase-4.md
  phase-5.md
```

<!-- DDM:TURN_SEP:v1 -->

## User

同时，需要考虑 AI 会不会跳过某些步骤执行？如何避免呢？

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 16）

两个问题是同一个根源：**执行文档本身没有约束 AI 的执行节奏**。

AI 的默认行为是"理解意图 → 找最短路径 → 一次性执行"，会把 Step 3.1 到 3.9 视为一个整体批量完成。

**解决方案：在文档里硬编码执行守卫（Execution Guard）**：

```markdown
## ⚙️ 执行协议（AI 必须遵守）

- 每次只执行**一个 Step**，执行完后立即停止
- 停下来，告诉用户"Step X.Y 完成，请验证：[具体验证项]"
- 等用户明确说"通过"或"继续"后，才执行下一个 Step
- 遇到验证失败，立即停止并报告，不自行修复后跳过
```

每个 phase 文件的每个 Step 结尾加"**停止信号**"模板，这是 AI 发言的约束模板，利用 AI 对明确指令的服从性强制分步执行。

<!-- DDM:TURN_SEP:v1 -->

## User

是的，可以的。

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 17）

读取 plan.md 末尾内容，然后执行重构：
- 将 `plan.md` 简化为总览 + 通用执行协议
- 创建 `phase-0.md` 到 `phase-5.md`，每个文件含前置确认 + Step 列表 + 每 Step 结尾的停止信号

<!-- DDM:TURN_SEP:v1 -->

## User

（Turn 18-25 内容为 plan 拆分执行确认、Phase 0-2 开始实施讨论。）

*Turn 18-25 相关内容：Phase 0（提取 app.css）、Phase 1（utils.js + api.js + Vitest + 39 个测试用例）、Phase 2（state.js + main.js 骨架）陆续完成。每阶段均通过 npm test + 人工验证清单后提交 Git commit。Period 设计讨论包含：Vitest 测试覆盖策略、constants.js 的 import 时机、ES Module 切换对 DOMContentLoaded 时序的影响。*

<!-- DDM:TURN_SEP:v1 -->

## （Turn 26–54 省略）

*省略内容说明：Turn 26–54 为 Phase 3（sidebar + cards 迁移）、Phase 4（viewer + dialogs 迁移）的完整执行过程，包含约 19 个子步骤（Step 3.1–3.9 + Step 4.0–4.9）的逐步迁移、验证、修复和提交。每步均遵循执行协议：一步一停、人工确认通过后继续。Phase 4 Steps 4.0–4.5 在上一对话窗口完成，Steps 4.6–4.10 在本窗口执行。*

<!-- DDM:TURN_SEP:v1 -->

## User

请检查任务是否全部执行完，检查遗漏项目。

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 55）

全部任务已完成，无遗漏。

**状态总览：**

| Phase | Steps | 状态 |
|-------|-------|------|
| Phase 4 | 4.0–4.10 | ✅ 全部完成 |
| Phase 5 | 5.1–5.3 | ✅ 全部完成 |

**当前文件结构（终态）：**
- `index.html` L129：`<script type="module" src="js/main.js"></script>`（单行，script 内容完全清空）
- `js/main.js`：接管所有初始化 + 事件绑定 + Tab 切换（archive ↔ feed）
- `js/feed.js`：Feed 渲染模块（X / Podcasts / Blogs 三区块，懒加载，只加载一次）
- `js/components/modals/`：`commit-dialog.js` / `delete-dialog.js` / `move-dialog.js`
- `js/components/comments.js`、`links-bar.js`：注解和链接栏模块

**39/39 tests 通过**（Vitest 覆盖 utils.js 纯函数 + api.js fetch 封装）。

**Phase 5 Feed 行为**：
- 点击「📡 Builders」切到 feed 视图（懒加载，只加载一次）；再次点击切回 archive
- X 按人分组展示推文，含 likes / retweets / replies 统计和原文链接
- Podcasts 前 300 字预览，点击「展开全文」显示完整 transcript
- Blogs 空状态友好提示（当前 blogs: [] 为空）

<!-- DDM:TURN_SEP:v1 -->
