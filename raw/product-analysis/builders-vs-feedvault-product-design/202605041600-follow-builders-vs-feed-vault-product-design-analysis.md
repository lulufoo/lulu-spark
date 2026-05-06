# follow-builders vs feed-vault：产品设计分析与方法论反思

> 创建时间：2026年5月4日 16:00

> 导航：[distilled](../../../distilled/product-analysis/builders-vs-feedvault-product-design/202605041600-follow-builders-vs-feed-vault-product-design-analysis.md) · [digest](../../../digest/product-analysis/builders-vs-feedvault-product-design/202605041600-follow-builders-vs-feed-vault-product-design-analysis.md) · [trace](../../../trace/product-analysis/builders-vs-feedvault-product-design/202605041600-follow-builders-vs-feed-vault-product-design-analysis.md)

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 1）

1 调研 https://github.com/zarazhangrui/follow-builders 这个项目做什么的
2 请审查当前的项目 和 https://github.com/zarazhangrui/follow-builders 的设计区别，比较优劣。

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 1）

先并行获取两侧资料。

### 1. follow-builders 是什么

follow-builders 是一个 **AI 建造者日报 Skill**，本质是"内容订阅推送工具"：

- **核心逻辑**：中心化采集 → 分发到个人
  - GitHub Actions 每日自动运行，用官方 X API（需 Bearer Token）抓取 25 位 AI 建造者的推文、用 RSS 抓取 6 个播客转录文本、用爬虫抓取 Anthropic/Claude 官方博客，写入仓库内的三个 JSON 文件（`feed-x.json` / `feed-podcasts.json` / `feed-blogs.json`）
  - 用户安装 SKILL.md 到本地 AI 工具（Claude Code / OpenClaw），Skill 拉取这个中央 feed，由本地 AI "remix"成摘要，推送到 Telegram / Discord / Email

- **关键特征**：
  - 内容来源固定（作者中心化维护，用户无法自定义）
  - **无 Web UI**，交付渠道是消息 App 或 AI 对话框
  - 零 API Key（用户侧），采集侧 API Key 存在仓库 Secrets 里
  - Stars: 3658，Forks: 433 — 已有较大用户群

### 2. 两个项目的设计对比

| 维度 | feed-vault（当前项目）| follow-builders |
|------|----------------------|-----------------|
| **产品定位** | 个人 Topic 信息流档案库，自定义任意主题 | AI 建造者日报，固定内容领域 |
| **内容采集主体** | 用户本地 AI（Skill 触发 WebFetch/RSS） | 中央 GitHub Actions（作者运维）|
| **来源管理** | 用户完全自定义：Topic + 平台来源 + 专属来源 | 固定（25 X 账号 + 6 播客 + 2 博客），用户不可修改 |
| **内容存储格式** | 每条 feed 一个 `.md` 文件，按 topic/type 归档 | 三个聚合 JSON 文件，按来源类型 |
| **浏览入口** | Next.js Web UI（兴趣列表、Topic 详情） | 消息 App（Telegram/Discord/Email）+ AI 对话 |
| **交付形式** | 浏览 / 检索为主，存档驱动 | 推送摘要为主，阅后即焚 |
| **个性化** | Watchlist 分组 + 按需浏览 | 频率/语言/推送时间/摘要风格 prompt 可调 |
| **数据归属** | 存在用户自己的 GitHub 仓库 | 存在作者的 GitHub 仓库（中央一份）|
| **采集 API Key** | 无（依赖公开 RSS / WebSearch） | X Bearer Token（作者承担） |
| **运维负担** | 用户自行触发 Skill；无后台服务 | 作者维护 GitHub Actions + X API 配额 |
| **可扩展性** | 任意 Topic、任意来源类型、任意用户 | 扩展 = 作者维护 PR |

### 3. 优劣比较

#### follow-builders 的优势
1. **零配置上手**：用户一条命令安装，当天就能收到 digest，无需理解 Topic / Watchlist 等概念
2. **内容质量稳定**：源列表经过人工筛选，平均信噪比高；YouTube 转录 + AI 摘要质量可控
3. **推送 = 主动触达**：消息 App 推送使日报真正进入用户注意力，而不是"有空了才去看"
4. **无运维压力（用户侧）**：X API 配额、爬虫维护、scheduled job 都由作者承担
5. **Skill 对话式配置**：通过自然语言调整设置，不需要学 UI

#### follow-builders 的劣势
1. **内容固定，无法扩展**：不能追踪作者列表之外的任何人/产品/领域
2. **数据不归用户**：feed 存在作者仓库，作者停止维护则服务中断
3. **无归档检索**：日报是流式消费，无法回溯"某人三个月前说了什么"
4. **单一场景**：只能做 AI 领域建造者内容，垂直程度高但通用性低
5. **中央化风险**：X API 涨价 / 封锁 → 整个采集链路断掉，所有用户同时失效

#### feed-vault 的优势
1. **完全自定义**：任意 Topic、任意来源，不限领域
2. **数据归档，长期可用**：每条 feed 独立 `.md` 文件存自己仓库，永久存档可检索
3. **Web UI 可浏览**：Watchlist → Topic → feed 条目，支持结构化查看历史
4. **去中心化**：无中央服务，无依赖第三方 API 的单点故障
5. **可组合**：Skill 本身可被 Cursor / Claude 等任意 AI 工具触发

#### feed-vault 的劣势（相较于 follow-builders）
1. **上手门槛高**：需要配置 Topic、来源、PAT、Watchlist，有认知成本
2. **无主动推送**：以"拉取浏览"为主，缺乏日报类的定期推送机制
3. **采集稳定性依赖公开 RSS**：X/Twitter 无官方 RSS，Nitter 不稳定；follow-builders 用官方 API 更可靠
4. **内容质量需用户把关**：没有预置的高质量源，用户自己选源质量参差不齐
5. **无摘要层**：feed-vault 存原始内容，follow-builders 有 AI remix 摘要层，消费效率更高

**一句话**：follow-builders 解决"每天自动读到好内容"，是消费驱动；feed-vault 解决"把自己关注的一切都归档到一处"，是存档驱动。两者互补而非竞争，follow-builders 的 feed 理论上可作为 feed-vault 的一个内容来源接入。

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 2）

follow-builders 的内容都是订阅了哪些内容？请你给我列一下。请你客观评估，消费型的设计和沉淀型的设计，哪个符合用户需求？

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 2）

### follow-builders 订阅内容清单

#### X/Twitter 账号（25 个）

| # | 姓名 | 身份 |
|---|------|------|
| 1 | Andrej Karpathy | AI 研究员，前 Tesla/OpenAI |
| 2 | Swyx | AI 工程师，Smol.ai 创始人 |
| 3 | Josh Woodward | Google Labs VP |
| 4 | Kevin Weil | OpenAI CPO |
| 5 | Peter Yang | Roblox PM |
| 6 | Nan Yu | AI 产品 |
| 7 | Madhu Guru | AI 产品 |
| 8 | Amanda Askell | Anthropic 研究员 |
| 9 | Cat Wu | AI 工程师 |
| 10 | Thariq | AI 建造者 |
| 11 | Google Labs | 官方账号 |
| 12 | Amjad Masad | Replit CEO |
| 13 | Guillermo Rauch | Vercel CEO |
| 14 | Alex Albert | Anthropic |
| 15 | Aaron Levie | Box CEO |
| 16 | Ryo Lu | AI 产品 |
| 17 | Garry Tan | YC 总裁 |
| 18 | Matt Turck | FirstMark 合伙人 |
| 19 | Zara Zhang（作者本人） | 投资人 |
| 20 | Nikunj Kothari | AI 投资 |
| 21 | Peter Steinberger | PSPDFKit 创始人 |
| 22 | Dan Shipper | Every.to CEO |
| 23 | Aditya Agarwal | Dropbox 前 CTO |
| 24 | Sam Altman | OpenAI CEO |
| 25 | Claude（@claudeai） | Anthropic 官方账号 |

#### 播客（6 个，含 RSS）

| 播客名 | 媒体 |
|--------|------|
| Latent Space | Substack RSS |
| Training Data | Megaphone RSS |
| No Priors | Megaphone RSS |
| Unsupervised Learning | Simplecast RSS |
| The MAD Podcast with Matt Turck | Anchor RSS |
| AI & I by Every | Anchor RSS |

#### 官方博客（2 个，爬虫采集）

- Anthropic Engineering（技术深度文章）
- Claude Blog（产品公告）

### 消费型 vs 沉淀型：哪个更符合用户需求？

**客观结论：消费型覆盖更广，沉淀型服务更深。大多数用户真正需要的是消费型。**

#### 市场已投票

follow-builders：3658 stars、433 forks，上线后迅速传播。沉淀型工具（如个人知识库、RSS 阅读器）在 AI 原生时代普遍面临"建了不用"的问题。这不是偶然。

#### 消费型符合人的实际行为

| 行为事实 | 消费型 | 沉淀型 |
|----------|--------|--------|
| 用户是否每天主动打开 | 不需要，推送到来 | 需要，取决于习惯 |
| 信息是否被实际消化 | 是（AI 已摘要） | 未必（存了不一定读）|
| 认知负担 | 低（接收即完成） | 高（分类/归档/检索） |
| 价值兑现时间 | 即时 | 延迟（"有用的时候再查"）|

> 大多数人的"知识管理"焦虑来自**积累了大量未消化的内容**。沉淀型工具可能加剧这种焦虑，而不是解决它。

#### 沉淀型的真实价值更窄

沉淀型有价值，但适用人群特定：
- **研究者/分析师**：需要回溯某人 3 个月前的观点，做横向比较
- **内容生产者**：需要以 feed 数据为素材写文章/报告
- **数据控**：获得感来自"归档完整"本身

#### 两种设计的根本矛盾

| | 消费型 | 沉淀型 |
|--|--------|--------|
| **解决的问题** | 信息过载，不知道该看什么 | 信息遗忘，找不到看过的东西 |
| **用户动机** | 主动，好奇心驱动 | 被动，防御性驱动（"万一以后有用"）|
| **实际使用频率** | 高（每日推送） | 低（偶尔检索）|
| **感知价值时机** | 即时 | 延迟，且不确定 |

**"万一以后有用"是沉淀型工具最大的伪需求陷阱。** 大多数被归档的内容，从未被检索过。

#### 对 feed-vault 的启示

如果 feed-vault 只做沉淀，用户会面临"搭好了不用"的命运。真正的机会在于：

> **沉淀是基础设施，消费是入口。**

feed-vault 的档案数据，加上"每周 digest"推送、AI 摘要层，才能把沉淀的价值兑现成消费体验。两者不是非此即彼，而是沉淀服务消费。

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 3）

你觉得可以把 feed-vault 改成消费型的吗？

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 3）

先把问题拆清楚，再给判断。

**"改成消费型"有两种解读，结论截然不同：**

### 解读 A：完全放弃存档，转型为日报工具

**不建议。**

这样做的结果是：重新造一个功能更弱的 follow-builders。
- follow-builders 已有 3658 stars，有先发优势
- 它有 X 官方 API 做数据源，feed-vault 只有公开 RSS
- 你的差异化（自定义 Topic、用户自己的数据）全部丢失

等于用劣势去打别人的优势。

### 解读 B：在存档基础上，叠加消费层

**这个值得做，而且 feed-vault 天然适合。**

follow-builders 的 digest 每次只能基于"今天的内容"。
feed-vault 的 digest 可以基于"过去三个月所有归档内容"——这是 follow-builders 做不到的。

需要加的东西不多：

| 需要新增 | 现有基础 | 改动量 |
|----------|----------|--------|
| Skill：生成 digest（读 `data/feeds/` 下的 .md 文件，AI 摘要） | feeds 数据已在本地 | 小，写一个新 Skill |
| GitHub Actions：定时触发 Skill 生成 digest | 仓库已存在 | 小，参考 follow-builders 的 workflow |
| 推送渠道（Telegram/Email） | 无 | 中等，但可复用 follow-builders 的 deliver.js 思路 |

### 核心判断

> **存档 → 消费** 是单向的，有存档才能消费。
> **消费 → 存档** 是反向的，没有存档就没有什么可消费的。

feed-vault 先建存档是对的。现在缺的不是"转型"，而是在存档上加一层"定期把价值吐出来"的机制。

**一个具体的最小路径：**

在 `skills/feedvault/` 下增加一个 `generate-digest.md` Skill，逻辑是：
1. 读取用户指定 Watchlist 下所有 Topic 的最近 N 条 `.md` feed
2. 按 Topic 分组，AI 摘要每个 Topic 的关键内容
3. 组合成 digest，输出到终端 or 推送 Telegram

这样 feed-vault 就同时具备两个能力：**看历史的仓库** + **推新内容的日报**，而 follow-builders 只有后者。

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 4）

我现在觉得 feed-vault 有一点鸡肋，信息本身就是被消费的。存档没有特别的价值。

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 4）

这个感受是诚实的，值得认真面对。

**存档有价值的前提是：你会回来查。**

现实是：大多数人不会。信息的价值在它新鲜的时候最高，存下来之后价值递减，再也不看的概率极大。这不是懒，是信息消费的自然规律。

**那 feed-vault 的存档对谁有价值？**

只对一类行为有意义：**你需要用历史内容做二次生产**。
- 写分析文章，需要回溯某人 3 个月内的观点变化
- 做研究，需要横向比较多个人对同一件事的反应
- 训练数据、检索增强等技术用途

这是研究员/分析师的工作方式，不是普通用户每天的信息消费行为。

**所以你的感受指向一个真实的问题：**

feed-vault 目前的核心设计（Skill 抓取 → GitHub 存 `.md` → Web 浏览）是一套**基础设施**，但缺少一个能直接感受到价值的**消费体验**。用户搭好了，打开网站看一堆 `.md` 列表，然后呢？

这不是存档本身鸡肋，是**存档和消费之间缺了一层**。

**两个方向，你需要选一个：**

| | 方向 A：拥抱消费，砍掉存档包袱 | 方向 B：存档是差异，补上消费层 |
|--|---|---|
| 做什么 | 学 follow-builders，做推送日报 Skill，不存文件 | 现有存档不动，加 digest Skill + 推送 |
| 放弃什么 | 数据归属、历史检索、自定义 Topic | 无，向上叠加 |
| 竞争态势 | 正面竞争 follow-builders | 差异化：自定义 + 有历史的 digest |
| 适合场景 | 你只想每天收到 AI 领域更新 | 你想追踪自己定义的任意主题 |

**"万一以后有用"是沉淀型工具最大的伪需求陷阱。** 大多数被归档的内容，从未被检索过。

**feed-vault 的真正价值不是存档，是"自定义"。** 存档只是自定义的副产品。

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 5）

你觉得feed-vault 的模式，适合什么样的产品？是否有合适的转型方向？

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 5）

**先看 feed-vault 的模型本质：**

> GitHub 作数据库 + AI Skill 作采集管道 + 用户定义追踪范围

这个组合的核心能力是：**把分散在公开网络上的信息，按用户定义的维度持续汇聚、结构化存储**。

### 这个模型适合什么样的产品

适合的前提条件：

1. **信息是分散的** — 需要从多个来源聚合，而不是一个平台能搞定
2. **范围是用户自定义的** — 每个人关注的对象不同，固定列表满足不了
3. **有时间维度价值** — 历史数据比单次快照更有意义
4. **数据是公开的** — 不需要登录/授权采集

符合这四个条件的产品类型：

| 产品类型 | 具体场景 |
|----------|----------|
| **竞品情报** | 追踪竞争对手的博客、定价页、招聘（招聘是公司战略信号）、产品更新 |
| **投资研究** | 追踪被投公司/目标公司的公开动态、创始人发言、行业信号 |
| **招聘雷达** | 追踪目标公司的岗位变化趋势（岗位增减 = 业务方向信号）|
| **学术追踪** | 追踪特定研究方向的 arXiv 论文、特定研究员的产出 |
| **个人 CRM** | 追踪你关心的人（合作伙伴、客户、潜在投资人）的公开内容 |

### 最自然的转型方向

不是换赛道，是**重新定位 feed-vault 解决的问题**：

```
现在：信息存档工具（"把东西存下来"）
转型：个人情报工具（"持续监测，把值得关注的事推给你"）
```

两者底层架构完全一样，差的只是**一层"情报产出"机制**：

- 存档工具：采集 → 存 → 等用户来看
- 情报工具：采集 → 存 → **AI 分析变化/新内容 → 主动推出结论**

### 最有商业势能的方向：竞品情报

原因：

1. **有明确付费意愿** — 公司为市场情报每年付几千到几万美元，且预算已存在
2. **现有工具都很贵** — Crayon、Klue 这类工具年费 $1w+，但功能是 feed-vault 能覆盖的
3. **自定义是核心价值** — 每家公司的竞争格局不同，固定列表没用，用户定义范围正好契合
4. **历史数据真的有用** — "竞品三个月前还在主推 A 功能，现在悄悄改口了"——这种洞察需要存档

这个方向下，feed-vault 的产品名和定位可以变成：

> **"用 AI 追踪你的竞品和市场，每周一封情报摘要"**

**一句话**：feed-vault 的模式，是一套很好的 B 端个人/团队情报基础设施。现在缺的不是转型，是找到一个让"存档"变成"情报产出"的应用场景，竞品情报是最清晰的那个。

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 6）

GitHub Actions 可以代替Skill做抓取吗？follow-builders 使用 GitHub Actions 是做抓取吗？就是定时抓取某一天的内容是吗？

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 6）

### follow-builders 的 GitHub Actions 做什么

**是的，做的就是定时抓取。**

```
每天 UTC 06:00 自动运行：
  1. 调 X 官方 API（需 Bearer Token）→ 抓 25 个账号当天推文
  2. 拉 6 个播客 RSS → 获取最新剧集
  3. 爬 2 个博客页面 → 提取新文章
  4. 结果写入 feed-x.json / feed-podcasts.json / feed-blogs.json
  5. git commit & push 回仓库
```

抓到的是"当天新增的内容"，不是历史全量。每次运行增量更新那三个 JSON 文件。

### GitHub Actions vs Skill，做抓取谁更合适

| 维度 | GitHub Actions | Skill（AI 工具触发）|
|------|---------------|---------------------|
| **定时自动** | ✅ 原生支持 cron | ❌ 需要用户手动触发，或配 crontab |
| **稳定性** | ✅ 无头服务，不依赖用户在线 | ❌ 用户关电脑就停了 |
| **需要 API Key** | ✅ 支持（存 Secrets） | ✅ 支持（存本地 .env）|
| **无 API Key 的公开内容** | ✅ 直接 `curl` / `node fetch` | ✅ WebFetch |
| **需要 AI 理解页面** | ❌ 只能跑代码，不能"理解"内容 | ✅ 这是 Skill 的优势 |
| **成本** | 免费（公开仓库无限分钟）| 消耗 AI token |
| **用户部署难度** | 低（fork 仓库即用）| 中（需装 AI 工具）|

### 核心区别

**GitHub Actions 只能跑代码**，适合结构化来源（有 RSS / 有 API / 有固定格式的页面）：直接 fetch → 解析 → 存文件，不需要 AI。

**Skill 的不可替代场景**：来源没有 RSS、页面结构不固定、需要 AI 判断"这条内容是否相关"——这时候必须用 AI 读。

### 对 feed-vault 的含义

feed-vault 目前用 Skill 做抓取，有一个隐含假设：来源可能是无结构的，需要 AI 处理。

**但如果来源都是 RSS / 结构化 API**，完全可以把抓取移到 GitHub Actions：

```
GitHub Actions（定时抓取 → 存 .md）
+
Skill（生成 digest / 处理无结构来源）
```

两者不互斥，分工更合理：**机械的事给 Actions，需要理解的事给 Skill**。

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 7）

follow-builders 这个产品，它的Bearer Token 是公开了吗？

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 7）

**没有公开，Token 存在 GitHub Repository Secrets 里。**

```yaml
env:
  X_BEARER_TOKEN: ${{ secrets.X_BEARER_TOKEN }}
```

`${{ secrets.X_BEARER_TOKEN }}` 是 GitHub 的加密密钥机制：
- 只有仓库 Owner/Admin 能设置，其他人不可见
- 在 Actions 日志里自动被 `***` 遮蔽
- fork 仓库不会继承这个 Secret

**这意味着什么：**

作者的仓库里有一个 X Bearer Token，每天自动用它抓数据，然后把结果（已处理好的 JSON）公开在仓库里。

用户 fork 之后，拿到的只是数据产物（`feed-x.json` 等），**不需要也拿不到** Token 本身。

这是 follow-builders 的核心设计决策：**把 API 成本集中在作者一侧，用户零配置**。代价是：作者一人承担 X API 配额和费用，且如果作者停止维护或 Token 失效，所有用户的数据同时断更。

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 8）

那其实我不需要做当前的项目了，我只需要订阅它的内容即可。

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 8）

这个结论是合理的，但先确认一个前提：

**follow-builders 的内容范围能覆盖你的需求吗？**

它固定追踪的是：
- 25 个作者选定的 AI 领域人物（X 账号）
- 6 个英文 AI 播客
- Anthropic 官方博客

如果你关注的恰好就是这些——**订阅它就够了，不需要造轮子**。

但如果你有以下任何一种需求，follow-builders 给不了你：

- 追踪它列表以外的人（比如中文 AI 领域、你关心的创业者、特定产品）
- 追踪非 X/播客的来源（某个产品的更新日志、某个论坛、某个 newsletter RSS）
- 数据存在自己的仓库，自己控制
- 按自己的分类维度组织（而不是一锅 AI 建造者）

**你真实的需求是什么？**

是"每天收到 AI 领域更新"，还是"追踪一批我自己定义的信息源"？

前者：关注 follow-builders 项目，安装它的 SKILL.md，结束。
后者：feed-vault 仍然是你需要的东西，只是现在的设计需要补一层消费机制。

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 9）

但是，x 账号订阅，怎么收费呢？

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 9）

X API 定价（⚠️ 推断，基于 2023-2024 年调整后公开报道）：

| 套餐 | 月费 | 读取能力 |
|------|------|----------|
| Free | $0 | 基本只能发推，读取极度受限 |
| Basic | $100 | 每月约 10,000 条推文读取 |
| Pro | $5,000 | 每月约 100 万条推文读取 |
| Enterprise | 定制 | 不限量 |

follow-builders 追踪 25 个账号，每日一次，**Basic $100/月 理论上够用**。

**这个费用对 follow-builders 的意义：**

作者把这个成本自己扛了，所有用户免费用。这是这个项目的核心商业模式风险——一旦作者不续费或 Token 被封，全部用户同时断更。

**对你的直接影响：**

你作为用户，**不需要付这笔钱**，只需 fork 仓库 + 安装 SKILL.md，内容由作者的 Actions 更新。

这也是 feed-vault 用 Nitter RSS 而不用 X API 的原因——规避这笔费用，代价是稳定性下降。

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 10）

你作为用户，不需要付这笔钱，只需 fork 仓库 + 安装 SKILL.md，内容由作者的 Actions 推送过来，你拿到的是已处理好的 JSON。我没有理解，如何收到 由作者的 Actions 推送过来 的内容？你调查一下。我看到的是git项目里面自动在更新内容。

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 10）

你的观察是准确的，我之前的描述不精确，纠正如下。

### 实际数据流

```
[作者仓库]                          [你]
GitHub Actions 每天运行
  ↓
抓取 X + 播客 + 博客
  ↓
写入 feed-x.json                   ← 你 fork 或直接访问作者公开仓库的这个文件
       feed-podcasts.json
       feed-blogs.json
  ↓
git commit & push（公开仓库）
```

**到这里，"推送"就结束了。** 内容已经在作者的公开 GitHub 仓库里更新了。

### 你怎么"收到"内容

你的理解完全正确：**内容变化体现在 git 仓库里，不是传统意义上的"推送到你"**。

消费这个内容有两种方式：

**方式一：手动触发（按需）**
- 你在 Claude Code 里输入 `/ai`
- Skill 执行：`curl` 作者仓库的 `feed-x.json` → AI 读取 → 生成摘要 → 输出到终端

**方式二：自动定时（需配置 crontab + Telegram/Email）**
- 你本地设一个 `crontab`，每天早上 8 点运行
- crontab 调用 AI 工具执行 Skill
- Skill 拉取最新 JSON → 生成摘要 → 通过 Telegram Bot 发给你

### 关键纠正

我之前说"由作者的 Actions 推送过来"表述不准确。正确的说法是：

> 作者的 Actions **更新了公开 JSON 文件**，你的 Skill **主动去拉取**这些文件，在本地 AI 工具里生成摘要，再由你自己配置的 Telegram Bot 发给你。

**推送的最后一公里，是你自己的 Telegram Bot，不是作者。**

没有 Telegram 配置时，就只能手动 `/ai` 拉取，不会自动推送任何东西。

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 11）

刚刚讨论的 follow-builders 有几个点非常有价值，这是 follow-builders 这种产品设计背后的思考。

1 内容应该划分为：沉淀 vs 消费。x / youtube 的内容都是被消费的，你可以消费之后，提炼出来自己的内容。follow-builders 是在做一款让你高效消费内容的产品，内容应该主动推送给你。

2 GitHub Action是一个很有意思的应用。我之前只想到Skill去触发，但是 GitHub Action 提供了另外一个快速执行一个定时任务的能力，这个能力可以在很多其他产品里面使用

3 follow-builders 把最难的那部分做了，然后它的价值是所有使用这个产品的人的利益。有一天她把这个产品做成收费，可能也有很多人愿意付费。这个产品把最难的一步解决了，而且解决了很多人的核心需求。

3 我觉得feed-vault现在可以暂停了，这给我的教训是，做一款产品需要更多的思考：客户是谁？产品的使用场景是什么？类似的产品都是怎么做的？

4 你提到的最后一公里，我觉得是现在我需要做的。我可以把follow-builders最为数据源，做成skill，或者把内容嵌入我的个人工具里面去。

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 11）

四点都清晰，逐一回应。

**1. 消费 vs 沉淀的认知**

准确。更精炼的表达是：**消费是主流需求，沉淀是专业需求**。大多数人需要的是"每天读到好东西"，少数人需要的是"找到三个月前看过的那篇"。产品要先服务主流需求。

**2. GitHub Actions 的价值**

你发现了一个关键点：**GitHub Actions = 免费的、人人可 fork 的云端定时任务**。这在 AI 工具时代特别有用——用户 fork 一个仓库，等于部署了一套自动化基础设施，零运维。这个范式可以套在很多场景：定时爬取、定时生成报告、定时更新静态数据。

**3. follow-builders 的产品逻辑**

它把"解决最难问题"作为护城河，而不是把"功能最多"作为护城河。这是正确的产品直觉。X API 的访问成本 + 稳定维护，对个人用户是不可逾越的障碍，作者用一个仓库解决了所有人的问题。**把最高的门槛变成自己的壁垒**，这是值得学习的设计思路。

**4. feed-vault 暂停 + 产品方法论**

你总结的三个问题——客户是谁、使用场景是什么、竞品怎么做——是产品设计的基本功，但往往在动手之后才真正理解。这不是教训，是正常的学习路径。代码没有白写，架构认知在下一个产品里还能用。

**关于你说的"最后一公里"**

具体是两件事，选一个方向：

**方向 A：写一个 Skill，把 follow-builders 的 feed 变成你的个人日报**
- 读 `feed-x.json` / `feed-podcasts.json` / `feed-blogs.json`（公开 URL，直接 curl）
- AI 按你的偏好 remix 成摘要
- 推到你的 Telegram

成本：半天，一个 Markdown Skill 文件。

**方向 B：把 follow-builders 的 feed 接入你现有的个人工具**
- 如果你有 Notion / Obsidian / 其他工具，做一个同步脚本
- GitHub Actions 定时拉取 → 写入目标工具

成本取决于目标工具的 API 复杂度。

<!-- DDM:TURN_SEP:v1 -->

## User（Turn 12）

我倾向后者，我需要一个web页面承载。因为，我有一个个人工具，每天都会使用。主要是，我觉得 Skill 不适合阅读，体验不太好。

除此之外，我其实做这个项目的另外一个目的是，完全一行代码不写的情况下，通过产品、架构、技术方案设计实现了这个项目。而且，基本上代码摄生成之后就可以运行了。

这是一个vibe coding的尝试，其实严格来说不算vibe coding，但是又不是特别严格的harness engineering。

<!-- DDM:TURN_SEP:v1 -->

## AI（Turn 12）

两件事分开说。

**关于 web 页面承载**

先确认一个前提：你说的"个人工具"是已有的独立工具，还是就是 feed-vault 这个 web 项目？

- 如果是 **feed-vault web**：最简单，加一个 `/digest` 页面，直接 `fetch` follow-builders 的三个公开 JSON，渲染出来
- 如果是**另一套独立工具**：需要了解它的技术栈，才能判断怎么接入

两种情况工作量差很多，先确认。

**关于 vibe coding 的反思**

你描述的方式有个更准确的名字，我觉得可以叫 **"设计驱动的 AI 编码"**：

```
你做的事：  产品定义 → 架构设计 → 技术方案 → 审查生成物
AI 做的事：  代码生成 → 运行验证
```

和 vibe coding 的区别是：vibe coding 是"感觉对了就行"，你做的是**在动手前把约束和边界想清楚**，AI 在一个收窄的解空间里生成，所以生成质量高、几乎能直接跑。

这其实是当前 AI 编码的最优姿势。不需要你写代码，但需要你有足够的系统性思维。**你的价值不在键盘上，在设计决策上。**

feed-vault 虽然暂停了，但这套方法论的价值不会随项目消失。

<!-- DDM:TURN_SEP:v1 -->
