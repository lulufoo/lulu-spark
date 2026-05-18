# LuLu Workbench 流程指南

> 仓库：`https://github.com/lulufoo/lulu-workbench`（本地：`lulu-workbench`）
> 读者：AI（作为执行 instruction）+ LuLu（作为操作手册）
> 定位：框架层文档。描述系统本质、层次关系和核心流程。实现细节见 `refactor-*/`、`skills/` 及代码注释。

---

## 一、定位

LuLu Workbench 是**个人知识处理管道的中枢**，负责三件事：

1. 接收各类信息输入，加工为结构化条目（Entry）
2. 在工作台中进行二次思考、整理笔记
3. 将成熟的笔记沉淀为个人知识，推送到对应知识仓库

---

## 二、三层架构

```
原始信息（对话 / 文章 / 视频 / 闪念）
         |
         | Skill 加工
         v
  层1  信息输入加工层
         |
         | 生成 Entry（条目），落入 raw/
         v
  层2  工作台层（lulu-workbench）
         |
         | 笔记完整 → 一键同步
         v
  层3  知识沉淀层（topics.json 各仓库）
         |
         | _index.md 汇总 → 内嵌 system prompt
         v
  AI 协作知识索引
```

---

## 三、层1：信息输入加工层（Skill 层）

**职责**：将原始信息加工为可归档的结构化单元，写入工作台。

**执行者**：各 Skill（`skills/dialogue-summary/`、`skills/theme-line/` 等）

| 信息形态 | 典型 Skill |
|----------|-----------|
| 对话过程 | dialogue-summary |
| 视频 / 文章笔记 | theme-line |
| 闪念记录 | 直接归档 |

**输出**：一条 Entry，写入 `raw/{topic-path}/{ts}-{slug}.md`。

---

## 四、层2：工作台层

### 4.1 Entry（条目）

**Entry** 是进入工作台的最小记录单元，不限形态。

- 不是"一篇文章"，是**一条有来源、有时间戳的输入记录**
- Entry 只增不改，是原始归档

Entry 来源类型：`dialogue`（对话）/ `article`（文章）/ `video`（视频）/ `note`（闪念）/ `learning`（学习产物）

### 4.2 工作台的三项职能

**① 记录归档**：Entry 按 `topic-path` 有序写入 `raw/`，只增不改，是原始历史。

**② 二次加工**：在 `annotations/` 里写标注和笔记，记录处理进度（`done`）、关联的沉淀文档（`links`）和对 Entry 的注解（`comments`）。

**③ 沉淀判断**：笔记达到"可沉淀"标准后，触发同步写入知识仓库。

### 4.3 "可沉淀"标准

以下三条同时满足：

- **自洽**：不依赖读原始 Entry 即可理解
- **有结构**：按框架组织，不是对话的平铺复述
- **有观点**：包含自己的理解或抽象结论，而非摘抄

### 4.4 相关知识面板

打开任意 Entry 时，viewer 右侧自动检索层3知识库，展示相关沉淀文档（依赖本地 Meilisearch 进程，不可用时面板静默隐藏）。交互设计详见 `refactor-3.0/interaction-design.md`。

---

## 五、层3：知识沉淀层

### 5.1 定位

- 存放**足够抽象的个人知识**，覆盖 `topics.json` 中所有仓库
- 作为 AI 协作时的认知索引（AI 检索路径详见 §七）
- 同时是本地 Meilisearch 全文搜索的语料来源

### 5.2 每个知识仓库的目录结构

```
{repo}/
├── _index.md              # AI 入口：主题索引表（自动维护）
├── README.md              # 仓库简介
└── {doc-theme}/           # 按主题分目录，kebab-case
    └── {ts}-{slug}.md     # 沉淀文档
```

---

## 六、断裂层打通机制

> 断裂层：工作台笔记 → 知识仓库，需要明确的触发动作。

```
① annotations.comments 笔记达到"可沉淀"标准
         |
② 一键同步操作（workbench UI 按钮）
         |
③ 按 topic-path 规范写入目标知识仓库
         |
④ 建立双向链接（M:N 关系）
   · 沉淀文档头部 → 各源 Entry
   · annotation.links → 追加沉淀文档 URL
         |
⑤ 追加一条记录到目标仓库 _index.md
         |
⑥ 即时 upsert 到本地 Meilisearch 索引（server.py 自动执行）
```

---

## 七、知识检索

两条检索路径服务于不同使用者，相互独立：

| | LuLu 本地搜索 | AI 检索 |
|---|---|---|
| 触发 | 打开 Entry 时自动触发 | instruction 规则触发 |
| 数据源 | 本地 Meilisearch 索引 | GitHub `gh api` 实时读取 |
| 依赖 | 本机 Meilisearch 进程 | GitHub 网络 |
| 用途 | viewer 相关知识面板 | 对话上下文增强 |

### AI 检索：两级索引原则

目标：**让 AI 用最少 Token 找到最相关的知识文档。**

```
system prompt 内嵌一级索引（0次读取）
  → gh api 读取目标仓库 _index.md（第1次）
  → gh api 读取目标文档（第2次）
```

- **一级索引**：corpus 成员清单在 `.cache/knowledge-index.json`（人工维护）；文本块从该 JSON 复制到 system prompt / instruction。改 index 后须在浏览器 **⊙ 全量同步**，由 `scripts/update_topics_from_github.py` 读 index 写 `.cache/topics.json`（本地缓存，不提交 git；不自动触发）。
- **二级索引**：各仓库根目录的 `_index.md`，列出所有 doc-theme 和 GitHub URL。

`_index.md` 维护方式：断裂层同步步骤⑤自动追加（主路径）；定时任务全量重建（保底）。

---

## 八、快速参考

| 场景 | 操作 |
|------|------|
| 新增 Entry | Skill 执行 → 写入 `raw/` |
| 写笔记 | workbench UI → 存入 `annotations/` |
| 沉淀知识 | 笔记达标 → 一键同步 → 写入知识仓库 → 自动 upsert 索引 |
| AI 查知识库 | system prompt 含一级索引 → `gh api` 读 `_index.md` → `gh api` 读文档 |
| 维护 corpus 成员 | 编辑 `.cache/knowledge-index.json` 并提交 |
| 同步 topics | 浏览器 **⊙ 全量同步**，或 `python3 scripts/update_topics_from_github.py`（默认输出 `.cache/topics.json`）；前端经 `GET /api/topics` 读取 |
| 首次建搜索索引 | `python3 scripts/build_knowledge_index.py --wipe` |

---

## 九、核心原则

**1. 分层分离**：对话知识库（思考过程）与沉淀知识库（结论产物）必须分离。

**2. 沉淀产物三要素**：有结构、有溯源、自洽。

**3. 触发机制**：需要固定的触发机制确保每次对话都被处理，否则内容持续累积而未被沉淀。

**4. 双向关联**：沉淀文档头部链回原始对话；对话归档条目链回沉淀文档。两侧同步更新。

**5. 统一视图**：知识库的价值不在于存了多少，在于沉淀的内容是否被使用。

---

## 十、信息流转全景

```
        原始信息
         |
         | Skill 加工
         v
    层1  信息输入层
         |
         | 生成 Entry → raw/
         v
    层2  工作台层
         |
         | 人工巡检
         |
         +-- 暂不处理 ────────────────────────────┐
         |                                       |
         | 提炼 · 抽象 · 结构化                    |
         v                                       |
    层3  知识沉淀层  ·  分散 · 多个仓库              |
         ^                                       |
         |  双向关联：文档头部 ↔ 对话归档条目         |
         +<──────────────────────────────────────┘
         |
         v
   统一视图（HTML viewer）
         |
         +── 按时间浏览对话知识库 · 识别未沉淀
         +── 按 Entry 检索相关沉淀文档（Meilisearch）
         +── 按主题浏览沉淀覆盖（AI gh api 检索）
```
