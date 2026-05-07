# LuLu-Workbench 支持搜索 - Meilisearch 接入

> 创建时间：2026年5月7日 18:20

> 导航：[distilled](../../../distilled/ai-software-dev/workbench-search-meilisearch-integration/202605071820-lulu-workbench-meilisearch-search.md) · [digest](../../../digest/ai-software-dev/workbench-search-meilisearch-integration/202605071820-lulu-workbench-meilisearch-search.md) · [trace](../../../trace/ai-software-dev/workbench-search-meilisearch-integration/202605071820-lulu-workbench-meilisearch-search.md)

<!-- DDM:TURN_SEP:v1 -->

## User 

请输入对话内容： https://github.com/lulufoo/lulu-workbench/blob/main/raw/product-analysis/obsidian/202605071216-obsidian-lccm-lulu-session.md

这个对话讨论到一个点，Workbench作为第二层，需要做知识库的搜索链接到第三层的知识库上。下面是讨论中涉及的一个方案：把搜索建在层3内容本身上，不依赖 Obsidian 是否运行。

层3知识仓库（GitHub repos）
    ↓ 定时 clone / pull 到本地/服务端
本地 markdown 文件目录
    ↓ 建索引
轻量全文搜索（如 whoosh / meilisearch）
    ↓
Python 后端 /api/search-knowledge
    ↓
工作台前端「相关知识」面板

请你客观评估一下本方案的合理性？ 

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 1）

现在我有足够的上下文了。对这个方案做客观评估：

---

## 方案评估：独立 Markdown 全文搜索

**方案核心**：克隆层3知识仓库 → 本地建索引 → Python 后端提供搜索 API → 前端展示「相关知识」面板

---

### 合理性

**✅ 架构决策是对的，理由充分：**

1. **解耦 Obsidian 进程依赖**——对话 Turn27 已识别出 Local REST API 的硬约束：Obsidian 必须在本机运行，服务端部署后断路。方案绕开了这个限制。

2. **与现有栈一致**——`server.py` 已经是 Python HTTP server，加一个 `do_GET` 分支处理 `/api/search-knowledge` 是自然扩展，无技术债。

3. **数据主权清晰**——层3是 Markdown 纯文本，不依赖任何外部服务，和工作台的 Local-first 原则对齐。

---

### 待明确的决策点

**⚠️ 搜索引擎选择**：whoosh vs meilisearch 有本质差异

| | whoosh | meilisearch |
|--|--------|-------------|
| 依赖 | 纯 Python，零外部进程 | 独立二进制进程，需常驻 |
| 搜索质量 | 关键词精确匹配 | 拼写容错 + 语义排序更好 |
| 适合场景 | 本机原型，快速跑通 | 服务端部署，长期稳定 |
| 维护状态 | 已停止主动维护（2021） | 活跃维护 |

对于阶段性验证，whoosh 够用且零摩擦；服务端场景下 meilisearch 更稳，但引入了进程管理复杂度。

**⚠️ 索引更新策略**：「定时 clone/pull」的频率和触发时机需要设计。层3仓库更新频率不高，按需触发（手动 pull 或 git hook）比定时轮询更轻量。

**⚠️ 搜索范围**：索引全部层3仓库还是按 topic 过滤？Entry 处理时通常有明确 topic，按 topic 缩小搜索范围可以提升结果相关性。

---

### 整体判断

**方案本身合理**，是对话中两阶段建议（先 Local REST API 原型验证 → 再独立索引）的第二阶段。跳过原型阶段直接做这个也成立，前提是确认「相关知识面板」的需求价值足够确定，不需要再验证。

如果希望做计划，可以直接进入方案设计。

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 2）

可以进入方案设计，需要先聊清楚方向： whoosh vs meilisearch ，除了这2个，还有其它的方案吗？ 请发散调研一下。

<!-- DDM:TURN_SEP:v1 -->

## AI

## 可选方案全景（2026年现状）

| 方案 | 类型 | 依赖 | 是否嵌入 | 容错搜索 | 维护状态 |
|---|---|---|---|---|---|
| **SQLite FTS5** | 标准库 | 零 | ✅ | ❌ | ✅ 活跃 |
| **tantivy-py** | Rust绑定 | Rust wheels | ✅ | ✅ | ✅ 活跃 |
| **meilisearch** | 独立进程 | 外部二进制 | ❌ | ✅ | ✅ 活跃 |
| **typesense** | 独立进程 | 外部二进制 | ❌ | ✅ | ✅ 活跃 |
| **Xapian** | C++绑定 | 系统库 | ✅ | ✅ | ✅ 活跃 |
| **lunr.py** | 纯Python | 零 | ✅ | ❌ | ⚠️ 几乎停更 |
| **whoosh** | 纯Python | 零 | ✅ | ❌ | ❌ 已停更 |

---

### 各方案关键差异

**SQLite FTS5**
Python stdlib 自带，`sqlite3` 模块直接调用，零安装。支持 BM25 排名、NEAR 短语查询、前缀匹配。唯一短板：不支持拼写容错（搜「kownledge」找不到「knowledge」）。对你的场景（中文标题 + 英文 slug 的 Markdown）够用。

**tantivy-py**
Rust 写的搜索引擎，Python 绑定。嵌入式（不需要外部进程），原生支持拼写容错。性能远超纯Python方案。要求 Python 3.10+，需要安装 wheels（约 20MB）。⚠️ 验证一下你的服务端Python版本。

**typesense**
功能上类似 meilisearch，补充点：更稳定的向量搜索支持、内置 geo-search，更偏企业向。如果未来需要语义搜索（embedding），typesense 是更完整的选择。

**Xapian**
老牌搜索库（20年），评分模型更丰富。但需要编译 C++ 或安装系统包，设置成本高，对这个规模的场景得不偿失。不推荐。

---

### 对你的场景的映射

你的约束：
- 现在本地运行（`server.py` 纯标准库）
- 未来部署到服务端
- 搜索目标：层3 Markdown 文档（中英混合，数百到数千篇）

形成三条路径：

**路径A：零依赖**
`SQLite FTS5` 全程——本地和服务端完全一致，无外部进程，索引存一个 `.db` 文件。中文需要自行分词（`jieba`），或按标题+路径搜索绕开正文分词问题。

**路径B：嵌入式 + 容错**
`tantivy-py`——保留嵌入式优势，获得拼写容错，性能更好。引入一个二进制依赖，需要确认服务端 Python ≥ 3.10。

**路径C：两阶段渐进**
本地原型用 `SQLite FTS5`（零摩擦验证需求），服务端上线时换 `meilisearch` 或 `typesense`（同一套API，换URL即可）。

---

**在进入方案设计之前，有两个问题需要确认：**
1. 主要内容语言是中文吗？（影响分词策略）
2. 是否需要拼写容错？（如搜索 「kownledge」 能找到 「knowledge」）

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 3）

（上下文确认：内容主要是中文，有英文 slug；需要容错搜索；Python 3.9）

<!-- DDM:TURN_SEP:v1 -->

## AI

**主要中文 + 需要容错**——这把 SQLite FTS5 从首选降级了。原因：FTS5 对中文缺乏原生分词（中文无空格，按字符切割会漏掉词组），容错也没有。

---

## 更新后的推荐方向

**SQLite FTS5 出局**（不处理中文分词 + 无容错），从候选中移除。

剩下两条可行路径：

### 路径B：tantivy-py（嵌入式）
- 中文：需要配 `jieba` 做分词预处理，把 Markdown 内容先分词再写入索引
- 容错：原生支持
- 约束：Python 3.10+，需确认服务端版本

### 路径C：meilisearch 或 typesense（独立进程）
- 中文：两者都内置中文分词，开箱可用，不需要手动接 jieba
- 容错：原生支持
- 代价：多一个常驻进程，部署时需要管理

---

**两者的本质分界点只有一个**：是否接受「多一个外部进程」。

- 接受 → meilisearch（更简单，中文更省心）
- 不接受 → tantivy-py + jieba（嵌入，但有预处理成本）

你倾向哪条？确认后进入具体方案设计。

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 4）

可以 meilisearch 接入，但是有一个点需要调查清楚，仓库经常更新的，索引需要手动维护更新吗？

<!-- DDM:TURN_SEP:v1 -->

## AI

## Meilisearch 索引更新——不需要手动维护

### Meilisearch 的更新机制

Meilisearch 支持**文档级增量更新**，核心 API 是 `add_or_update_documents`：

- 每个文档有一个主键（可以是文件路径）
- 文件更新时只推送那一个文档，不需要全量重建索引
- 删除文档用 `delete_document(id)` 精确删除
- 全量重建仅在初次建库时需要一次

### 与你现有流程的天然接入点

`server.py` 里已经有两个精确的触发时机，**不需要新增调度机制**：

**接入点1：`_handle_settle`（最重要）**
Settle 完成时，`server.py` 已经知道：文件路径、内容、目标仓库。在 Step1 成功返回后，直接调 meilisearch upsert 推送这一条文档即可。✅ 这是最精准的时机——每次沉淀自动同步。

**接入点2：初始全量建库**
第一次部署时，遍历层3本地克隆目录，批量推送所有 Markdown 文档。一次性操作。

**接入点3：手动触发（可选兜底）**
加一个 `/api/reindex-knowledge` 端点，允许手动触发全量重建，处理极端情况（如仓库外直接编辑文件）。

### 层3仓库更新路径梳理

```
workbench settle → _handle_settle → gh api 推送文件
                                  → 同时 upsert meilisearch（新增）

外部直接 git push → 本地 clone 需要 pull
                 → /api/reindex-knowledge 手动补跑（低频兜底）
```

层3的更新**几乎都来自 workbench 的 settle 操作**，所以接入点1覆盖了主路径。「外部直接推送」是低频路径，有手动触发端点兜底即可。

---

**结论**：正常使用下索引完全自动维护，`_handle_settle` 是唯一需要改动的位置，加约 10 行代码。可以进入方案设计了。

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 5）

这里有一个澄清： 我说的知识库搜索，不是workbench ，而是下游的沉淀知识库。 workbench 本身的检索，是后续考虑的，暂时不做。

<!-- DDM:TURN_SEP:v1 -->

## AI

理解了，收回接入点1的描述——settle 是写入路径，不是搜索范围。

**重新对齐搜索边界：**

```
搜索范围：层3知识仓库（ai-assisted-domain-learning、ai-thinking-framework 等 GitHub repos）
搜索入口：workbench 前端的「相关知识」面板
workbench 自身内容（raw/distilled/digest）：不在本次范围内
```

**索引更新机制重新分析：**

层3仓库通过两条路径更新：
1. **workbench settle** → `gh api` 直接推送到 GitHub → 本地克隆不自动同步
2. **其他方式直接 push**（如在知识仓库里直接写作）

两条路径都不会自动更新本地克隆，因此接入点变成：

| 时机 | 机制 | 覆盖路径 |
|---|---|---|
| settle 完成后 | server 已有文件内容 → 直接 upsert meilisearch，绕过本地克隆 | 覆盖路径1 |
| 手动触发 `/api/reindex-knowledge` | `git pull` 所有层3克隆 → 差量重建索引 | 覆盖路径1+2的补全 |
| 首次部署 | `git clone` 所有层3仓库 → 全量建库 | 初始化 |

核心结论不变：**正常使用（通过workbench settle）索引自动更新**；直接在知识仓库里编辑的内容，手动触发一次补全。

---

这个边界清楚了，可以进入方案设计了吗？

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 6）

你给我一个完整、详细的方案

<!-- DDM:TURN_SEP:v1 -->

## AI

## Plan: 层3知识库全文搜索接入

**TL;DR**: 部署 Meilisearch 作为搜索引擎（原生支持中文分词，无需 jieba 预处理）。将 `topics.json` 中 15 个层3 GitHub 仓库克隆到本地 `.cache/knowledge-repos/`，建索引；`server.py` 扩展两个新端点；前端 viewer 增加「相关知识」面板。

---

### Phase 0 · 环境配置

**步骤 1** — 安装 Meilisearch
- 本地开发：`brew install meilisearch`（macOS）
- 服务端部署：`docker run -p 7700:7700 -e MEILI_MASTER_KEY=xxx meilisearch/meilisearch`

**步骤 2** — 配置文件
- 生成 master key，写入 `.cache/meili.env`
- `.gitignore` 追加 `.cache/knowledge-repos/` 和 `.cache/meili.env`
- `server.py` 封装 `_meili_request(method, path, body)` 工具函数，使用 `urllib.request`（stdlib，无外部依赖，与现有 server.py 风格一致）

---

### Phase 1 · 仓库同步脚本

**步骤 3** — 新建 `scripts/sync_knowledge_repos.py`

逻辑：
- 读 `topics.json`，筛选有 `repo` 字段的条目（共 15 个）
- 每个 repo：目录不存在 → `git clone git@github.com:{repo}.git`；已存在 → `git pull --rebase`
- 克隆目录：`.cache/knowledge-repos/{repo-name}/`
- 命令行参数：`--all`（全部）/ `--repo=lulufoo/xxx`（单个 repo）

---

### Phase 2 · 索引构建脚本

**步骤 4** — 新建 `scripts/build_knowledge_index.py`

Meilisearch 文档 schema：
```
id           — "{owner}__{repo}__{urlsafe_path}"（唯一键）
title        — 文件第一个 H1 标题，没有则用文件名
body         — Markdown 原文
repo         — "lulufoo/ai-thinking-framework"
path         — "topic/filename.md"
url          — GitHub blob 链接
topic_desc   — 来自 topics.json 的 description 字段
```

跳过的文件（黑名单）：`_index.md`、`README.md`、frontmatter-only 的空文件

批量上传：每批 100 文档，调 Meilisearch `PUT /indexes/knowledge/documents`

参数：`--wipe`（先删再建，全量重建）/ 无参数（增量 upsert）

---

### Phase 3 · server.py 扩展

**步骤 5** — `do_GET` 新增路由 `/api/search-knowledge` → `_handle_search_knowledge()`

**步骤 6** — `do_POST` 新增路由 `/api/reindex-knowledge` → `_handle_reindex_knowledge()`

**步骤 7** — `_handle_settle()` 追加 upsert

---

### Phase 4 · 前端

**步骤 8** — `js/api.js` 加 `searchKnowledge(q)` 函数
**步骤 9** — 新建 `js/components/knowledge-search.js`
**步骤 10** — `index.html` 加挂载点
**步骤 11** — `viewer.js` 引入组件，entry 打开时以 title/slug 自动触发搜索

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 7）

搜索范围：15 个层3 GitHub repos 的 Markdown 文件。这个是需要本地构建这个.cache 吗？ 还是说配置映射规则，映射到本地的路径就可以了？

<!-- DDM:TURN_SEP:v1 -->

## AI

层3仓库**全部已经克隆在本地** `/Users/lulu/Code/` 下，和仓库名一一对应：

```
/Users/lulu/Code/ai-assisted-domain-learning/
/Users/lulu/Code/ai-thinking-framework/
/Users/lulu/Code/learning-with-ai/
... （共 13 个可见，命名完全对应 topics.json 的 repo 末段）
```

**结论：不需要 `.cache`，也不需要 clone 脚本。**

方案可以大幅简化：

```
topics.json repo 字段
    ↓ 取 repo 末段（如 ai-thinking-framework）
/Users/lulu/Code/{repo-name}/   ← 直接读现有克隆
    ↓ 遍历 .md 文件
meilisearch 建索引
```

改动点：

1. **删掉** `sync_knowledge_repos.py`（Phase 1 整体去掉）
2. **`build_knowledge_index.py`** 改为读一个**路径映射配置**：

   | 方式 | 优点 | 缺点 |
   |---|---|---|
   | **约定优先**：`{base_dir}/{repo-name}/`，`base_dir` 写进配置一行 | 零维护，新 repo clone 后自动识别 | 要求所有 repo 在同一个父目录 |
   | **显式映射**：`topics.json` 每个条目加 `local_path` 字段 | 灵活，每个 repo 路径独立 | 需要手动维护映射 |

   你的情况全部在 `/Users/lulu/Code/` 下，**约定优先**更合适——只需在配置里配置一个 `KNOWLEDGE_BASE_DIR = /Users/lulu/Code`，自动推导，零维护。

3. **`/api/reindex-knowledge`** 也简化：直接运行 `build_knowledge_index.py`，不再需要先 pull。

---

**需要你确认一个事情**：`learning-ai-lmm`（本地目录）和 `learning-ai-llm`（topics.json）是同一个仓库还是两个不同的？确认后 plan 更新路径映射逻辑。

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 8）

是同一个，我等下直接调用同步一致，你可以忽略这个问题

<!-- DDM:TURN_SEP:v1 -->

## AI

Plan 已更新。简化后的完整方案：

---

## Plan 更新：直接读本地克隆，去掉 sync 脚本

**路径约定**：`KNOWLEDGE_BASE_DIR=/Users/lulu/Code`，`build_knowledge_index.py` 通过 `{base_dir}/{repo-name}/` 直接定位各仓库。服务端部署时用环境变量覆盖，脚本逻辑不变。

---

### 相关文件变动（更新后）

- `scripts/build_knowledge_index.py` — 新建
- ~~`scripts/sync_knowledge_repos.py`~~ — **去掉**
- `server.py` — 3 处改动
- `js/api.js` / `js/components/knowledge-search.js` / `index.html` — 前端 3 处
- `.gitignore` — 追加 `.cache/meili.env`

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 9）

需要注意，本地的缓存可能是过期的。搜索之前要做一次过期判断，过期要手动更新。

<!-- DDM:TURN_SEP:v1 -->

## AI

## 过期检测方案

### 两层过期的来源

```
远端 GitHub（最新）
    ↓ git pull（可能滞后）
本地克隆 /Users/lulu/Code/{repo}/
    ↓ build_knowledge_index（可能滞后）
Meilisearch 索引（当前搜索的数据源）
```

只有一种触发方式能让两层都新鲜：**`git pull` + `reindex` 串联执行**。
所以 `/api/reindex-knowledge` 应该是：`git pull 所有 repos` → `重建索引`，而不只是重建索引。

### 过期判断机制

**存储位置**：`.cache/meili-meta.json`

```json
{
  "last_indexed_at": "2026-05-07T12:00:00+08:00",
  "stale_threshold_hours": 24
}
```

**推荐：时间阈值**。搜索前读 `meili-meta.json`，判断 `now - last_indexed_at > stale_threshold_hours`，不发网络请求。阈值可配（默认 24h），写在 `.cache/meili.env` 里。

### 用户交互流程

```
用户打开 entry → 触发知识搜索
    ↓
/api/search-knowledge 响应额外携带：
  { hits: [...], stale: true, last_indexed_at: "2026-05-06T10:00+08:00" }
    ↓ stale=true
前端「相关知识」面板顶部显示：
  ⚠️ 索引已 26 小时未更新   [立即同步]
    ↓ 点击「立即同步」
/api/reindex-knowledge（git pull + 重建）
    ↓ 完成
更新 meili-meta.json → stale=false → 面板刷新结果
```

**关键设计原则**：
- 过期不阻断搜索——旧索引依然返回结果，只是加提示
- 更新是手动触发，不自动执行
- `_handle_settle()` 追加的单条 upsert **不更新** `last_indexed_at`（因为它只同步了一条，其他 repo 仍可能过期）

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 10）

如下这是什么阶段的事情？ 是初次建立索引吗？ 还是有更新的时候？

/api/reindex-knowledge 耗时估计：15 个 repo git pull（网络，快则几秒慢则数十秒）+ 遍历所有 .md 建索引。

<!-- DDM:TURN_SEP:v1 -->

## AI

两个场景，触发路径不同：

---

**场景 A：首次建立索引（一次性，命令行执行）**

```bash
# 手动在终端运行，不走 API
python3 scripts/build_knowledge_index.py --wipe
```

本地克隆已存在（`/Users/lulu/Code/` 下全部就位），不需要 `git pull`，直接全量扫描建库。**只运行一次**。

---

**场景 B：索引过期更新（日常，前端触发）**

```
面板显示 ⚠️ 索引已 26 小时未更新 → 用户点「立即同步」
→ /api/reindex-knowledge
→ 15 个 repo git pull + 增量 reindex
```

这就是你说的那个耗时操作。它的定位是：**过期后的手动刷新**，不是常规搜索路径，用户主动触发，等待是预期内的。

---

**关系梳理**：

| 操作 | 触发方式 | git pull | 索引模式 | 频率 |
|---|---|---|---|---|
| 首次建库 | 命令行 `--wipe` | ❌ 不需要 | 全量 | 仅一次 |
| 过期更新 | 前端「立即同步」→ API | ✅ 15 个 repo | 增量 upsert | 按需，每天级别 |
| settle 同步 | settle 完成后自动 | ❌ 不需要 | 单条 upsert | 每次 settle |

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 11）

你做成点击之后异步处理，不要阻塞前台的其它任务。 看看还有别的问题吗？

<!-- DDM:TURN_SEP:v1 -->

## AI

**server.py 用的是 `http.server.HTTPServer`（单线程）**，不是 `ThreadingHTTPServer`。在 handler 里阻塞会卡死整个服务器。异步 reindex 的正确实现方式：

- 模块级变量 `_reindex_job = {status, started_at, log}` 存状态
- `POST /api/reindex-knowledge` → 检查是否已在运行（防重复） → 起 `threading.Thread` 跑后台任务 → 立即返回 `{status: "running"}`
- `GET /api/reindex-status` → 返回当前 job 状态
- 前端点「立即同步」后每 2 秒轮询一次，完成时刷新搜索结果

---

还有 2 个 UX 问题没确认：

1. **面板位置**：viewer 右侧新增一栏，还是文档内容下方？
2. **初始搜索触发词**：用 entry 的 `title` 字段还是 `slug`（去掉时间戳的文件名部分）？

（用户确认：面板在右侧，初始搜索用 title + slug 组合）

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 12）

1 你先提炼一个需求出来，写入到 reactor-3.0。
2 然后，这里一个架构方案给我，也写入到目录。
3 Plan等一下再说。

<!-- DDM:TURN_SEP:v1 -->

## AI

两个文档已写入 `refactor-3.0/`：

- [refactor-3.0/requirements.md](refactor-3.0/requirements.md) — 问题定义、功能需求（F1-F7）、非功能需求、搜索范围、关键约束
- [refactor-3.0/architecture.md](refactor-3.0/architecture.md) — 整体架构图、选型理由、Schema、新增/改动文件清单、各模块详设（server.py 6 处、build 脚本、前端 UI + 同步流程）、Verification

有一个地方在 architecture 里已标注需要特别注意：`#md-panel` 改为横向 flex 后，`#md-header` / `#md-commit-bar` / `#md-links-bar` 需要和 `#md-body` 一起包进 `<div id="md-content-col">`，这是 `index.html` 改动量最集中的地方。Plan 阶段需要单独作为一个 WO 处理。

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 13）

我需要一个前端的设计输入，作为前端技术方案的依据。

<!-- DDM:TURN_SEP:v1 -->

## AI

（首次尝试输出了技术文档，用户说不是这个）

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 14）

这个不是前端的design，我要的是一个设计师版本的交互设计稿。

<!-- DDM:TURN_SEP:v1 -->

## AI

[refactor-3.0/interaction-design.md](refactor-3.0/interaction-design.md) 已写入。

内容覆盖：

1. **用户场景** — 一句话说清楚用户在哪、遇到什么、想要什么
2. **Viewer 布局变化** — 现状 vs 新布局的 ASCII 线框图，header 全宽不动，body 区横向分栏
3. **面板结构线框图** — 标题栏 / 搜索框 / 过期 banner / 结果卡片列表
4. **默认搜索词规则** — 构成方式 + 具体示例
5. **6 个交互状态** — 加载中 / 有结果+过期 / 有结果+新鲜 / 空结果 / 同步中 / 不可用，每个状态配线框图
6. **结果卡片设计** — 字段布局 + hover 交互
7. **「立即同步」交互细节** — 4 个阶段的按钮状态和 banner 变化
8. **视觉规格** — 完整 Token 表，全部引用现有设计系统颜色
9. **不处理场景** — 明确排除边界

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 15）

现在生成技术方案Plan。

<!-- DDM:TURN_SEP:v1 -->

## AI

[refactor-3.0/plan.md](refactor-3.0/plan.md) 已写入。

结构：
- **TL;DR** — 四模块一句话概括
- **Work Order 表** — 9 个 WO + 依赖关系
- **并行执行分组** — 4 个批次，最短交付路径
- **各 WO 详细说明** — 每个 WO 包含：改动文件、逻辑伪代码/改动位置、关键注意点
- **Verification 检查单** — 8 个可执行验证步骤（V1~V8）

`refactor-3.0/` 目录现在包含 4 个文档：

| 文件 | 内容 |
|---|---|
| requirements.md | 问题定义 + 功能/非功能需求 + 搜索范围 |
| architecture.md | 技术选型 + 模块详设 + 改动清单 |
| interaction-design.md | 交互设计稿（线框图 + 状态 + 视觉规格）|
| plan.md | 9 个 WO + 并行分组 + Verification |

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 16）

审查一下，4个文档是否有冲突和不一致，列出来

<!-- DDM:TURN_SEP:v1 -->

## AI

4 个文档读完。发现以下冲突和不一致：

---

## 冲突清单

### C1 【严重】layout 改法互相矛盾

**architecture.md § 8.3**：把 `#md-panel` 的 `flex-direction` 直接改为 `row`，然后用 `#md-content-col` 把 header/commit-bar/links-bar/md-body 整体包起来。

**plan.md WO-P3A**：`#md-panel` 保持 `column`，只在 body 区域外包一层 `#md-content-row`（flex-row），把 `#md-body` + `#md-edit-area` + `#knowledge-panel` 包进去。

两种完全不同的 DOM 结构方案，会产生不同的 HTML 改动范围。interaction-design.md 的设计意图（header/links-bar 全宽不变）与 **plan.md 的方案一致**，architecture.md 方案是旧草稿残留。

### C2 【严重】api.js 新增函数数量不一致

**architecture.md § 5.2**：`js/api.js` 一行写的是「加 `searchKnowledge(q)`、`getReindexStatus()` **两个函数**」。

**plan.md WO-P3B**：「末尾追加 **3 个函数**：`searchKnowledge` / `reindexKnowledge()` / `getReindexStatus()`」

architecture.md 漏写了 `reindexKnowledge()`，与 `/api/reindex-knowledge` 路由的调用方形成断档。

### C3 【中】plan.md 内部存在死引用

**plan.md WO-P3B**：「详见 `interaction-design.md` §五·js/api.js 新增函数」

但 interaction-design.md §五 的标题是「**交互状态全览**」（状态 A~F 线框图），不包含任何 api.js 函数说明。该引用指向了不存在的内容。

### C4 【中】`_meili_upsert_doc` 无归属 WO

**plan.md WO-P2C** 调用 `_meili_upsert_doc(repo, dst_path, full_content, topic_desc)`，但没有任何 WO 负责**定义**这个函数。WO-P0 只定义了 `_meili_request`。实现时会产生"谁来写这个函数"的歧义。

### C5 【轻】interaction-design.md 内部：卡片点击区域前后矛盾

**§三 结构线框**：标注 `.ks-hit-title（点击跳 GitHub）`，隐含只有标题可点。

**§六 卡片设计**：「整张卡片可点击，点击在新 Tab 打开 GitHub 文档」，整卡可点。

---

## 优先级

| # | 严重程度 | 需修改的文档 |
|---|---|---|
| C1 | 🔴 严重，会导致实现方向错误 | architecture.md § 8.3 |
| C2 | 🔴 严重，前端无法调用 reindex | architecture.md § 5.2 |
| C3 | 🟡 中，引用失效影响执行参考 | plan.md WO-P3B |
| C4 | 🟡 中，实现时会有歧义 | plan.md WO-P2C 或 WO-P0 |
| C5 | 🟢 轻，二选一即可 | interaction-design.md §三 |

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 17）

1 你先把需求整理清楚，需求整理完了，其它文档参考需求文档。
2 架构文档 / 交互设计 也是以需求输入作为依据的。
3 技术方案以 1 和 2的文档作为依据。

结合上面的问题，并依据以上的依赖链，优化冲突

<!-- DDM:TURN_SEP:v1 -->

## AI

全部 5 处修复完成。

| # | 文档 | 位置 | 改动 |
|---|---|---|---|
| C2 | architecture.md | § 5.2 改动清单 | `js/api.js` 由 2 个函数补全为 3 个（加 `reindexKnowledge()`）|
| C5 | architecture.md | § 8.1 面板结构 | 点击标注从 `.ks-hit-title` 移到 `.ks-hit` 层级，整卡可点击 |
| C1 | architecture.md | § 8.3 layout | 推翻旧 `#md-panel flex-direction:row` 方案；改为 `#md-content-row` 包裹 body 区域，`#md-panel` 保持 column 不变，header/links-bar 全宽不受影响，与 interaction-design.md 一致 |
| C4 | plan.md | WO-P0 | 补 `_meili_upsert_doc()` 函数定义（含 doc dict 构建逻辑），明确供 WO-P2C 调用 |
| C3 | plan.md | WO-P3B | 删除指向不存在章节的死引用，改为内联说明 3 个函数的路由和返回值 |

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 18）

1 再审查一下 plan，是否与其他文档和仓库结构，是否不一致？
2 一个额外的问题：附件的2个文件，是否重复的，如果是的，请删除dialogue ... 文件

<!-- DDM:TURN_SEP:v1 -->

## AI

## 完整审查结果

### 修复的问题

| # | 位置 | 原文 | 实际代码 | 修复 |
|---|---|---|---|---|
| P1 | WO-P3D | `openViewer` / `showDoc` | viewer.js 实际导出函数为 `openDoc(entry, layer)` | ✅ 已改 |
| P2 | WO-P0 | 未提及 `import threading` | server.py 当前无此 import，WO-P2B 需用 `threading.Thread` | ✅ 已补 |

### 无问题确认

- `learning-ai-llm`：本地目录 `/Users/lulu/Code/learning-ai-llm` 存在，与 topics.json 一致 ✅
- `js/api.js` 现有函数无重名，3 个新函数可直接追加 ✅
- `#md-panel` 实为 `flex-direction: column`，与 plan.md WO-P3A 方案完全吻合 ✅

两文件内容完全相同（diff 为空），`dialogue-knowledge-management-guide.md` 已删除。

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 19）

按照Plan，执行。

<!-- DDM:TURN_SEP:v1 -->

## AI

所有 9 个 WO 全部完成：

| WO | 内容 | 文件 |
|---|---|---|
| WO-P0 | server.py 基础设施：import threading、meili 配置常量、_load_meili_config()、_meili_request()、_meili_upsert_doc()、_reindex_job | server.py |
| WO-P1 | 全量/增量索引构建脚本 | scripts/build_knowledge_index.py（新建）|
| WO-P2A | GET /api/search-knowledge：读 meili-meta.json 判断过期，调 Meilisearch POST search，返回 {hits, stale, last_indexed_at} | server.py |
| WO-P2B | POST /api/reindex-knowledge（异步 threading.Thread）+ GET /api/reindex-status | server.py |
| WO-P2C | _handle_settle() 追加单条 upsert | server.py |
| WO-P3A | #md-content-row wrapper + #knowledge-panel div | index.html + app.css |
| WO-P3B | searchKnowledge / reindexKnowledge / getReindexStatus | js/api.js |
| WO-P3C | 面板组件：状态机、搜索、同步、渲染 | js/components/knowledge-search.js（新建）|
| WO-P3D | openDoc() 末尾调用 mountKnowledgeSearch + triggerKnowledgeSearch | js/components/viewer.js |

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 20）

是否可以停止，我自己手动安装。

<!-- DDM:TURN_SEP:v1 -->

## AI

已停止。安装完成后，执行以下步骤：

```bash
# 1. 启动 Meilisearch（V1）
meilisearch --master-key=lulu-workbench-meili-key-2026

# 2. 新开终端，建索引（V2）
cd /Users/lulu/Code/lulu-workbench
python3 scripts/build_knowledge_index.py --wipe

# 3. 启动 server，验证搜索 API（V3）
curl "localhost:8765/api/search-knowledge?q=TPM"
```

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 21）

Thank you for using Meilisearch!
...
2026-05-07T09:49:50 INFO actix_server::server: Actix runtime found; starting in Actix runtime
2026-05-07T09:49:50 INFO ... listening on: 127.0.0.1:7700

<!-- DDM:TURN_SEP:v1 -->

## AI

Meilisearch 已启动成功。运行建索引脚本：`python3 scripts/build_knowledge_index.py --wipe`

**第一次运行报错**：`ERROR: Cannot reach Meilisearch at http://localhost:7700: HTTP Error 503`

诊断：`curl localhost:7700/health` 返回 200，但 Python urllib 返回 503。根本原因：macOS 系统代理被 Python urllib 遵守，localhost 请求被代理拦截。

**修复**：在 `build_knowledge_index.py` 和 `server.py` 的 `_meili_request` 中均使用 `urllib.request.build_opener(urllib.request.ProxyHandler({}))` 替代默认 opener，绕过系统代理。

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 22）

执行：dtd_normalize， 主题： LuLu-Workbench 支持搜索 - meilisearch接入

（执行中，但搜索不到内容——索引写入失败）

<!-- DDM:TURN_SEP:v1 -->

## AI

诊断索引为空：

**第一个问题**：文档 ID 包含 `.`（来自 `.md` 扩展名），Meilisearch 只允许 `a-z A-Z 0-9 - _`。所有批次写入失败。

**修复**：ID 生成改用 `re.sub(r'[^a-zA-Z0-9\-_]', '_', raw_id)[:511]`——将所有非法字符（包括 `.`、中文字符、空格）替换为 `_`，并截断到 511 字节。

第二次重建：752 文档全部写入成功（14 个有效仓库，`product-analysis` 本地目录不存在跳过）。

server.py 需要重启以加载新路由。重启后 `curl "localhost:8765/api/search-knowledge?q=TPM"` 即可验证。

<!-- DDM:TURN_SEP:v1 -->
