# FeedVault 产品设计与规格迭代 — DDM Raw 归档

> 创建时间：2026年05月04日 12:05 （东八区）

> **归一化处理（P0）**：剔除 assistant 中英文内部独白、Turn/CHAT 例行行、`Let me` / `The user` 复述与多余工具元数据；保留用户原文、中文交付段落与 fenced 代码。

> 导航：[distilled](../../../distilled/common-tech/feedvault-product-architecture-session/202605041144-feedvault-product-architecture-session.md) · [digest](../../../digest/common-tech/feedvault-product-architecture-session/202605041144-feedvault-product-architecture-session.md) · [trace](../../../trace/common-tech/feedvault-product-architecture-session/202605041144-feedvault-product-architecture-session.md)

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 1）

<timestamp>Monday, May 4, 2026, 9:15 AM (UTC+8)</timestamp>
<user_query>
我想做一个topic 信息流的应用，它包含了： A 前台的 website，B  skill，用来做数据更新；C 使用github作为数据存储

A website
- 可以根据分类 -> topic 展示不同分类的topic，topic下面按照信息来源分类展示同步到的信息。
- 可以对topic管理： 添加分类、添加分类下的topic、设置某个topic的信息来源。
- 信息来源可以管理，通用的信息来源：例如 youtuber / x 平台，这是所有topic都一样的，但是某个topic可以选择需要哪几个：另外，某个topic 还可以添加自己特殊的信息来源，例如Andrej Karpathy  的topic，添加 Andrej Karpathy  自己的个人网站。
- topic下的信息展示，按照来源分类。不过，暂时分为：文本 / 视频。 弄2个tab切换，每个tab弄一个表格展示即可。

B skill 做数据更新
- B1 一个整体的skill流程控制器
- B2  流程1 获取所有topic列表 驱动 B3 执行
- B3  根据topic下的信息源，搜索数据，并整理更新。

C 使用github作为数据存储 
- 设计一套数据结构，使用github承载。

</user_query>

<!-- DDM:TURN_SEP:v1 -->

## AI

用户意图：对这个「topic 信息流」系统做完整的架构规划，明确 A/B/C 三部分的设计方案，以便后续实施。

<!-- DDM:TURN_SEP:v1 -->

## AI

好的，我先把三部分的完整设计方案梳理出来，确认后再动手。

---

## 架构设计方案

### C：GitHub 数据存储结构

建议用一个独立的 GitHub 仓库（如 `topic-feed-data`），以 JSON 文件组织数据，通过 GitHub API 读写。

```
data/
  config/
    categories.json      # 分类列表
    topics.json          # topic 列表（含所属分类、绑定的信息来源）
    sources.json         # 全局信息来源注册表（YouTube / X 等）
  feeds/
    {topicId}/
      text.json          # 该 topic 的文本类信息条目
      video.json         # 该 topic 的视频类信息条目
```

**各文件结构：**

```jsonc
// categories.json
[{ "id": "ai", "name": "AI", "description": "..." }]

// sources.json（全局来源注册表）
[{ "id": "youtube", "type": "youtube", "name": "YouTube", "config": {} },
 { "id": "x-twitter", "type": "x", "name": "X / Twitter", "config": {} }]

// topics.json
[{
  "id": "andrej-karpathy",
  "categoryId": "ai",
  "name": "Andrej Karpathy",
  "sources": [
    { "sourceId": "youtube", "config": { "channelId": "UCbXgNpp0jedKWcQiULLbDTA" } },
    { "sourceId": "x-twitter", "config": { "handle": "karpathy" } },
    { "type": "website", "name": "Karpathy Blog", "url": "https://karpathy.github.io" }
    // ↑ topic 独有的来源，直接内联，不进全局注册表
  ]
}]

// feeds/{topicId}/text.json
[{ "id": "...", "title": "...", "url": "...", "summary": "...", "publishedAt": "...", "sourceId": "...", "sourceName": "..." }]

// feeds/{topicId}/video.json
[{ "id": "...", "title": "...", "url": "...", "thumbnail": "...", "publishedAt": "...", "sourceId": "...", "sourceName": "..." }]
```

**读写策略：** 前台通过 GitHub Raw URL 或 GitHub Contents API 读；Skill（B）通过 GitHub API + commit 方式写。

---

### A：前台 Website

**技术选型：** Next.js（App Router）+ Tailwind CSS，部署到 GitHub Pages 或 Vercel。

**页面结构：**

```
/                        → 首页，显示所有分类
/{categoryId}            → 某分类下的 topic 列表
/{categoryId}/{topicId}  → topic 详情页（文本 Tab / 视频 Tab）

/manage                  → 管理入口
/manage/categories       → 分类管理（增删）
/manage/topics           → topic 管理（增删、绑定来源）
/manage/sources          → 全局信息来源管理
```

**Topic 详情页 UI：**
- 两个 Tab（文本 / 视频）
- 每个 Tab 一个表格，列：标题、来源、发布时间、链接

**管理页认证：** 简单的 GitHub Token（存 localStorage），写操作通过 GitHub API 提交。

---

### B：Skill（数据更新流程）

作为 Cursor Skill 实现，分三层：

**B1 — 整体流程控制器** `skill-update-feeds.md`
- 入口，负责协调 B2 → B3
- 支持全量更新 / 指定 topicId 更新

**B2 — 获取 topic 列表**
- 读 `data/config/topics.json`
- 输出：带来源配置的 topic 列表

**B3 — 单 topic 数据更新**
- 输入：一个 topic 及其来源配置
- 对每个来源，调用对应的抓取逻辑（YouTube API / X API / RSS / 网页抓取）
- 合并去重（按 `id` 或 `url`），追加到对应的 `feeds/{topicId}/text.json` 或 `video.json`
- 通过 GitHub API commit 写回

```
B1: skill-update-feeds
  ├── B2: fetch-topic-list（读 GitHub topics.json）
  └── B3: update-topic-feed（循环，对每个 topic 执行）
        ├── 来源适配器：youtube-fetcher
        ├── 来源适配器：x-fetcher
        ├── 来源适配器：rss-fetcher（通用网站/博客）
        └── 写回 GitHub
```

---

## 需要你确认的决策点

在开始实施前，有几个关键选择需要你明确：

1. **数据仓库**：新建一个独立的 `topic-feed-data` 仓库，还是在当前这个 Java 仓库里开一个 `data/` 目录？（建议独立仓库，职责清晰）

2. **Website 框架**：Next.js（功能完整，部署稍复杂）还是纯静态 HTML+JS（简单，但管理功能受限）？

3. **Website 部署**：GitHub Pages 还是 Vercel？

4. **Skill 类型**：这里的 Skill 是指 Cursor Agent Skill（`.md` 文件，由 AI 执行），还是独立的 Node.js/Python 脚本（可定时运行在 GitHub Actions）？

5. **信息来源的抓取方式**：YouTube 和 X 需要 API Key，你是否已有？或者希望先用 RSS/Mock 数据占位，后续再接真实 API？

6. **实施顺序**：是从 C（数据结构）→ A（网站）→ B（更新 Skill），还是你有其他偏好？

---

请确认或修正以上方案，我再开始逐步实施。

> 模型：Claude Sonnet 4.5

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 2）

<timestamp>Monday, May 4, 2026, 9:25 AM (UTC+8)</timestamp>
<user_query>
C：GitHub 数据存储结构
这里注意，每一条数据，需要分开，也就是 text / video 需要是一个目录。并且，每一条Item，文件名 以 YYYYMMDDHHMMSS + randam ID.md 。每一条Item内容需要设计一个数据结构。

A：前台 Website
页面需要本地可以启动使用，可以部署到 GitHub Pages 或 Vercel。

关于你的问题，我会新建一个独立的github仓库，skill，web，data都在这个仓库里面。 名字 topic-feed-data 不行，需要一个产品化的名字

可以使用 Next.js，但是我之前使用的是Python作为web服务。你觉得怎么样？  部署到 GitHub Pages，哪一类更合适？ 考虑另外一个点，我需要做文件修改操作，需要再website里面编辑，并gh提交到github

Skill 类型：这里的 Skill 是指 Cursor Agent Skill

信息来源的抓取方式： 你刚刚给我搜索的数据不可以吗？ 没有 API Key也可以吧？ 

C（数据结构）→ B（更新 Skill）→ A（网站）

</user_query>

<!-- DDM:TURN_SEP:v1 -->

## AI

用户意图：在上一轮方案基础上，根据补充说明确认剩余关键决策点，然后出完整计划。

在出计划前，有两个关键问题需要你确认：

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 3）

<timestamp>Monday, May 4, 2026, 9:28 AM (UTC+8)</timestamp>
<user_query>
刚刚断了，请重试
</user_query>

<!-- DDM:TURN_SEP:v1 -->

## AI

好的，重新问：

<!-- DDM:TURN_SEP:v1 -->

## AI

产品名确认为 **FeedVault**。关于认证方式，第二个问题需要补充确认：

<!-- DDM:TURN_SEP:v1 -->

## AI

✅ 可以。GitHub 目前支持 **Fine-grained PAT（细粒度个人访问令牌）**，可以精确设置为：

- 仅限 **单个仓库**（FeedVault）
- 仅开放 `Contents: Read & Write` 权限
- 其他仓库和权限完全隔离

这样即使 PAT 泄露，攻击者也只能操作这一个仓库的内容，安全风险可控。这个方案可以继续用。

---

现在我已有所有必要信息，开始出完整计划。

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 4）

<timestamp>Monday, May 4, 2026, 9:33 AM (UTC+8)</timestamp>
<user_query>
你需要新建一个.cache/design 目录，并且将plan分为4个文件： 产品介绍； 整体架构设计；数据仓库设计； Skill 设计； Next.js 前台网站设计。想做这个拆分。 
</user_query>

<!-- DDM:TURN_SEP:v1 -->

## AI

用户意图：将计划拆分为 5 个独立设计文档（你列了 5 个主题，但说的 4 个，我按 5 个主题执行），存入 `.cache/design/`。

<!-- DDM:TURN_SEP:v1 -->

## AI

5 个设计文档已创建完毕：

```
.cache/design/
  01-product-introduction.md      # 产品介绍
  02-system-architecture.md       # 整体架构设计（含 Mermaid 流程图）
  03-data-repository-design.md    # 数据仓库设计（目录结构、JSON Schema、Item 文件格式）
  04-skill-design.md              # Skill 设计（B1/B2/B3 流程 + 抓取策略）
  05-nextjs-website-design.md     # Next.js 前台网站设计（页面结构、API 封装、部署配置）
```

你列了 5 个主题，我按 5 个文件拆分了（你说的 4 个应该是笔误）。

确认这份设计没问题后，就可以按 **C → B → A** 的顺序开始实施了。

> 模型：Claude Sonnet 4.6

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 5）

<timestamp>Monday, May 4, 2026, 9:42 AM (UTC+8)</timestamp>
<user_query>
关于Skill数据更新： Skill是通用的，可以通过任何AI工具触发，不过需要指定更新的数据目录。并且，默认应该是选择某个topic 更新，或者某一组topic 更新。

另外，考虑Topic可能非常多，Web Site展示上，可能需要调整一下。 Topic设计是按照分类 / Topic / 数据源 这样分类的。但是，Web Site展示上，需要单独设计一个关注分类，这个分类是按照用户喜好分组的，例如：科技，包括了Topic A Topic B，Topic A Topic B可能是不同的分类。

并且，Skill的更新功能，可以是复制Websit的某个topic id就行更新。或者，直接复制某个关注分类。更新某个列表topic。
</user_query>

<!-- DDM:TURN_SEP:v1 -->

## AI

用户意图：对计划做两处关键调整：
1. Skill 通用化（不绑定 Cursor，支持指定目录 + 单/组 topic 触发）
2. Website 新增「关注分类」（Watchlist）概念，作为跨分类的个性化浏览入口

我来更新三个设计文档：数据仓库、Skill、网站设计。

<!-- DDM:TURN_SEP:v1 -->

## AI

现在更新 Skill 设计文档：

<!-- DDM:TURN_SEP:v1 -->

## AI

现在更新网站设计，加入 Watchlist 概念并重新设计导航：

<!-- DDM:TURN_SEP:v1 -->

## AI

同步更新整体架构文档中的 Watchlist 内容：

<!-- DDM:TURN_SEP:v1 -->

## AI

设计文档更新完成，变更摘要如下：

---

**本轮变更了 3 个文档：**

**`03-data-repository-design.md`**
- `config/` 新增 `watchlists.json`
- 说明 Watchlist 是浏览维度，Category 是管理维度，两者独立
- 同一个 Topic 可出现在多个 Watchlist 中

**`04-skill-design.md`**（重写）
- Skill 去掉 Cursor 绑定，通用化为任何 AI 工具可触发
- 所有操作通过 `dataDir` 参数指定数据目录，支持多环境
- 触发粒度：`topic=` / `topics=` / `watchlist=`，**不默认全量**
- 无范围时提示选择，不自动执行

**`05-nextjs-website-design.md`**（重写）
- 首页改为 **Watchlist 列表**，这是日常浏览主入口
- 新增 `/watchlist/[watchlistId]` 页、保留 `/browse` 按 Category 浏览
- Topic URL 改为 `/topic/[topicId]`（不依赖 Category 层级）
- Topic 详情页和 Watchlist 详情页各有**一键复制 Skill 触发命令**按钮
- 管理页新增 Watchlist CRUD

> 模型：Claude Sonnet 4.6

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 6）

<timestamp>Monday, May 4, 2026, 9:49 AM (UTC+8)</timestamp>
<user_query>
@.cache/design/01-product-introduction.md  这个文档没有修改， 信息浏览。按 分类 → Topic 层级组织，快速导航到感兴趣的主题。需要特别说清楚，UI上的创建的是兴趣 -> Topic。实际内容组织是 人物 / 产品 -> Topic分类。 这是2个概念。 兴趣 -> Topic 可以随时创建，修改，删除。 人物 / 产品 -> Topic分类 属于源数据管理，设计上属于后台管理的内容。 Web Site也需要把前台UI 和后台管理，弄成2个入口。默认是现实 前台UI 。 
</user_query>

<!-- DDM:TURN_SEP:v1 -->

## AI

用户意图：更新产品介绍文档，明确「兴趣分类 → Topic」与「人物/产品分类 → Topic」是两个独立概念；并同步更新网站设计，将前台 UI 和后台管理设计为两个明确入口。

<!-- DDM:TURN_SEP:v1 -->

## AI

同步更新网站设计文档，把前台/后台入口分离说清楚：

<!-- DDM:TURN_SEP:v1 -->

## AI

同步更新目录结构：

<!-- DDM:TURN_SEP:v1 -->

## AI

更新完成，本轮核心变更：

**`01-product-introduction.md`（重写）**
- 单独一节「两个核心概念」，用类比和对比表格明确区分：
  - **兴趣分类**（前台，用户视角，随时增删）= 浏览器收藏夹
  - **人物/产品分类**（后台，数据视角，低频变更）= RSS 订阅源列表
- 网站入口设计说明：默认进前台，后台是独立入口

**`05-nextjs-website-design.md`（更新）**
- 路径规范：前台无前缀（`/`），后台统一 `/admin` 前缀
- 后台 Layout 独立，顶部导航与前台完全隔离
- 前台首页导航只有「我的兴趣」，右上角有次要的「后台管理 →」入口
- 移除了前台的 `/browse` 按 Category 浏览（Category 是后台概念，不暴露到前台）

> 模型：Claude Sonnet 4.6

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 7）

<timestamp>Monday, May 4, 2026, 9:54 AM (UTC+8)</timestamp>
<user_query>
后台管理（独立入口），默认不展示，需要确认权限才展示。不过，这里设计一个安全设计。 如果公开作为github pages上面，那如何展示这个入口呢？ 我希望只有我自己可以看到  后台管理（独立入口）。并且，我不希望通过账户来管理权限。 
</user_query>

<!-- DDM:TURN_SEP:v1 -->

## AI

用户意图：为 GitHub Pages 静态站设计一个「无账户、只有自己能看到后台入口」的轻量安全方案。先分析选项，再更新设计文档。

---

## 静态站的无账户隐藏后台：方案分析

GitHub Pages 是纯静态托管，没有服务端，所有鉴权逻辑只能在浏览器端做。以下是三个可行方案：

### 方案 A：魔法 URL（Hash 片段解锁）

访问 `https://you.github.io/feedvault/#admin=your-secret`，JS 读取 `#` 后的片段，与构建时内嵌的 **SHA-256 哈希值**比对，匹配则在 localStorage 写入解锁标记，后台入口出现在导航。

- Hash 片段（`#` 之后）**不会发送到服务器**，不出现在 GitHub Pages 访问日志
- 书签保存这个 URL，日常访问无需输入任何内容
- 源码里只存 SHA-256 哈希，不存原始 secret
- 若有人拿到你的书签 URL，可以访问——但这是个人工具，风险可控

### 方案 B：管理密码（本地哈希校验）

进入 `/admin` 时显示密码输入框，输入 passphrase → JS 计算 SHA-256 → 与构建内嵌哈希比对 → 匹配则解锁并写 localStorage。

- 每次需要输入密码（或依赖 localStorage 维持会话）
- 无 URL 泄露风险
- 代码审计者能看到哈希但无法反推密码

### 方案 C：隐藏路径（Security by obscurity）

后台路径不是 `/admin` 而是 `/admin-{随机 32 位字符串}`，路径本身就是 secret，无校验逻辑。

- 最简单，一个书签搞定
- 路径本身就是密钥，但静态文件的路径在 HTML 源码中可能被间接发现（如 sitemap、`_next/` 静态资源引用）
- **不推荐**：Next.js 静态导出会在页面 JS bundle 中包含所有路由

---

### 推荐：方案 A + B 组合

**初次解锁用魔法 URL（A），后续会话用 localStorage 持久化，重置时清除 localStorage 即可。**

```
解锁流程：
1. 书签保存：https://you.github.io/feedvault/#admin=your-secret
2. 访问时 JS 读取 hash，SHA-256 校验
3. 通过 → localStorage.setItem('feedvault_admin_token', hashedSecret)
4. 后台入口出现，直到 localStorage 被清除或浏览器隐私模式

日常使用：
- 普通访问 feedvault 首页，没有任何后台入口
- 浏览器书签一键解锁（或 localStorage 已有 token 自动解锁）
- 换设备/清缓存后重新用书签解锁
```

**Secret 管理：**
- 原始 secret 只存在你本地的 `.env.local`（不提交 git）
- 构建时通过 `NEXT_PUBLIC_ADMIN_HASH` 环境变量注入 SHA-256 值
- GitHub Actions 构建时从 GitHub Secrets 读取该哈希
- 任何人审查源码只能看到哈希，无法反推 secret

---

现在更新设计文档：

<!-- DDM:TURN_SEP:v1 -->

## AI

设计文档更新完毕。

---

**安全方案小结：**

这个方案的本质是**双层保护**：

- **第一层（隐藏）**：后台入口对所有访客不可见，无法被「碰巧发现」
- **第二层（写保护）**：即使有人拿到书签 URL 访问了后台，所有修改操作仍需 GitHub Fine-grained PAT，没有 PAT 只能看界面，什么都改不了

所以即使 secret 泄露，实际风险仍然有限——攻击者最多能看到你的 Topic 配置，不能修改任何数据。

> 模型：Claude Sonnet 4.6

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 8）

<timestamp>Monday, May 4, 2026, 10:05 AM (UTC+8)</timestamp>
<user_query>
这个问题，先不解决吧 作为后续更新解决，一期只考虑本地启动服务使用。 
</user_query>

<!-- DDM:TURN_SEP:v1 -->

## AI

收到，后台安全机制移出一期范围，后续迭代再解决。一期只考虑本地启动使用，后台直接可访问。

更新两个文档：

<!-- DDM:TURN_SEP:v1 -->

## AI

好，一期后台无鉴权，本地 `npm run dev` 直接访问 `/admin`，干净简单。

设计文档现在的状态是完整的一期方案，后台安全作为 backlog 记录在案。可以开始实施了吗？

> 模型：Claude Sonnet 4.6

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 9）

<timestamp>Monday, May 4, 2026, 10:08 AM (UTC+8)</timestamp>
<user_query>
1 后台入口，默认放到右上角，不需要： **后台入口默认隐藏，通过「魔法 URL」解锁，无需账户体系。**
2 本产品设计，需要细化一下。 概念二：人物/产品分类（后台数据管理维度） 需要描述清楚，交互路径是怎么样的。用户可以添加分类，添加分类的topic，设置topic数据源。 topic common数据源设置。  
</user_query>

<!-- DDM:TURN_SEP:v1 -->

## AI

用户意图：
1. 后台入口改为默认显示在右上角（去掉隐藏描述）
2. 产品介绍中「概念二」补充完整的交互路径

<!-- DDM:TURN_SEP:v1 -->

## AI

同步更新网站设计中后台入口的描述：

<!-- DDM:TURN_SEP:v1 -->

## AI

完成。本轮变更：

**`01-product-introduction.md`**
- 后台入口改为「右上角固定显示」，去掉魔法 URL 隐藏描述
- 「概念二」补充了完整的 4 步交互路径：
  - Step 1：添加分类
  - Step 2：分类下添加 Topic
  - Step 3：为 Topic 配置来源（通用数据源勾选+填参数 / 私有数据源独立添加）
  - Step 4：管理通用数据源库（所有 Topic 可选的来源类型注册表）

**`05-nextjs-website-design.md`**
- 后台入口描述更新为右上角固定可见

> 模型：Claude Sonnet 4.6

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 10）

<timestamp>Monday, May 4, 2026, 10:14 AM (UTC+8)</timestamp>
<user_query>
关于数据抓取，有几个方式：

1 通过Skill自动搜索整理  Step 4：管理通用数据源库 只配置粗糙的URL 域名
2 通过你说的RSS等方式

请对比2种方式的优劣。 
</user_query>

<!-- DDM:TURN_SEP:v1 -->

## AI

用户意图：对比两种数据抓取方式的优劣，作为设计决策参考。

---

## 两种抓取方式对比

### 方式一：Skill AI 搜索整理（只配置粗糙域名）

来源库只存 `x.com/karpathy`、`karpathy.github.io` 这样的入口 URL，Skill 调用 WebSearch / WebFetch 让 AI 自己理解页面、提取内容。

**优点：**
- 配置极简，用户只需填一个 URL/handle，无需了解 RSS 地址、channelId 等技术细节
- 对没有 RSS 的来源（如部分个人网站、论坛）也能工作
- AI 能跨格式理解内容，结构不规则的页面也能处理

**缺点：**
- **结果不稳定**：每次抓取 AI 理解可能有差异，同一篇内容可能标题/摘要不一致
- **容易漏内容**：AI 每次只看当前页面快照，历史内容可能抓不全
- **被反爬限制**：直接 WebFetch 正文页面，大量请求容易触发 cloudflare / rate limit
- **成本高**：每次更新都要 AI 理解页面，消耗更多 token
- **难以调试**：出错时不知道是网页结构变了还是 AI 理解偏了

---

### 方式二：RSS / 结构化方式（配置精确参数）

来源库存结构化参数（YouTube `channelId`、X `handle`、博客 `rssUrl`），Skill 直接 WebFetch 对应的 RSS/Atom XML，解析标准字段。

**优点：**
- **结果稳定可靠**：RSS 是结构化数据，标题/链接/时间字段固定，解析无歧义
- **覆盖历史内容**：RSS feed 通常包含最近 N 条，不依赖 AI 判断
- **不易被封**：访问 RSS endpoint 是合法的订阅行为，平台主动提供
- **无需 AI token**：纯 HTTP 请求 + XML 解析，运行成本极低
- **易于调试**：直接打开 RSS URL 就能看到原始数据

**缺点：**
- 配置需要多一步（找到 channelId 或确认 rssUrl），对非技术用户有门槛
- 部分来源没有 RSS（X 原生不支持，需要 Nitter 中转；Nitter 实例可能不稳定）
- RSS 内容通常是摘要，正文需要额外 fetch

---

## 综合建议：分层混合策略

| 来源类型 | 推荐方式 | 说明 |
|----------|----------|------|
| YouTube | RSS（方式二） | YouTube 官方提供 RSS，稳定可靠 |
| 个人博客 / 技术网站 | RSS（方式二） | 大多数博客有 Atom/RSS，优先使用 |
| X / Twitter | Nitter RSS（方式二）| Nitter 不可用时降级为方式一 |
| 无 RSS 的来源 | AI 搜索（方式一）| 作为兜底，仅在 RSS 不存在时使用 |

**数据源库的配置字段设计应支持两种方式：**

```json
{
  "type": "youtube",
  "fetchMode": "rss",                         // 方式二
  "rssTemplate": "youtube.com/feeds/videos.xml?channel_id={channelId}"
}

{
  "type": "personal-site",
  "fetchMode": "ai-search",                   // 方式一（兜底）
  "urlTemplate": "{url}"
}
```

这样 Skill 在运行时先尝试 RSS，RSS 失败或不存在再降级到 AI 搜索，兼顾稳定性和覆盖范围。

---

这个对比需要更新到设计文档吗？还是确认策略后直接开始实施？

> 模型：Claude Sonnet 4.6

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 11）

<timestamp>Monday, May 4, 2026, 10:16 AM (UTC+8)</timestamp>
<user_query>
可以，放进去吧 
</user_query>

<!-- DDM:TURN_SEP:v1 -->

## AI

同步更新 `03-data-repository-design.md` 中 sources.json 的 `fetchMode` 字段：

<!-- DDM:TURN_SEP:v1 -->

## AI

完成。对比内容已写入 `04-skill-design.md`（新增「数据抓取策略」章节），`sources.json` 结构同步加上了 `fetchMode` 和 `fallback` 字段。

> 模型：Claude Sonnet 4.6

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 12）

<timestamp>Monday, May 4, 2026, 10:19 AM (UTC+8)</timestamp>
<user_query>
概念一：兴趣分类（前台 UI 维度） 这里没有体现  兴趣分类管理，需要再前台UI设计添加 / 删除 兴趣分类，添加 / 删除 分类下的Topic。

</user_query>

<!-- DDM:TURN_SEP:v1 -->

## AI

用户意图：在「概念一」和前台 UI 中明确加入兴趣分类的管理能力（增删兴趣、增删兴趣下的 Topic），这是前台功能，不放在后台。

<!-- DDM:TURN_SEP:v1 -->

## AI

同步更新网站设计中首页和兴趣详情页的 UI 描述：

<!-- DDM:TURN_SEP:v1 -->

## AI

同步把后台管理页中兴趣分类管理的说明更新为「也可从后台统一管理」，避免职责模糊：

<!-- DDM:TURN_SEP:v1 -->

## AI

完成。本轮变更：

**`01-product-introduction.md`**
- 「概念一」补充了前台管理交互路径（含新增兴趣、删除兴趣、添加/移除 Topic 的完整路径图）
- 「核心功能 - 前台 UI」更新为体现管理能力的描述

**`05-nextjs-website-design.md`**
- 首页：新增内联管理操作（`+ 新增兴趣`、每张卡片的删除按钮）
- 兴趣详情页：新增内联管理操作（`+ 添加 Topic`、每张 Topic 卡片的移除按钮）
- 后台 `/admin/watchlists`：定位调整为「统一视图 / 批量编辑」，日常操作以前台为主

> 模型：Claude Sonnet 4.6

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 13）

<timestamp>Monday, May 4, 2026, 10:22 AM (UTC+8)</timestamp>
<user_query>
请你逐行审查产品文档：
1、列出有内容遗漏的导致完备性缺失的内容； 
2、列出文档内部，前后逻辑不一致的内容；
3、列出文档内部，描述存在二义性的内容；

以上，一并给出优化方案。 
</user_query>

<!-- DDM:TURN_SEP:v1 -->

## AI

逐行审查完毕，共发现 **14 个问题**，分三类列出并给出优化方案：

---

## 一、完备性缺失（6 项）

**O1 — 「修改」兴趣名称/描述的路径缺失**
- 位置：Line 20 说兴趣分类可「创建、修改、删除」，但 Line 24-31 的交互路径图中只有新增和删除，没有「编辑兴趣名称/描述」的操作入口。
- 优化：交互路径图中补充 `兴趣卡片 [编辑] → 修改名称/描述 → 保存`

**O2 — 前台写操作的 PAT 前提未说明**
- 位置：Line 133-137 前台功能中，新增兴趣、删除兴趣、添加/移除 Topic 都是写操作，需要 PAT，但完全未提及。用户若未配置 PAT，点击「新增兴趣」会静默失败。
- 优化：前台功能描述中补充「写操作需要提前在后台设置页配置 GitHub PAT；未配置时写操作不可用，提示引导配置」

**O3 — 「从所有 Topic 中选择」的前提依赖未说明**
- 位置：Line 29「从所有 Topic 中搜索/选择」，但未说明「所有 Topic」来自哪里——需要先在后台创建 Topic 才能在前台选择。若后台无 Topic，面板为空，用户不知道为什么。
- 优化：补充「所有 Topic 指后台已创建的 Topic；若列表为空，提示前往后台新增」

**O4 — 数据更新后内容展示时机未说明**
- 位置：Line 151-152 说更新结果写入 GitHub，但未说明用户何时能在网站上看到更新内容。静态站在本地需要重新构建，在 GitHub Pages 需要重新部署。
- 优化：补充「本地使用时，Skill 写入数据后需重新执行 `npm run build` 或 `npm run dev` 刷新；GitHub Pages 部署时，push 触发 Actions 自动重建」

**O5 — 私有数据源的抓取方式未说明**
- 位置：Step 3（Line 80-82）举例了 Karpathy Blog 作为私有数据源，只有 URL，但未说明系统如何判断用哪种方式抓取（RSS 还是 AI 搜索）。
- 优化：补充「私有数据源填写 URL 后，Skill 优先尝试在该 URL 下发现 RSS；若无则降级为 AI 搜索抓取」

**O6 — 产品介绍层面缺少 Skill 命令示例**
- 位置：Line 137 提到「一键复制更新命令」，但产品介绍里没有给出命令格式的示例，读者无法直观理解。
- 优化：补充示例 `feedvault update topic=andrej-karpathy` 和 `feedvault update watchlist=my-tech`

---

## 二、逻辑不一致（3 项）

**I1 — 兴趣分类管理同时出现在前台和后台，边界模糊**
- 位置：Line 20/22-31 定义兴趣分类是前台功能；Line 143 后台功能列表中又写「管理兴趣分类（增删兴趣、将 Topic 加入/移出某个兴趣）」，完全覆盖了前台的职责。
- 优化：后台的兴趣分类入口定位为「统一视图/批量操作」，日常增删操作以前台为主。后台描述改为「兴趣分类统一视图（批量管理，主要入口在前台）」

**I2 — 入口结构图将「管理兴趣分类」列在后台列**
- 位置：Line 117-125 的入口对照图，后台列写了「管理兴趣分类」，但前台已有兴趣的完整增删能力，图中前台列却没有体现，造成职责倒置的视觉误导。
- 优化：入口图前台列补充「兴趣分类管理（新增/删除/编辑）」，后台列的「管理兴趣分类」改为「兴趣分类统一视图」

**I3 — 「一期仅本地使用」与「技术特点：GitHub Pages」矛盾**
- 位置：Line 127「一期仅本地启动使用」；Line 158「零服务器成本：网站静态部署（GitHub Pages）」，暗示已部署到 GitHub Pages。两句话方向相反。
- 优化：技术特点里的 GitHub Pages 描述加注「设计支持部署到 GitHub Pages，一期以本地启动为主」；两处措辞统一口径

---

## 三、描述二义性（5 项）

**A1 — Topic 在 Line 5 和 Line 41 的定义侧重不同**
- Line 5：「用户围绕自己感兴趣的**主题**（Topic）」→ Topic 像是用户定义的话题
- Line 41：「Topic 代表一个具体的**信息主体**，例如 Andrej Karpathy（人物）、Cursor（产品）」→ Topic 是具体的人/产品
- 优化：在产品定位处就给出 Topic 的准确定义，与 Line 41 一致：「Topic 是具体的信息主体（某个人物或产品），用户围绕这些主体组织信息源」

**A2 — 「Topic 数据」vs「Topic 本身」措辞不统一**
- Line 27：「不影响 Topic 数据」
- Line 31：「不删除 Topic 本身」
- 两句都想表达同一个意思，但「数据」和「本身」语义范围不同，读者会疑惑区别。
- 优化：统一为「不影响 Topic 本身及其 feed 数据」

**A3 — 「通用数据源」命名有歧义**
- 「通用」容易被误解为「免费/公开/通用协议」，实际含义是「平台级别的来源类型，所有 Topic 都可选择启用」。
- 优化：改名为「**平台来源**（所有 Topic 可选的来源类型）」；对应的「私有数据源」改为「**Topic 专属来源**」

**A4 — 「所有 Topic 中搜索/选择」范围不清**
- Line 29 的「所有 Topic」未限定范围，个人工具语境下应明确是「所有已在后台创建的 Topic」。
- 优化：改为「从后台已创建的 Topic 中搜索/选择」

**A5 — 「右上角」没有说明是哪个页面**
- Line 115 「后台管理入口默认显示在右上角」，未说明是前台每个页面的右上角（持续可见），还是仅首页。
- 优化：明确为「前台所有页面的顶部导航右上角，持续可见」

---

## 汇总：是否现在统一修入文档？

共 14 项，逐项修入大概需要重写大半文档。你选择：

1. **全部修入** — 我一次性更新 `01-product-introduction.md`
2. **只修关键问题** — 优先修 I1/I2/I3（逻辑矛盾）+ A1/A3（高频二义性），遗漏类后续补

> 模型：Claude Sonnet 4.6

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 14）

<timestamp>Monday, May 4, 2026, 10:27 AM (UTC+8)</timestamp>
<user_query>
全部补充完整 
</user_query>

<!-- DDM:TURN_SEP:v1 -->

## AI

完成，14 项问题全部修入。逐项对应：

| 问题 | 修改位置 |
|------|----------|
| O1 交互路径缺「编辑」 | 前台交互路径图补充 `[编辑]` 操作 |
| O2 PAT 前提未说明 | 交互路径图下方新增 `> 前提` 说明块 |
| O3 Topic 来源范围不清 | 「添加 Topic」改为「从后台已创建的 Topic 中选择」，补充空列表提示 |
| O4 更新后内容展示时机 | 「数据自动更新」节末尾补充本地/Pages 两种情况说明 |
| O5 专属来源抓取方式 | Step 3 补充「系统自动识别 RSS/降级 AI 搜索」 |
| O6 缺少命令示例 | 核心功能中补充两条命令示例 |
| I1 兴趣管理前后台边界 | 后台描述改为「兴趣分类统一视图（批量操作；日常以前台为主入口）」 |
| I2 入口图前台列缺兴趣管理 | 入口图前台列补充「兴趣管理（新增/编辑/删除）」和「Topic 管理（添加/移除）」 |
| I3 本地 vs GitHub Pages 矛盾 | 产品定位和技术特点措辞统一，加注「设计支持，一期以本地为主」 |
| A1 Topic 定义不一致 | 产品定位首句统一为「Topic 是具体的信息主体（某个人物或产品）」 |
| A2 「数据」vs「本身」措辞 | 全文统一为「不影响 Topic 本身及其 feed 数据」 |
| A3 「通用数据源」歧义 | 全文改为「平台来源」；「私有数据源」改为「Topic 专属来源」 |
| A4 「所有 Topic」范围不清 | 改为「后台已创建的 Topic」 |
| A5 「右上角」缺乏上下文 | 改为「前台所有页面的顶部导航右上角持续可见」 |

> 模型：Claude Sonnet 4.6

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 15）

<timestamp>Monday, May 4, 2026, 10:36 AM (UTC+8)</timestamp>
<user_query>
关于  一、完备性缺失；二、逻辑不一致；三、描述二义性，你觉得这是评估一个产品文档的完备的维度吗？ 还是这几个维度有更完善的版本？ 请你深度思考之后回答。
</user_query>

<!-- DDM:TURN_SEP:v1 -->

## AI

用户意图：深度评估「完备性 / 逻辑一致性 / 描述二义性」这三个维度是否足以覆盖产品文档质量评估的全部需要。

---

## 深度分析

### 我用的三个维度的本质

回看这三个维度：

- **完备性缺失**：该有的内容有没有写
- **逻辑不一致**：写了的内容互相矛盾吗
- **描述二义性**：写了的内容有没有多种解读

这三个维度本质上都是**文本层面的质量检查**，关注的是「字面上写得好不好」。它们是必要的，但只覆盖了产品文档质量的一个层面。产品文档还有更高层次的质量要求，现有三个维度完全没有触及。

---

### 缺失的维度

**第四维：可验证性（Testability）**

描述是否具体到可以被测试或接受？「一键复制更新命令」是功能描述，但没有接受条件：复制成功时有没有反馈？复制的格式是什么？「引导前往配置」是什么形式的引导？

现有三个维度不会发现这类问题，因为这些描述文字上没有矛盾，也不歧义，但无法被验证。

---

**第五维：假设显式化（Explicit Assumptions）**

产品文档中隐含了大量未声明的假设，一旦假设不成立，文档就会崩溃。例如：

- 「Nitter RSS 可用」是一个假设——Nitter 实例经常因政策原因下线
- 「GitHub Pages 可以处理 `/watchlist/[id]` 这样的动态路由」是一个假设——实际上 GitHub Pages 是纯静态托管，Next.js 的动态路由需要特殊处理（`output: export` + 所有路径预生成），否则刷新页面会 404
- 「用户的 GitHub Fine-grained PAT 不会过期」是一个假设

这些假设不写出来，实施时会踩坑，而现有三个维度无法发现它们。

---

**第六维：边界与异常流程（Edge Cases & Error Flows）**

产品文档通常只描述正常路径（Happy Path）。FeedVault 的文档现在也是这样：写了「保存兴趣」，没写「保存失败时怎么样」；写了「Skill 更新」，没写「某个来源 RSS 不可用时怎么处理」；写了「PAT 配置」，没写「PAT 无效或权限不足时的提示」。

这不是「完备性缺失」——那节点的功能描述是完整的——而是**流程的边界没有被定义**，属于独立维度。

---

**第七维：术语一致性（Terminology Consistency）**

这与「描述二义性」有交集但不同。二义性是「一句话有两种解读」，术语一致性是「同一个概念在文档不同位置用了不同词汇」。这需要一个显式的术语表（Glossary），并检查每处用词是否对齐。

例如在 FeedVault 文档里，「来源」「数据源」「信息来源」「来源类型」出现在不同地方，指的是同一个概念，但读者无法确认。这不是歧义（每处单独理解都没问题），但系统性术语混乱会导致实施时沟通成本增加。

---

**第八维：优先级与范围边界（Priority & Scope Boundary）**

文档描述了大量功能，但没有区分哪些是 MVP 必须有、哪些是一期可选、哪些是后续迭代。「后台安全机制作为后续迭代」是一处显式说明，但其他功能的优先级没有标注。开始实施时，面对完整的功能列表，不知道从哪里切入、什么可以先留空。

这不属于「完备性缺失」（功能都写了），也不属于「逻辑不一致」（互相不矛盾），而是**决策信息的缺失**。

---

### 完整的评估维度框架

| 层次 | 维度 | 核心问题 |
|------|------|----------|
| **文本质量** | 完备性 | 该有的内容有没有写？ |
| | 逻辑一致性 | 写了的内容有没有互相矛盾？ |
| | 描述无歧义性 | 写了的内容有没有多种解读？ |
| | 术语一致性 | 同一概念是否始终用同一词汇？ |
| **可执行性** | 可验证性 | 描述是否具体到可以被测试？ |
| | 假设显式化 | 隐含假设是否都被声明出来？ |
| | 技术可行性 | 描述是否与技术选型不冲突？ |
| **流程完整性** | 异常流程覆盖 | 非正常路径是否有描述？ |
| | 用户旅程完整 | 所有流程是否有明确的起点和终点？ |
| **决策支持** | 优先级与范围 | MVP vs 迭代 vs 排除项是否清晰？ |
| | 依赖关系 | 功能间的依赖和前提条件是否显式？ |

我用的三个维度只覆盖了第一层「文本质量」中的三条，且漏了「术语一致性」。剩余三个层次完全没有覆盖。

---

### 对 FeedVault 文档最重要的补充维度

按优先级排序，对当前阶段最有价值的是：

1. **假设显式化**：GitHub Pages 动态路由处理方式是一个会直接影响实施的技术假设，需要现在确认
2. **优先级与范围**：实施顺序 C→B→A 虽然定了，但每个部分内部哪些功能是 MVP，还不清楚
3. **异常流程**：至少要定义 PAT 失效、RSS 不可用、GitHub API 失败三个关键异常的处理预期

是否要针对这三个维度，对 FeedVault 文档做一次补充检查？

> 模型：Claude Sonnet 4.6

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 16）

<timestamp>Monday, May 4, 2026, 10:55 AM (UTC+8)</timestamp>
<user_query>

1 完整的评估维度框架，给我整理到md文档。
2 假设显式化：GitHub Pages，可以详细说一下吗？ 我不太懂：GitHub Pages 动态路由处理方式。
3 优先级可以按照C的功能做一下标柱，请你将功能做成依赖树，越在树的顶端，优先级越高。
4 是的。

此外，提一个题外话题： 对比 “把文档优化一下，看看有没有遗漏和错误。” vs 按照 评估维度框架 审查并优化产品文档。你觉得这是抽象维度的不同，还是内容结构化的不同，还是什么的不同？ 哪一种更好？ 为什么？ 
</user_query>

<!-- DDM:TURN_SEP:v1 -->

## AI

用户意图：4 件事并行 + 1 个元问题。先做元问题的回答（纯思考输出），再并行执行文档任务。

---

## 元问题：两种指令的本质差异

「把文档优化一下，看看有没有遗漏和错误」vs「按照评估维度框架审查并优化」——

这不是抽象维度的差异，也不是内容结构的差异，是**思维模型的显式化程度**不同。

第一种指令依赖执行者脑内隐含的质量标准。同一个人在不同时间执行，结果不一样；两个人执行，结果也不一样。「遗漏」「错误」这两个词看起来具体，其实是开放集合——你的注意力落在哪里，就发现哪里的问题，落不到的地方就看不见。这是一种**依赖直觉的扫描**，覆盖率由当下的状态决定。

第二种指令把「什么叫好」这件事先外化成一张清单，执行时沿着清单逐项走，每一项是否覆盖是可以判断的。这是**把认知结构嵌入流程**，把「我觉得」变成「按标准得」。

更深一层的差异：第一种问的是「结果怎么样」，第二种问的是「按什么标准衡量结果」。前者是执行层，后者是元层（meta-level）。框架让评估本身可被审计——你可以问「你按哪个维度检查的」，也可以事后说「这个问题漏掉了，因为框架里缺少 X 维度」，从而改进框架本身。

哪种更好？**框架驱动的更好**，原因是三点：
1. **覆盖率可保证**：每个维度都被检查过，不依赖状态
2. **可复用、可迭代**：框架本身可以被质疑和改进，积累为方法论
3. **输出可沟通**：两个人按同一框架评审，可以对齐分歧，而不是「我觉得 vs 你觉得」

但框架有一个局限：它只能发现框架内定义好的问题类型，对框架没有覆盖的维度是盲区。所以真正好的做法是：**框架驱动 + 最后留一个开放式通读**，补捉框架之外的直觉信号。

---

现在并行执行 4 个文档任务：

<!-- DDM:TURN_SEP:v1 -->

## AI

现在并行：GitHub Pages 路由说明写入文档，同时做 C 的依赖树：

<!-- DDM:TURN_SEP:v1 -->

## AI

现在做 C 的功能依赖树，写入数据仓库设计文档：

<!-- DDM:TURN_SEP:v1 -->

## AI

最后，针对三个新维度对产品文档做补充检查（任务 4），将发现写入产品介绍文档：

<!-- DDM:TURN_SEP:v1 -->

## AI

完成。本轮 4 件事全部执行：

**新建文件**
- `00-product-doc-evaluation-framework.md`：11 维度完整评估框架，含检查清单和修复优先级建议

**更新 `02-system-architecture.md`**
- 新增「GitHub Pages 与动态路由」章节，用类比说明问题根源（请求静态文件找不到 → 404），`generateStaticParams` 的解决方案，以及 3 条显式假设

**更新 `03-data-repository-design.md`**
- 新增 C 组件功能依赖树（Level 0~4，11 个子任务），每层有依赖说明，末尾有批次化实施顺序表

**更新 `01-product-introduction.md`**
- 新增「显式假设与约束」表（5 条假设 + 影响）
- 新增「关键异常流程」表（6 个场景的用户预期）
- 新增「功能优先级（一期 MVP）」：必须有 / 一期可选 / 后续迭代三档

> 模型：Claude Sonnet 4.6

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 17）

<timestamp>Monday, May 4, 2026, 11:04 AM (UTC+8)</timestamp>
<user_query>
1 请把 元问题：两种指令的本质差异，补充到  产品文档质量评估维度框架
2 功能优先级（一期 MVP） 使用功能依赖树作为模型，你觉得是一个完备的方案吗 ？这是否也是 元问题 对应的  思维模型的显式化。我可以把它理解成一种结构化思维吗？另外，从结构化形式看，是否可以理解为一种领域建模？ 
</user_query>

<!-- DDM:TURN_SEP:v1 -->

## AI

先回答 Task 2 的思考，再执行 Task 1。

---

## Task 2：深度回答

**功能依赖树是完备的优先级模型吗？**

不完备。依赖树只捕捉了优先级的一个维度：**技术可行性约束**（什么必须先存在，什么才能被构建）。它告诉你「不能」的边界，不告诉你「应该」的顺序。

优先级实际上有四个维度：

| 维度 | 问题 | 依赖树覆盖？ |
|------|------|-------------|
| 技术依赖 | X 构建前 Y 必须存在吗？ | ✅ |
| 业务价值 | X 对用户的核心工作流有多关键？ | ❌ |
| 验证风险 | X 是否是最不确定的部分，应该早验证？ | ❌ |
| 用户旅程 | X 是否在用户能走通第一个完整流程的关键路径上？ | ❌ |

在 FeedVault 的 C 组件里：`sources.json`（C-2）和 `categories.json`（C-3）同在 Level 0，依赖树说它们平级。但从用户旅程看，如果没有 `sources.json`，Skill 根本无法运行；没有 `categories.json`，只是少了一个分类标签，影响小得多。依赖树看不出这个差异。

所以：**依赖树是必要约束，不是完备模型**。它划定了「可行」的边界，边界内的顺序仍需要价值、风险、用户旅程三个维度共同决定。

---

**这是「思维模型显式化」/ 结构化思维吗？**

是，这是同一个底层模式的具体实例。

回看这条链：
- Turn15 说「框架驱动 vs 开放式指令」的差异是「思维模型的显式化」
- 依赖树的做法是：把「逻辑上应该先做什么」这个隐性判断，外化为有节点、有边、有层级的有向无环图（DAG）

结构化思维的本质特征：
1. **分解**：把复杂问题拆成离散的单元
2. **关系建模**：把单元之间的关系显式化（依赖、包含、顺序、因果）
3. **可推导**：结构本身可以产生结论（层级决定顺序，不需要额外判断）
4. **可审计**：别人可以质疑结构本身，而不仅仅是结论

依赖树完全符合这四点，所以是结构化思维的典型实例，也是把隐性判断显式化的同一个模式。

---

**这是一种领域建模吗？**

有交集，但需要区分层次。

- **广义领域建模**（对问题领域做结构化描述）：是，依赖树、评估框架、实体关系图都是这个意思下的建模行为
- **狭义领域建模**（DDD 语境：实体、聚合、限界上下文、领域事件）：不完全是，依赖树更接近「构建依赖图」，关注的是构造顺序，不是领域知识本身

更准确的说法是：依赖树、评估框架、实体模型，都是**图结构建模**的不同应用——用节点和边把隐性关系显式化，区别只在于「建模什么」：

| 工具 | 建模对象 | 边的含义 |
|------|----------|----------|
| 依赖树 | 构建顺序 | X 必须先于 Y |
| 实体关系图（ERD） | 数据结构 | X 拥有/引用 Y |
| 评估框架 | 质量标准 | 维度之间的层次关系 |
| 因果图 | 逻辑推理 | X 导致 Y |
| 流程图 | 执行流程 | X 之后是 Y |

它们都是结构化思维，都是显式化的不同面，也都在广义上属于领域建模。更精确的上位概念是：**用结构替代直觉**——把只存在于头脑中的判断，转化为可以被看见、被质疑、被改进的图形或框架。

这也是为什么工程师比较擅长解决复杂问题——不是因为比别人聪明，而是有一套把复杂性结构化的习惯工具。

---

现在执行 Task 1，把元问题分析补充到评估框架文档：

<!-- DDM:TURN_SEP:v1 -->

## AI

完成。

`00-product-doc-evaluation-framework.md` 末尾新增了「为什么用框架而不是直觉」章节，包含：
- 两种指令的本质差异分析
- 框架的三个优势 + 局限
- 「用结构替代直觉」的更广泛模式对比表（依赖树、ERD、流程图等统一到同一个元模式）

关于 Task 2 的结论归纳在表格里了——依赖树、评估框架、领域建模，都是同一个底层动作（隐性 → 显式结构）在不同建模对象上的投影，区别只在于「节点是什么、边的含义是什么」。

> 模型：Claude Sonnet 4.6

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 18）

<timestamp>Monday, May 4, 2026, 11:07 AM (UTC+8)</timestamp>
<user_query>
1 关于 功能依赖树是完备的优先级模型吗？ 你整理一个 草稿给我： 产品优先级评估模型。

</user_query>

<!-- DDM:TURN_SEP:v1 -->

## AI

草稿已写入 `.cache/design/draft-product-priority-model.md`。

核心结构是**三层模型**：

```
Layer 1  依赖树（可行性门控）
         → 过滤掉当前不可构建的功能

Layer 2  D1~D4 四维评分（在可行范围内排序）
         D1 用户旅程关键路径   — 缺它旅程走不通？
         D2 验证风险          — 越不确定越应该早做
         D3 价值密度          — 价值 ÷ 成本
         D4 可逆性            — 做错了改得动吗？

Layer 3  MVP 定义
         → 先写出第一个完整用户旅程
         → 找出旅程必经的功能集合
         → 这个集合就是 MVP
```

依赖树只负责 Layer 1（可行与否），Layer 2 和 Layer 3 补上它看不见的部分。草稿末尾列了已知局限，包括主观性、静态快照、资源约束等，可以继续迭代。

> 模型：Claude Sonnet 4.6

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 19）

<timestamp>Monday, May 4, 2026, 11:10 AM (UTC+8)</timestamp>
<user_query>
现在按照产品文档为标准，同步升级 02 技术架构文档，给我一个升级列表。
</user_query>

<!-- DDM:TURN_SEP:v1 -->

## AI

先全量读取两个文档，再对比差异：

<!-- DDM:TURN_SEP:v1 -->

## AI

对比完成，梳理出 **9 项需要升级**的内容，按位置列出：

---

## 02 架构文档升级列表

### 架构图（Mermaid）

**U1 — web 子图节点描述过时**
- 现状：`browse["浏览页 / 首页 / 分类页 / Topic 详情"]`，`manage["管理页 / 分类 / Topic / 来源 CRUD"]`
- 01 明确：前台是「兴趣分类 → Topic 详情」，后台是 `/admin`（人物/产品分类管理）
- 升级：将 web 子图拆为 `frontend["前台 UI<br>兴趣列表/兴趣详情/Topic详情"]` 和 `admin["后台管理(/admin)<br>分类/Topic/来源/PAT"]`

**U2 — 用户角色节点不完整**
- 现状：只有 `user2["用户（管理）"] → manage`，暗示管理=后台
- 01 明确：前台兴趣管理（新增/删除/添加Topic）也是写操作，也需要 PAT
- 升级：`user2` 重命名为「用户（写操作，需 PAT）」，连接到前台和后台两处

**U3 — Skill 更新流注释仍绑定 Cursor**
- 现状：`user3 →|"复制触发命令粘贴到 AI 工具"| B1`（文字已改，但 B1 注释里仍含 Cursor 隐含假设）
- 01 明确：可被任意 AI 工具触发
- 升级：在图中 B1 注释补充「通用，不绑定 Cursor」

---

### 组件职责表

**U4 — A（前台网站）职责描述过于笼统**
- 现状：`浏览 feed、管理配置、通过 GitHub API 提交变更`
- 01 明确区分：前台负责「兴趣分类浏览+兴趣管理写操作」，后台负责「人物/产品分类、Topic来源配置」
- 升级：将 A 拆为「前台 UI」和「后台管理」两行，各自说明职责

---

### 数据流

**U5 — 更新流描述仍说「在 Cursor 触发」**
- 现状：`用户在 Cursor 触发 Skill`
- 升级：改为「用户在任意 AI 工具中触发 Skill」

**U6 — 更新流缺少「平台来源 vs Topic 专属来源」的抓取分支**
- 现状：`WebFetch YouTube RSS / X / Nitter / 个人博客` 并列，没有区分
- 01 明确：平台来源用 RSS 模板，Topic 专属来源先发现 RSS、无则降级 AI 搜索
- 升级：更新流拆为两个分支描述

---

### 技术选型

**U7 — Skill 一行仍强调 Cursor**
- 现状：`Cursor Agent Skill（.md 指令文件）｜无需独立运行时，借助 AI 能力做 Web 抓取`
- 升级：改为「通用 AI Skill（Markdown 指令文件，可被 Cursor / Claude / GPT 等触发）」

---

### 部署配置

**U8 — 一期「本地启动为主」未体现**
- 现状：只写了 GitHub Pages 部署配置，没有本地启动说明
- 01 明确：一期以本地启动为主，GitHub Pages 是后续
- 升级：补充本地启动方式（`npm run dev`），GitHub Pages 配置标注「设计支持，一期暂不启用」

---

### 关键假设

**U9 — 假设清单比 01 少两条**
- 现状：只有 3 条（GitHub Pages 相关）
- 01 还有：
  - 「单人使用，不考虑并发写入」→ 影响整个写入架构
  - 「Nitter 实例可访问」→ 影响 X/Twitter 来源抓取路径
- 升级：补充这两条假设到关键假设章节

---

### 缺失章节

**U10 — 缺少「双入口架构说明」**
- 01 的核心架构概念（前台/后台双入口、兴趣维度/人物产品维度分离）在 02 完全没有体现
- 升级：在组件职责前新增「双入口架构」节，用一段文字和对照表说明两个入口的技术边界（静态路由 vs 客户端读写）

---

共 **10 项**（U1~U10），涉及：Mermaid 图 3 处、组件表 1 处、数据流 2 处、技术选型 1 处、部署配置 1 处、假设 1 处、新增章节 1 处。

是否现在执行？

> 模型：Claude Sonnet 4.6

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 20）

<timestamp>Monday, May 4, 2026, 11:17 AM (UTC+8)</timestamp>
<user_query>
请执行更新 
</user_query>

<!-- DDM:TURN_SEP:v1 -->

## AI

变更点较多且分散，整体重写最干净：

<!-- DDM:TURN_SEP:v1 -->

## AI

完成。10 项升级全部执行，逐项确认：

| 编号 | 内容 | 状态 |
|------|------|------|
| U1 | web 子图拆为 `frontend`（前台）和 `admin`（后台） | ✅ |
| U2 | `user2` 改为「写操作，需 PAT」，同时连前台和后台 | ✅ |
| U3 | B1 注释补充「通用，不绑定 Cursor」 | ✅ |
| U4 | 组件职责表 A 拆为 A-前台 / A-后台 两行 | ✅ |
| U5 | 更新流「Cursor 触发」→「任意 AI 工具触发」 | ✅ |
| U6 | 更新流补充平台来源（RSS优先）vs Topic专属来源（RSS发现→降级AI）两分支 | ✅ |
| U7 | 技术选型 Skill 一行去掉 Cursor 绑定 | ✅ |
| U8 | 部署配置拆为「一期本地启动（主要）」和「后续 GitHub Pages（设计支持）」 | ✅ |
| U9 | 假设表补充「单人使用」和「Nitter 可访问」，并补充第 6 条 Admin 路径假设 | ✅ |
| U10 | 新增「双入口架构说明」章节，含技术边界对照表 | ✅ |

> 模型：Claude Sonnet 4.6

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 21）

<timestamp>Monday, May 4, 2026, 11:29 AM (UTC+8)</timestamp>
<user_query>
FeedVault 产品介绍 定位是：FeedVault 产品规格说明书。请你根据定位，以及： @.cache/design/00-product-doc-evaluation-framework.md  在此审查，给出优化建议。 
</user_query>

<!-- DDM:TURN_SEP:v1 -->

## AI

逐维度完整审查，以「产品规格说明书」为标准评估：

---

## 审查结果

### 第一层：文本质量

**维度1 — 完备性**

| 编号 | 问题 | 位置 |
|------|------|------|
| P1-1 | Topic 详情页缺少字段规格：文本 Tab / 视频 Tab 的表格列定义（列名、排序规则、分页）完全没有 | 核心功能·前台 |
| P1-2 | 后台「兴趣统一视图·批量操作」内容未定义，只说"批量操作"，不知道批量操作了什么 | 后台管理功能 |
| P1-3 | PAT 配置页的具体 UI 行为未描述（填写框？如何验证有效性？保存后提示什么？） | 后台管理功能 |
| P1-4 | 操作后跳转状态全部缺失：新增兴趣后停在哪个页面？删除兴趣后返回哪里？ | 全文交互路径 |
| P1-5 | 首次使用完整旅程缺失：从空仓库到看到第一条 feed 的端到端步骤没有描述 | 全文 |

**维度2 — 逻辑一致性**

| 编号 | 问题 | 位置 |
|------|------|------|
| P2-1 | 交互路径图（Line 27）有「兴趣卡片 [编辑]」，但「核心功能·前台」只写「新增/删除」，漏了「编辑」 | 核心功能 vs 交互路径 |
| P2-2 | 同上，MVP 定义中「前台首页（兴趣列表+新增/删除）」也漏了「编辑」 | 功能优先级 |
| P2-3 | 「异常流程」中「Skill 写入失败」的行为描述是 Skill 内部行为，与「用户看到/系统行为」的列标题不符——Skill 是 AI 工具执行的，用户看不到这个输出 | 关键异常流程 |

**维度3 — 描述无歧义性**

| 编号 | 问题 | 位置 |
|------|------|------|
| P3-1 | 「系统抓取并归档各来源的**最新内容**」— 多新算新？RSS 通常只有最近 20 条，这是否是上限？未定义 | 产品定位 |
| P3-2 | 「平台来源（勾选并填参数）」— UI 如何引导用户知道填什么参数？「参数」二义：是固定提示填 channelId，还是自由文本框？ | Step 3 |
| P3-3 | 「写操作不可用」— 是按钮置灰，还是点击后弹错误，还是整个页面不渲染？行为不明确 | 前提说明 |

**维度4 — 术语一致性**

| 编号 | 问题 | 位置 |
|------|------|------|
| P4-1 | **「全局平台来源库」（Step 4 标题）vs「通用数据来源库」（后台管理功能·第3条）** — 同一概念，两个词 | Step 4 vs 核心功能 |
| P4-2 | 全文没有术语表（Glossary），「兴趣」「Watchlist」「兴趣分类」三个词在文档中混用（早期版本遗留） | 全文 |

---

### 第二层：结构一致性

**维度5 — 优先级与范围边界**

| 编号 | 问题 | 位置 |
|------|------|------|
| P5-1 | MVP「一期可选：后台管理页面（可先用直接编辑 JSON 文件代替）」— 这是重大决策，但没说清楚：跳过后台 UI 的前提是用户知道如何手动编辑 JSON 并 commit，这个前提需要显式说明 | 功能优先级 |
| P5-2 | MVP 三档（必须有/一期可选/后续迭代）缺少验收标准——什么状态算「完成」无法判断 | 功能优先级 |

**维度6 — 依赖关系显式化**

| 编号 | 问题 | 位置 |
|------|------|------|
| P6-1 | Step 1 → Step 4 是顺序依赖（必须先建分类才能建 Topic，先建 Topic 才能配来源），但没有明确标注为「必须按顺序执行」 | 概念二·后台交互路径 |
| P6-2 | 「Skill 更新」依赖「PAT 已配置」+ 「topics.json 已有数据」，这两个前置条件未在 Skill 章节说明 | 数据自动更新 |

---

### 第三层：可执行性（规格说明书最大缺口）

**维度7 — 可验证性** ← **规格说明书的核心要求，目前整体缺失**

| 编号 | 问题 | 影响 |
|------|------|------|
| P7-1 | 没有任何功能有成功标准。「新增兴趣」是什么状态算成功？「Topic 详情页展示信息流」的完整条件是？ | 无法验收 |
| P7-2 | 表格内容没有字段规格：文本 Tab 显示哪些列？视频 Tab 显示哪些列？列宽？排序默认是发布时间还是抓取时间？ | 实施时需要猜测 |
| P7-3 | 「一键复制更新命令」：复制成功后用户看到什么？Toast？文字变化？按钮状态变化？ | 实施时需要猜测 |
| P7-4 | 没有边界值定义：Topic 名称最长多少字符？兴趣分类最多几个？ID 允许哪些字符（影响 GitHub 文件路径）？ | 可能产生运行时错误 |

**维度8 — 假设显式化**

| 编号 | 问题 | 位置 |
|------|------|------|
| P8-1 | 缺少「Topic ID / Category ID 只含字母数字和连字符」的假设——含特殊字符的 ID 会破坏 GitHub 文件路径 | 显式假设 |
| P8-2 | 缺少「feed 数据量没有上限」的假设说明——单个 Topic 下积累数千条 MD 文件后，git 操作和页面加载是否仍可用 | 显式假设 |

**维度9 — 技术可行性**

| 编号 | 问题 | 影响 |
|------|------|------|
| P9-1 | 「一键复制」在 HTTP（非 HTTPS）本地环境下，`navigator.clipboard` API 不可用——本地 `npm run dev` 是 HTTP，需要用 fallback | 本地使用会失败 |

---

### 第四层：流程与决策完整性

**维度10 — 异常流程覆盖**

| 编号 | 问题 | 位置 |
|------|------|------|
| P10-1 | 缺少：Topic 被后台删除，但仍在某个兴趣分类的 `topicIds` 里，前台访问该兴趣时如何处理 | 异常流程 |
| P10-2 | 缺少：Topic 详情页没有任何 feed 数据（Skill 从未运行）时的空状态 | 异常流程 |
| P10-3 | 缺少：PAT 在会话中途过期时的处理（前台写操作到一半失败） | 异常流程 |

**维度11 — 用户旅程完整性**

| 编号 | 问题 | 位置 |
|------|------|------|
| P11-1 | 关键操作后的返回状态全部缺失（新增/删除/编辑完成后用户落在哪个页面/状态？） | 全文交互路径 |
| P11-2 | 跨系统旅程缺失：「后台配置 PAT → 返回前台执行写操作」的完整流程没有端到端描述 | 全文 |
| P11-3 | 首次使用旅程完全缺失（空仓库 → 配置 → 创建分类 → 创建 Topic → 配来源 → 触发 Skill → 看到 feed） | 全文 |

---

## 优化建议汇总

按「规格说明书」标准，分优先级：

**立即修复（阻塞实施）：**
- P2-1 / P2-2：三处统一加回「编辑」
- P4-1：统一术语，选一个：「全局平台来源库」
- P4-2：在文档开头新增术语表（Glossary）
- P8-1：补充 ID 格式约束假设
- P9-1：一键复制改用兼容 HTTP 的 fallback 方案（`document.execCommand` 或提示手动复制）

**升级文档定位必须补充（规格说明书必须有）：**
- P7-2：为 Topic 详情页文本 / 视频 Tab 各补充完整字段规格表（列名、数据来源字段、排序）
- P7-3：为「一键复制」定义成功状态（Toast 提示 or 按钮文字变化）
- P7-4：补充关键字段的边界值约束
- P1-4 / P11-1：为每个写操作补充「完成后跳转到哪里」
- P11-3：补充「首次使用完整旅程」

**实施中补充（不阻塞启动）：**
- P1-2：定义后台「批量操作」的具体内容
- P1-3：PAT 配置页详细 UI 行为
- P5-1：说明跳过后台 UI 时的替代操作前提
- P10-1 / P10-2 / P10-3：补充三个缺失异常场景

---

共发现 **26 个问题**，其中 5 个立即修复、9 个规格升级必要、12 个实施中补充。是否现在逐项执行？

> 模型：Claude Sonnet 4.6

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 22）

<timestamp>Monday, May 4, 2026, 11:36 AM (UTC+8)</timestamp>
<user_query>
执行吧 
</user_query>

<!-- DDM:TURN_SEP:v1 -->

## AI

26 项变更分散在全文，整体重写最干净，同时将文档标题升级为「产品规格说明书」：

<!-- DDM:TURN_SEP:v1 -->

## AI

26 项全部完成。主要结构变化：

**新增章节：**
- `术语表（Glossary）` — 9 个核心术语统一定义，P4-2
- `字段规格` — 文本 Tab / 视频 Tab 各自的列定义、数据来源字段、展示格式，P7-2
- `边界值约束` — ID 格式、名称/描述长度、条目数量建议，P7-4
- `首次使用完整旅程` — Step A~K 的端到端路径，P11-3

**修复要点：**
- 三处「新增/删除」全部补齐为「新增/编辑/删除」（P2-1/P2-2）
- 「通用数据来源库」统一为「全局平台来源库」（P4-1）
- 交互路径图补全操作后跳转状态（P1-4/P11-1）
- 写操作按钮状态改为「置灰 + hover 提示」而非含糊的「不可用」（P3-3）
- 后台管理 Step 顺序标注为「顺序依赖，必须按步骤执行」（P6-1）
- PAT 设置页补充完整 UI 规格（验证 / 保存 / 状态展示），P1-3
- 一键复制降级行为（HTTP 环境）显式说明，P9-1
- 异常流程新增 3 条（Topic 被删但仍在 Watchlist / Topic 无 feed 空状态 / PAT 中途失效），P10-1/2/3
- Skill 异常改为「Skill 输出摘要标注」而非「用户看到」，P2-3

> 模型：Claude Sonnet 4.6

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 23）

<timestamp>Monday, May 4, 2026, 11:41 AM (UTC+8)</timestamp>
<user_query>
一期可选（不阻塞启动）：的 A：后台管理页面，A：一键复制 Skill 触发命令按钮 都添加到 必须有（阻塞启动）。 必须有（阻塞启动） 改名字： 一期功能范围。 
</user_query>

<!-- DDM:TURN_SEP:v1 -->

## AI

完成。「一期功能范围」现在包含：C 全部 + B（RSS 抓取）+ A 前台完整 + A 后台管理完整 + A 一键复制。「一期可选」只剩 B 的 AI 搜索降级抓取一项。

> 模型：Claude Sonnet 4.6

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 24）

<user_query>
对当前对话，执行：dtd_normalize。 
</user_query>

<!-- DDM:TURN_SEP:v1 -->

## AI

正在从 transcript JSONL 提取对话并写入 archive，脚本按 DDM P0 规范拼接用户与助手可见文本：

<!-- DDM:TURN_SEP:v1 -->

## AI

正在修正：`index.json` 已写入可能含过度剥离的条目。按 P0「正文逐字」，重新从 transcript 全量导出（不对 assistant 正文做 heuristic 剔除），并使用新文件名避免混淆。

<!-- DDM:TURN_SEP:v1 -->

## AI

> ✅ config.json 读取完成 · archive_root: `/Users/lulu/Code/cognitive-trace-archive`

> ✅ **dtd_normalize 完成**

> 📄 **raw**：raw/common-tech/feedvault-product-architecture-session/202605041144-feedvault-product-architecture-session.md

> 🗂 **index.json 已更新**（新条目 layers 包含 `raw`）

---

**条目**
- **id**：`7766b5fdae7b4534c9c44af154e94b68`
- **来源**：`/Users/lulu/.cursor/projects/Users-lulu-Code-tech-language-java/agent-transcripts/c71dab4e-00d5-44d2-a9d4-f0aa1bc7f305/c71dab4e-00d5-44d2-a9d4-f0aa1bc7f305.jsonl`（全量逐段拼接，assistant 正文未做摘要/改写）

**说明**
- 已删除上一轮误加的索引项 `2d11a8da…` 及对应过时 raw 文件名（若有）。
- 若你希望归档走 `tech-language-java` 下其它 topic-path，可再说，我可以改路径并更新 index。

Turn:当前对话归一化为一次 dtd_normalize

> 模型：Claude Sonnet 4.x
