# cognitive-trace-archive 仓库结构说明

> 仓库：`https://github.com/lulufoo/cognitive-trace-archive`
> 用途：供 AI 任务执行时定位文件路径、判断需调整的关联文件。

---

## 仓库性质

`.repository-type.json` 标注该仓库类型为 `KNOWLEDGE_CORPUS`，是对话知识的集中归档库，对应 DDM 流程中的 `CTA_BASE`。

---

## 目录结构

```
cognitive-trace-archive/
├── raw/                        # 层1：原始对话归档（源头，只增不改）
├── distilled/                  # 层2：蒸馏文档
├── digest/                     # 层3：摘要文档（可选层）
├── trace/                      # 层4：认知轨迹文档（可选层）
├── index.json                  # 全局条目注册表（真源）
├── topics.json                 # 主题目录注册表（新增文件时查目录用）
├── templates/
│   └── ddm-normalized-output-template.md
├── scripts/
│   └── unify_index_path_prefix.py
├── normalize.py                # 工具脚本
├── update_topics_from_github.py # 工具脚本
├── dialogue-knowledge-management-guide.md  # 知识管理治理说明
├── .repository-type.json       # 仓库类型元数据
└── README.md
```

每一层（raw/distilled/digest/trace）内部结构相同，均按 `<topic-path>/` 子目录组织：

```
<layer>/
└── <topic-path>/
    └── <ts>-<slug>.md
```

---

## 路径命名规范

| 符号 | 类型 | 说明 |
|------|------|------|
| `ts` | string[12] | `YYYYMMDDHHMM`，东八区本地时间；同一轮归档所有文件共享同一 `ts` |
| `slug` | string | 全小写连字符，不含 `ts` |
| `topic-path` | string | Phase 0 Step 1 确定，全程不变；格式见下节 |

**完整路径公式：**

```
<layer>/<topic-path>/<ts>-<slug>.md
```

**`topic-path` 来源**：从 `topics.json` 查询（见下节）。

**文内创建时间**：一级标题下一行写 `> 创建时间：YYYY年M月D日 HH:MM`，月日不补零，须与 `ts` 一致。

---

## topics.json 的作用

**新增文件时，`topics.json` 是确定 `topic-path` 的依据。**

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

> `topics.json` 的 `cognitive-trace-archive` 自身条目（`distilled/raw/templates/trace`）描述的是 CTA 在知识语料系统中的自分类，不用于路径推导。

---

## index.json 结构（v4，实际文件）

```json
{
  "version": 4,
  "entries": {
    "<32位hex id>": {
      "common_path": "<topic-path>/<ts>-<slug>.md",
      "raw":       true | false,
      "distilled": true | false,
      "digest":    true | false,
      "created_at": "<ts>",
      "trace":     true          // 仅在 trace 文件存在时出现；不存在时省略该字段
    }
  }
}
```

**字段说明：**

| 字段 | 说明 |
|------|------|
| `common_path` | 文件在四层中的共享相对路径（不含层前缀） |
| `raw / distilled / digest` | 对应层文件是否存在，始终写出 boolean |
| `created_at` | 归档时的 `ts`，与文件名中的 `ts` 一致 |
| `trace` | 仅在为 `true` 时写出；旧数据未补全（见 TODO） |

**新增文档时需要同步更新 `index.json`**，写入一个新 entry。

---

## 文档流转流程

```
原始对话
    │
    ▼
raw/<topic-path>/<ts>-<slug>.md        ← 只增不改，源头保留
    │
    ▼  DDM Phase 1-2（蒸馏）
distilled/<topic-path>/<ts>-<slug>.md
    │
    ▼  DDM Phase 4（摘要，可选）
digest/<topic-path>/<ts>-<slug>.md
    │
    ▼  DDM Phase 3（认知轨迹，可选）
trace/<topic-path>/<ts>-<slug>.md
```

**层的必要性：**

- `raw`：源头，建议始终存在；部分早期文档因流程原因 raw=false（见 TODO）
- `distilled`：主要加工产物，大多数条目存在
- `digest`：可选，目前凡有 distilled 的基本都有 digest
- `trace`：可选，仅少数条目存在

---

## 新增文档时需调整的文件清单

| 文件 | 操作 |
|------|------|
| `raw/<topic-path>/<ts>-<slug>.md` | 新建（归档原始对话） |
| `distilled/<topic-path>/<ts>-<slug>.md` | 新建（蒸馏后写入） |
| `digest/<topic-path>/<ts>-<slug>.md` | 按需新建 |
| `trace/<topic-path>/<ts>-<slug>.md` | 按需新建 |
| `index.json` | 新增 entry，id 用 32 位 hex，填写 common_path / raw / distilled / digest / created_at |

---

## 根目录其他文件（一句话说明）

| 文件 | 说明 |
|------|------|
| `dialogue-knowledge-management-guide.md` | 知识管理分层治理原则说明 |
| `normalize.py` | 对话归档格式标准化脚本 |
| `update_topics_from_github.py` | 从 GitHub 同步更新 topics.json 的工具脚本 |
| `scripts/unify_index_path_prefix.py` | 批量修正 index.json 路径前缀的维护脚本 |
| `templates/ddm-normalized-output-template.md` | DDM 标准化输出模板 |
| `.repository-type.json` | 仓库类型元数据，值为 `KNOWLEDGE_CORPUS` |
