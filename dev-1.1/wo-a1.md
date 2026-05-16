# WO-A1 · topics.json 重构

**依赖**：无（可立即执行）  
**修改文件**：`topics.json`  
**预计改动量**：全量重写（约 120 行 → 约 50 行）

---

## 背景

当前 topics.json `version: 3`，每个 repo 条目包含 `m`（一二级目录映射）和顶级 `bl`（黑名单）。  
重构后只保留 repo 列表，移除所有目录层级约束，让 AI 在新建文档时自由推断 `doc-theme`。

---

## 新结构说明

```json
{
  "version": 4,
  "source": "CTA KNOWLEDGE_CORPUS (gh)",
  "defaultBranch": "main",
  "topics": [
    { "repo": "lulufoo/<name>" },
    { "repo": "lulufoo/<name>", "dir": "<local-dir>" }
  ]
}
```

- 移除 `bl` 顶级字段
- 移除每个条目的 `m` 字段
- 新增可选 `dir` 字段：仅当本地目录名与 repo 短名不一致时填写

---

## 步骤

### 1. 保留的 repo 列表（照抄，顺序不限）

```
lulufoo/ai-assisted-domain-learning
lulufoo/ai-authored-learning
lulufoo/ai-collaboration-framework
lulufoo/ai-software-dev
lulufoo/ai-thinking-framework
lulufoo/android-dev-docs
lulufoo/lulu-workbench
lulufoo/learning-ai-agent
lulufoo/learning-ai-lmm        ← dir: "ai"（本地目录名是 ai/）
lulufoo/learning-with-ai
lulufoo/project-experience
lulufoo/social-sciences
lulufoo/tech-language-java
lulufoo/tech-language-kotlin
```

### 2. 虚拟条目（无对应 GitHub repo，本地遗留目录）

```json
{ "dir": "common-tech" }
```

`common-tech/` 本地存在但无独立 repo（属 learning-with-ai 子主题），需保留为可选 project。

### 3. 删除字段

- 顶级 `bl` 字段（整行删除）
- 每个条目的 `m` 字段（整个 key-value 删除）

### 4. version 改为 4

---

## 目标文件内容

```json
{
  "version": 4,
  "source": "CTA KNOWLEDGE_CORPUS (gh)",
  "defaultBranch": "main",
  "topics": [
    { "repo": "lulufoo/ai-assisted-domain-learning" },
    { "repo": "lulufoo/ai-authored-learning" },
    { "repo": "lulufoo/ai-collaboration-framework" },
    { "repo": "lulufoo/ai-software-dev" },
    { "repo": "lulufoo/ai-thinking-framework" },
    { "repo": "lulufoo/android-dev-docs" },
    { "repo": "lulufoo/lulu-workbench" },
    { "repo": "lulufoo/learning-ai-agent" },
    { "repo": "lulufoo/learning-ai-lmm", "dir": "ai" },
    { "repo": "lulufoo/learning-with-ai" },
    { "repo": "lulufoo/project-experience" },
    { "repo": "lulufoo/social-sciences" },
    { "repo": "lulufoo/tech-language-java" },
    { "repo": "lulufoo/tech-language-kotlin" },
    { "dir": "common-tech" }
  ]
}
```

---

## 验收

```bash
# version 为 4
jq '.version' topics.json

# 无 m 字段残留
jq '[.topics[] | has("m")] | any' topics.json
# 期望输出: false

# 无 bl 字段
jq 'has("bl")' topics.json
# 期望输出: false

# 条目数
jq '.topics | length' topics.json
# 期望输出: 15
```
