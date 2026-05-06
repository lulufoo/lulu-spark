# LuLu Workbench 流程指南

> 仓库：`https://github.com/lulufoo/lulu-workbench`（本地：`lulu-workbench`）
> 读者：AI（作为执行 instruction）+ LuLu（作为操作手册）

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

**执行者**：各 Skill（`skills/ddm/`、`skills/theme-line/` 等）

| 信息形态 | 示例 | 典型 Skill |
|----------|------|-----------|
| 对话过程 | Copilot 学习对话、讨论对话 | ddm |
| 视频 / 文章笔记 | YouTube 技术讲解、博客文章 | theme-line |
| 闪念记录 | 一闪而过的想法 | 直接归档 |
| 学习产物 | 对话本身就是学习过程 | ddm |

**输出**：一条 Entry，写入 `raw/{topic-path}/{ts}-{slug}.md`。

---

## 四、层2：工作台层

### 4.1 Entry（条目）

**Entry（条目）** 是进入工作台的最小记录单元，不限形态。

- 不是"一篇文章"，是**一条有来源、有时间戳的输入记录**
- 对应 UI 时间线里"2026年5月6日（周三）· 3 条"中的每一条
- Entry 只增不改，是原始归档

Entry 来源类型：

| `type` | 含义 |
|--------|------|
| `dialogue` | 与 AI 的对话过程 |
| `article` | 文章 / 博客阅读记录 |
| `video` | 视频笔记 |
| `note` | 主动记录的思考 / 闪念 |
| `learning` | 系统学习过程的产物 |

### 4.2 工作台的三项职能

**① 记录归档**
Entry 按 `topic-path` 有序写入 `raw/`，只增不改，是原始历史。

**② 二次加工**
在 `annotations/{topic-path}/{ts}.json` 里写标注和笔记：

```json
{
  "links": [],
  "raw": {
    "comments": [
      { "id": "b290fe...", "text": "笔记内容", "ts": "202605031759" }
    ]
  },
  "done": false,
  "importance": "normal"
}
```

- `raw.comments`：在 workbench UI 写的笔记条目
- `links`：关联的知识仓库文档 URL（同步后自动填入）
- `done`：是否已完成处理

**③ 沉淀判断**
笔记达到"可沉淀"标准后，触发同步写入知识仓库。

### 4.3 "可沉淀"标准

以下三条同时满足：

- **自洽**：不依赖读原始 Entry 即可理解
- **有结构**：按框架组织，不是对话的平铺复述
- **有观点**：包含自己的理解或抽象结论，而非摘抄

---

## 五、层3：知识沉淀层

### 5.1 定位

- 存放**足够抽象的个人知识**
- 覆盖 `topics.json` 中所有仓库
- 后续作为 AI 协作时的认知索引

### 5.2 每个知识仓库的统一目录结构

```
{repo}/
├── _index.md              # AI 入口：主题索引表（自动维护）
├── README.md              # 仓库简介（1-3 句话，供人读）
└── {doc-theme}/           # 按主题分目录，kebab-case
    └── {ts}-{slug}.md     # 沉淀文档
```

**`{ts}-{slug}.md` 命名规范**（与 workbench 现有约定一致）：
- `ts`：`YYYYMMDDHHMM`，东八区
- `slug`：全小写连字符，不含 `ts`

**约束**：
- 每个 `doc-theme/` 目录聚焦**一个具体主题**，不混用
- `_index.md` 由同步操作和定时任务自动维护（详见 §七）

### 5.3 `distilled/`（遗留，暂不处理）

workbench 内的 `distilled/` 是工作台中间理解层，用于辅助理解 `raw/` 的长文档，与各知识仓库的内容相互独立，不做同步。

---

## 六、断裂层打通机制

> 断裂层：工作台笔记 → 知识仓库，需要明确的触发动作和格式约定。

### 6.1 理想流程

```
① annotations.comments 笔记达到"可沉淀"标准
         |
② 一键同步操作（workbench UI 按钮）
         |
③ 按 topic-path 规范写入目标知识仓库
         |
④ 建立双向链接（M:N 关系）
   · 沉淀文档头部 → 各源 Entry（> 来源：...）
   · annotation.links 数组 → 追加沉淀文档 URL
         |
⑤ 追加一条记录到目标仓库 _index.md
```

### 6.2 双向链接规范

**沉淀文档头部**（支持多来源 Entry）：

```markdown
> 来源：[Entry-A](https://github.com/lulufoo/lulu-workbench/blob/main/raw/{topic-path}/{ts-a}-{slug-a}.md) · [Entry-B](...)
> 沉淀时间：YYYY年M月D日
```

**annotation.links**（同步后自动填入）：

```json
{
  "links": [
    { "url": "https://github.com/lulufoo/{repo}/blob/main/{doc-theme}/{ts}-{slug}.md" }
  ]
}
```

> Entry → 沉淀文档是 **M:N 关系**：一条 Entry 可关联多个沉淀文档；一个沉淀文档可来源于多条 Entry。

---

## 七、知识索引设计（AI 协作视角）

### 7.1 设计原则

目标：**让 AI 用最少 Token 找到最相关的知识文档。**

方案：**两级索引（Two-stage Retrieval）**

```
system prompt 内嵌一级索引（0次读取）
  → gh api 读取目标仓库 _index.md（第1次）
  → gh api 读取目标文档（第2次）
```

- 不全量嵌入（Token 不可控）
- 不搭 RAG（目前太重，后续可升级）
- 一级索引是文本块，直接写在 prompt 里，不是一个需要读取的文件

### 7.2 一级索引（system prompt 文本块）

**不是文件，直接内嵌在 system prompt 或 instruction 里：**

```
## LuLu 知识库

ai-thinking-framework: AI 思维框架，解题模型/画像模型/目标拆解; https://github.com/lulufoo/ai-thinking-framework/blob/main/_index.md
ai-assisted-domain-learning: AI 辅助学习方法，DDM/LCCM/领域深化; https://github.com/lulufoo/ai-assisted-domain-learning/blob/main/_index.md
android-dev-docs: Android 技术知识，Binder/AMS/协程/View; https://github.com/lulufoo/android-dev-docs/blob/main/_index.md
ai-software-dev: AI 工程实践，Agent/Harness/工具链; https://github.com/lulufoo/ai-software-dev/blob/main/_index.md
learning-with-ai: AI 学习方法论与经验; https://github.com/lulufoo/learning-with-ai/blob/main/_index.md
ai-collaboration-framework: AI 协作机制与反模式; https://github.com/lulufoo/ai-collaboration-framework/blob/main/_index.md
social-sciences: 社会科学与认知心理; https://github.com/lulufoo/social-sciences/blob/main/_index.md
```

每行格式：`{repo}: {描述，含3-5关键词}; {_index.md 的 GitHub URL}`

**生成方式**：`update_topics_from_github.py` 读取 `topics.json` 中的 `description` / `keywords` 字段，输出文本块到 `knowledge-index.md`（供复制粘贴到 prompt）。

### 7.3 二级索引（各仓库 `_index.md`）

每个知识仓库根目录维护一个 `_index.md`，通过 `gh api` 读取：

```markdown
# {仓库名} 知识索引

| 主题 | 一句话描述 | GitHub URL |
|------|-----------|------------|
| dialogue-distillation-model | DDM 对话蒸馏工作流的设计与迭代 | https://github.com/lulufoo/{repo}/blob/main/dialogue-distillation-model/{ts}-{slug}.md |
| layered-cognitive | LCCM 分层认知模型的结构与应用 | https://github.com/lulufoo/{repo}/blob/main/layered-cognitive/{ts}-{slug}.md |
```

**填写规范**：
- `主题`：即 `doc-theme` 目录名
- `一句话描述`：概括文档核心结论，不是标题的重复
- `GitHub URL`：绝对路径，通过 `gh api` 读取，跨环境可用

### 7.4 `_index.md` 自动维护

| 方式 | 触发时机 | 角色 |
|------|----------|------|
| **同步时追加** | 断裂层同步步骤⑤执行时，自动追加一行 | 主路径 |
| **定时重建** | 定时任务扫描仓库所有文档，重建完整文件 | 保底，修复漂移 |

### 7.5 `topics.json` 扩展方案

在各条目增加 `description` 和 `keywords` 字段（当前阶段：集中式；后续迁移到各仓库 `.repository-type.json` 分布式自描述）：

```json
{
  "repo": "lulufoo/ai-thinking-framework",
  "description": "AI 思维框架，解题模型/画像模型/目标拆解",
  "keywords": ["思维框架", "解题", "画像模型", "TPM"]
}
```

### 7.6 AI 检索触发规则（instruction 约定）

在 system prompt 或 skill instruction 里显式声明：

```
当用户话题涉及以下领域时，主动查阅对应知识库：
- Android 技术实现（Binder / AQS / 协程 / View）→ android-dev-docs
- AI 学习方法 / DDM / LCCM / 领域学习 → ai-assisted-domain-learning
- AI 协作思维框架 / 解题模型 / 画像 → ai-thinking-framework
- AI 协作机制 / 补丁倾向 / 意图约束 → ai-collaboration-framework
- 用户明确要求"查我的知识库" → 根据话题选择仓库

查阅步骤：
1. 从一级索引确认目标仓库
2. gh api 读取该仓库 _index.md，定位具体主题
3. gh api 读取目标文档
```

---

## 八、快速参考

| 场景 | 操作 |
|------|------|
| 新增 Entry | Skill 执行 → 写入 `raw/{topic-path}/{ts}-{slug}.md` |
| 写笔记 | 在 workbench UI 对 Entry 写 comment → 存入 `annotations/` |
| 沉淀知识 | 笔记达到"可沉淀"标准 → 一键同步 → 写入目标知识仓库 |
| 维护索引 | 同步时自动追加 `_index.md`；定时任务重建 |
| AI 查知识库 | system prompt 含一级索引 → gh 读 `_index.md` → gh 读文档 |
| 更新一级索引 | `topics.json` 加 `description`/`keywords` → 运行脚本生成 `knowledge-index.md` |

---

## 对话知识管理原则

**1. 分层分离**

对话知识库（思考过程）与沉淀知识库（结论产物）必须分离。混合存储会导致两者都难以使用。

**2. 沉淀产物三要素**

- 有结构：内容按框架组织，不是对话的平铺复述
- 有溯源：沉淀文档头部与原始对话归档条目互指（对话 → 沉淀文档；沉淀文档 → 对话）
- 自洽：不依赖读原始对话才能理解

**3. 触发机制**

需要固定的触发机制确保每次对话都被处理。无机制，内容会持续累积而未被沉淀。

**4. 关联维护**

双向关联：沉淀文档头部链回原始对话；对话归档条目链回对应沉淀文档（便于回看对话时核对是否沉淀到位）。两侧应同步更新。

**5. 统一视图**

沉淀库分散于多个仓库，需要一个聚合对话与沉淀的统一入口。定位是低摩擦读取，而非仅导航。知识库的价值不在于存了多少，在于沉淀的内容是否被使用。

---

## 三层知识结构与信息流转关联

```
        对话
         |
         | 归档（原始 · 只增不改）
         v
    对话知识库  ·  集中 · 单一仓库
         |
         | 人工巡检
         |
         +-- 暂不处理 ────────────────────────────┐
         |                                       |
         | 提炼 · 抽象 · 结构化                    |
         v                                       |
    沉淀知识库  ·  分散 · 多个仓库 · 按主题           |
         ^                                       |
         |  双向关联：文档头部 ↔ 对话归档条目         |
         +<──────────────────────────────────────┘
         |
         v
   统一视图(HTML)
         |
         +── 读取对话知识库  →  按时间浏览 · 识别未沉淀
         |
         +── 读取沉淀知识库  →  按主题浏览 · 跟踪沉淀覆盖
```
