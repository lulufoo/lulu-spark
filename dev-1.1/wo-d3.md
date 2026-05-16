# WO-D3 · workbench-structure.md — 路径规范三节更新

**依赖**：WO-A1 + WO-B2（topics.json 和文件迁移均完成）  
**修改文件**：`workbench-structure.md`

---

## 变更范围

更新以下三节，其余内容（仓库性质、文档流转流程、index.json 结构等）保持不变：

1. **路径命名规范** — topic-path 格式、prefix 公式、示例表格
2. **topics.json 的作用** — 结构说明、dir 字段、示例
3. （可选）**新增文档时需调整的文件清单** — 说明文字微调

---

## 修改内容

### 1. 替换「路径命名规范」节中的 topic-path 说明

**旧内容**（找到包含 `topic-path` 来源说明的段落）：

```markdown
**`topic-path` 来源**：从 `topics.json` 查询（见下节）。

**文内创建时间**：...
```

在「`topic-path` 来源」行替换为：

```markdown
**`topic-path` 格式**：固定 2 段：`<project>/<doc-theme>`
- `project`：topics.json 中的 `dir` 字段，或 `repo` 短名（`/` 后的部分）
- `doc-theme`：AI 根据对话内容或文档标题语义推断（kebab-case，英文，无空格）

**`prefix` 固定值**：`../../../`（topic-path 恒为 2 段，N 恒=2）

**文内创建时间**：...
```

### 2. 替换「路径命名规范」节中的 prefix 公式行（如有独立公式段落）

删除或替换：
```
N      := |topic-path|
prefix := "../" × (N+1)
```

改为：
```
prefix = "../../../"  （固定，topic-path 恒为 2 段）
```

### 3. 替换「topics.json 的作用」节的结构示意和示例表格

**旧内容**（结构示意 + 示例表格）：

```markdown
结构示意（精简）：

```json
{
  "topics": [
    {
      "repo": "lulufoo/<source-repo>",
      "m": {
        "<sub-topic>": ["<leaf>", ...]
      }
    }
  ]
}
```

`topic-path` = `<sub-topic>` 或 `<sub-topic>/<leaf>`，例如：

| source-repo | sub-topic | leaf | topic-path |
|-------------|-----------|------|------------|
| `android-dev-docs` | `system-principles` | `binder` | `android-dev-docs/system-principles/binder` |
| `ai-assisted-domain-learning` | `dialogue-distillation-model` | —— | `ai-assisted-domain-learning/dialogue-distillation-model` |
| `common-tech` | `language` | —— | `common-tech/language` |
```

**新内容**：

```markdown
结构示意（v4）：

```json
{
  "version": 4,
  "topics": [
    { "repo": "lulufoo/<source-repo>" },
    { "repo": "lulufoo/<source-repo>", "dir": "<local-dir>" },
    { "dir": "<virtual-local-dir>" }
  ]
}
```

- `dir` 字段：仅当本地目录名与 repo 短名不一致时填写（如 `learning-ai-lmm` → `dir: "ai"`）
- 无 `repo` 的条目（如 `{"dir":"common-tech"}`）：本地遗留目录，无对应 GitHub repo

`topic-path` = `<project>/<doc-theme>`（固定 2 段），例如：

| project（dir 或 repo 短名） | doc-theme（AI 语义推断） | topic-path |
|---|---|---|
| `android-dev-docs` | `binder-ipc-internals` | `android-dev-docs/binder-ipc-internals` |
| `ai-assisted-domain-learning` | `dialogue-distillation-model` | `ai-assisted-domain-learning/dialogue-distillation-model` |
| `common-tech` | `java-concurrency` | `common-tech/java-concurrency` |
| `ai` | `llm-token-generation` | `ai/llm-token-generation` |

> `topics.json` 的 `lulu-workbench` 自身条目描述的是 CTA 在知识语料系统中的自分类，不用于路径推导。
```

---

## 验收

检查修改后的文件，确认：
- 无 `m` 字段、`level-1`、`level-2`、`leaf`、`sub-topic` 等旧术语
- 无 `N = |topic-path|` 或 `"../" × (N+1)` 表达式
- 无 `bl` 字段说明
- 示例表格路径格式均为 2 段 `<project>/<doc-theme>`
