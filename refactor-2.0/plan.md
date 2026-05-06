# LuLu Workbench 流程重设计计划

> 本次重设计背景：重新定位 cognitive-trace-archive（即 lulu-workbench）的角色，从"对话知识归档库"升级为"个人知识处理管道的完整工作台"。

---

## 一、目标

重写流程指南文档，覆盖三层架构设计、Entry 概念定义、断裂层打通机制、沉淀仓库统一结构标准、知识索引方案。

文档读者：**AI（作为执行 instruction）+ LuLu（作为操作手册）**，两者兼顾。

---

## 二、三层架构总览

```
原始信息（对话 / 文章 / 视频 / 闪念）
         |
         | Skill 加工
         v
  层1  信息输入加工层
         |
         | 落入工作台，生成 Entry（条目）
         v
  层2  工作台层（cognitive-trace-archive）
         |
         | 笔记完整 → 触发同步
         v
  层3  知识沉淀层（topics.json 各仓库）
         |
         | _index.md 汇总 → 内嵌 system prompt
         v
  AI 协作知识索引
```

---

## 三、三层详解

### 层1 · 信息输入加工层（Skill 层）

- **职责**：将原始信息加工为可归档的结构化单元，写入工作台
- **执行者**：各 Skill（ddm、theme-line 等）
- **输出**：一条 Entry，落入 `raw/` 或直接产出 `distilled/`

Skill 负责处理的信息形态：

| 信息形态 | 示例 | 典型 Skill |
|----------|------|-----------|
| 对话过程 | Copilot 学习对话 | ddm |
| 视频 / 文章笔记 | YouTube 技术讲解 | theme-line |
| 闪念记录 | 一闪而过的想法 | 直接归档 |
| 学习产物 | 对话本身就是学习过程 | ddm |

---

### 层2 · 工作台层

#### 2.1 Entry（条目）概念定义

**Entry（条目）** 是进入工作台的最小记录单元，不限形态。

- 不是"一篇文章"，也不是"一个文档"，是**一条有来源、有时间戳的输入记录**
- 对应 UI 里的"2026年5月6日（周三）· 3 条"——每一条就是一个 Entry
- 每个 Entry 携带：来源类型、时间戳、主题归属、加工状态

Entry 来源类型枚举：

| 类型标识 | 含义 |
|----------|------|
| `dialogue` | 与 AI 的对话过程 |
| `article` | 文章/博客阅读记录 |
| `video` | 视频笔记 |
| `note` | 主动记录的思考/闪念 |
| `learning` | 系统学习过程的产物 |

#### 2.2 工作台的职能

1. **记录归档**：Entry 按 topic-path 有序存入，只增不改
2. **二次加工**：在 Entry 上写标注、整理笔记、关联其他 Entry（存储于 `annotations/`）
3. **沉淀判断**：笔记达到"可沉淀"标准后，触发写入知识仓库

#### 2.3 "可沉淀"标准

笔记满足以下条件，视为可沉淀：

- [ ] 内容自洽：不依赖读原始 Entry 即可理解
- [ ] 有结构：按框架组织，不是对话的平铺复述
- [ ] 有观点：包含 LuLu 自己的理解或抽象结论，而非摘抄

---

### 层3 · 知识沉淀层

#### 3.1 定位

- 存放**足够抽象的个人知识**，不是原始对话，不是加工笔记
- 后续作为 AI 协作时的认知索引：AI 读这里，快速建立对 LuLu 知识体系的理解
- 覆盖 topics.json 中所有仓库

#### 3.2 每个仓库的统一目录结构

```
{repo}/
├── _index.md              # AI 入口：主题索引表（见 §四）
├── README.md              # 仓库简介（供人读，1-3 句话）
└── {doc-theme}/           # 按主题分目录（kebab-case）
    └── {ts}-{slug}.md     # 沉淀文档，命名规范同现有约定
```

约束：
- 每个 `doc-theme/` 目录聚焦**一个具体主题**，不混用
- `_index.md` 自动维护（详见 §五）
- `distilled/` 是工作台内的中间理解层，与 topics 仓库内容无关（遗留，暂不处理）

---

## 四、断裂层打通机制

> 断裂层：工作台笔记 → 知识仓库，目前缺乏明确的触发动作和格式约定。

### 4.1 笔记系统现状（已验证）

笔记存储于 `annotations/{topic-path}/{ts}.json`，结构如下：

```json
{
  "links": [
    { "url": "https://github.com/lulufoo/{repo}/blob/main/..." }
  ],
  "raw": {
    "comments": [
      { "id": "...", "text": "笔记内容", "ts": "202605031759" }
    ]
  },
  "done": true,
  "importance": "high"
}
```

- `raw.comments`：用户在 workbench UI 写的笔记条目
- `links`：已有的单向链接（annotation → topic repo 文档）
- `done`：处理状态标记（积压可见性的雏形，后续扩展）

### 4.2 理想流程（不绑定当前实现）

```
① annotations.comments 笔记达到"可沉淀"标准
         |
② 触发同步操作（一键按钮）
         |
③ 按 topic-path 规范写入目标仓库
         |
④ 自动建立双向链接（M:N 关系）
   · 源 Entry（可多条）→ 沉淀文档：annotation.links 数组追加 URL
   · 沉淀文档头部 → 源 Entry（> 来源：...）
         |
⑤ 自动追加一条记录到目标仓库 _index.md
```

### 4.3 双向链接规范

沉淀文档头部增加（支持多来源）：

```markdown
> 来源：[Entry-A](https://github.com/lulufoo/lulu-workbench/blob/main/raw/{topic-path}/{ts-a}-{slug-a}.md) · [Entry-B](...)
> 沉淀时间：YYYY年M月D日
```

workbench `annotations/{path}.json` 对应条目更新：

```json
{
  "links": [
    { "url": "https://github.com/lulufoo/{repo}/blob/main/{topic-path}/{ts}-{slug}.md" }
  ]
}
```

> Entry → 沉淀文档是 **M:N 关系**：一个 Entry 可关联多个沉淀文档；一个沉淀文档可来源于多个 Entry。

---

## 五、知识索引设计（AI 协作视角）

### 5.1 设计原则

目标：**让 AI 用最少的 Token 消耗，找到最相关的知识文档。**

方案：**两级索引（Two-stage Retrieval）**

- 不全量嵌入：各仓库文档直接贴入上下文，Token 消耗不可控
- 不搭 RAG：向量化检索目前太重；后续知识规模扩大后可升级
- 折中：**一级索引内嵌 system prompt（非文件）；二级索引通过 `gh api` 按需读取**

### 5.2 一级索引（内嵌 system prompt，非独立文件）

**一级索引不是一个文件，而是直接写在 system prompt / instruction 里的文本块。**

格式（紧凑，控制 Token）：

```
## LuLu 知识库

ai-thinking-framework: AI 思维框架，解题模型/画像模型/目标拆解; https://github.com/lulufoo/ai-thinking-framework/blob/main/_index.md
ai-assisted-domain-learning: AI 辅助学习方法，DDM/LCCM/领域深化; https://github.com/lulufoo/ai-assisted-domain-learning/blob/main/_index.md
android-dev-docs: Android 技术知识，Binder/AMS/协程/View; https://github.com/lulufoo/android-dev-docs/blob/main/_index.md
ai-software-dev: AI 工程实践，Agent/Harness/工具链; https://github.com/lulufoo/ai-software-dev/blob/main/_index.md
learning-with-ai: AI 学习方法论与经验; https://github.com/lulufoo/learning-with-ai/blob/main/_index.md
```

每行格式：`{repo}: {描述，含3-5关键词}; {_index.md GitHub URL}`

执行链（**2次读取**）：

```
prompt 已含一级索引（0次读取）
  → gh 读取 _index.md（第1次）
  → gh 读取目标文档（第2次）
```

**数据来源**：由 `topics.json` 生成（各条目新增 `description` / `keywords` 字段，见 §5.5）

### 5.3 二级索引（各仓库 `_index.md`，通过 `gh api` 读取）

```markdown
# {仓库名} 知识索引

| 主题 | 一句话描述 | GitHub URL |
|------|-----------|------------|
| dialogue-distillation-model | DDM 对话蒸馏工作流的设计与迭代 | https://github.com/lulufoo/{repo}/blob/main/dialogue-distillation-model/{ts}-{slug}.md |
| layered-cognitive | LCCM 分层认知模型的结构与应用 | https://github.com/lulufoo/{repo}/blob/main/layered-cognitive/{ts}-{slug}.md |
```

设计要点：
- 路径用 **GitHub 绝对 URL**，通过 `gh api` 读取，跨环境可用
- 每行"主题 + 一句话描述 + URL"，AI 读一次完成定位
- 描述概括核心结论，不是标题的重复

### 5.4 `_index.md` 自动维护

| 方式 | 触发时机 | 角色 |
|------|----------|------|
| **同步时追加** | 断裂层同步步骤⑤，自动追加一行 | 主路径 |
| **定时重建** | 定时任务扫描仓库，重建完整文件 | 保底，修复漂移 |

### 5.5 `topics.json` 扩展方案

在各条目增加字段（集中式，后续迁移到各仓库 `.repository-type.json`）：

```json
{
  "repo": "lulufoo/ai-thinking-framework",
  "description": "AI 思维框架，解题模型/画像模型/目标拆解",
  "keywords": ["思维框架", "解题", "画像模型", "TPM"]
}
```

`update_topics_from_github.py` 扩展：读取上述字段，输出一级索引文本块到 `knowledge-index.md`（供复制粘贴到 prompt）。

### 5.6 检索触发规则（instruction 约定示例）

```
当用户话题涉及以下领域时，主动查阅对应知识库：
- Android 技术实现 → android-dev-docs
- AI 学习方法 / DDM / LCCM → ai-assisted-domain-learning
- AI 协作思维框架 → ai-thinking-framework
- 用户明确要求查知识库 → 根据话题选择仓库
```

后续升级：规模扩大后叠加向量化检索，不需要重构索引结构。

---

## 六、交付物清单

| # | 交付物 | 状态 |
|---|--------|------|
| 1 | 删除 `dialogue-knowledge-management-guide.md` | 待执行 |
| 2 | 新建 `lulu-workbench-workflow.md`（流程指南正文） | 待执行 |
| 3 | 新建 `refactor-2.0/_index-template.md`（各仓库 `_index.md` 结构模板） | 待执行 |
| 4 | 新建 `refactor-2.0/system-prompt-knowledge-index-template.md`（一级索引文本块模板） | 待执行 |

---

## 七、已确认决策

| 决策点 | 结论 |
|--------|------|
| 工作台最小单元命名 | **Entry（条目）** |
| 一级索引形态 | 内嵌 system prompt 的文本块，非独立文件 |
| 跨仓库链接方案 | GitHub 绝对 URL，通过 `gh api` 读取 |
| `_index.md` 维护 | 同步时追加（主）+ 定时重建（保底） |
| Entry→沉淀关系 | M:N；`annotations.links` 为数组 |
| `topics.json` 扩展 | 增加 `description`/`keywords` 字段（集中式，后续迁移分布式） |
| `distilled/` 定位 | 工作台中间理解层，与 topics 仓库无关（遗留，暂不处理） |
| 积压可见性（问题三） | 延后处理 |
| 向量化 RAG | 后续升级路径 |
