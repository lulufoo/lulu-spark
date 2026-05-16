# Refactor 3.0 — 层3知识库搜索接入

> 背景：工作台（层2）处理 Entry 时，找不到层3已沉淀知识的连接点，导致加工效率低、知识孤岛问题无法收敛。

---

## 一、问题定义

### 1.1 当前断层

```
层2 工作台（lulu-workbench）
    ↓ 处理 Entry 时
    ? 层3 知识库对工作台不可见
层3 知识仓库（15 个 GitHub repos）
```

用户在层2 加工一条新 Entry 时，无法感知层3 中已有哪些相关沉淀知识，连接点靠记忆或手动 grep。

### 1.2 不在本次范围内

- 工作台自身内容（raw / distilled / digest）的搜索
- 语义搜索（embedding / vector）
- Obsidian 接入

---

## 二、需求

### 2.1 核心需求

**在工作台 viewer 中展示「相关知识」面板**，当用户打开一条 Entry 时，自动从层3知识库中检索相关内容，展示匹配的知识文档列表。

### 2.2 功能需求

| # | 需求 | 优先级 |
|---|------|--------|
| F1 | 打开 Entry 时，自动以 entry title + slug 触发搜索，展示相关知识列表 | P0 |
| F2 | 搜索结果展示：标题、所属 repo、内容 snippet、GitHub 跳转链接 | P0 |
| F3 | 用户可修改搜索词，手动再次搜索（300ms 防抖） | P0 |
| F4 | 索引过期提示：超过阈值（默认 24h）未更新时，面板顶部显示提示 + 「立即同步」按钮 | P1 |
| F5 | 「立即同步」异步执行（git pull 所有层3 repo + 重建索引），不阻塞前台其他操作 | P1 |
| F6 | 同步进度可感知：按钮状态变化，完成后结果自动刷新 | P1 |
| F7 | settle 一条文档后，新文档自动进入搜索索引（单条实时 upsert） | P1 |

### 2.3 非功能需求

| # | 需求 |
|---|------|
| N1 | 搜索引擎支持中文分词，无需手动预处理 |
| N2 | 架构本地运行与服务端部署一致，可通过配置切换 |
| N3 | server.py 不引入新的 Python 外部依赖 |
| N4 | 层3 repo 本地克隆已存在（`/Users/lulu/Code/`），不需要额外同步脚本 |

---

## 三、搜索范围

**索引目标**：`topics.json` 中有 `repo` 字段的 15 个层3知识仓库的所有 Markdown 文件。

| 仓库 | 描述 |
|------|------|
| lulufoo/ai-assisted-domain-learning | AI 辅助领域学习方法 |
| lulufoo/ai-authored-learning | AI 生成学习内容 |
| lulufoo/ai-collaboration-framework | AI 协作机制 |
| lulufoo/ai-software-dev | AI 工程实践 |
| lulufoo/ai-thinking-framework | AI 思维框架 |
| lulufoo/android-dev-docs | Android 技术知识 |
| lulufoo/learning-ai-agent | AI Agent 学习 |
| lulufoo/learning-ai-llm | LLM 理解 |
| lulufoo/learning-with-ai | 与 AI 协作学习 |
| lulufoo/product-analysis | 产品分析 |
| lulufoo/project-experience | 项目实战经验 |
| lulufoo/social-sciences | 社会科学与认知心理 |
| lulufoo/tech-language-java | Java 技术 |
| lulufoo/tech-language-kotlin | Kotlin 技术 |
| lulufoo/personal-growth | 个人成长 |

排除文件：`_index.md`、`README.md`

---

## 四、关键约束

- 本地克隆已在 `/Users/lulu/Code/{repo-name}/`，路径由 `KNOWLEDGE_BASE_DIR` 配置
- 服务端部署时通过环境变量覆盖 `KNOWLEDGE_BASE_DIR`，代码逻辑不变
- 索引过期不阻断搜索，旧索引仍返回结果，仅显示提示
- settle 的单条 upsert 不更新全库过期时间戳
