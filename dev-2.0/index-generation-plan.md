# 一级索引生成 Plan（已审查版）

## 目标

为每个 KNOWLEDGE_CORPUS 仓库补充 `description` + `keywords` 字段，推送到各仓库的 `.repository-type.json`，然后重新生成 `topics.json` 与一级索引（供粘贴到 system prompt）。

**当前权威：** corpus 成员清单见仓库根 `.cache/knowledge-index.json`（人工维护）；`topics.json` 由「全量同步」从该 JSON 生成，不再由脚本写入 `refactor-2.0/knowledge-index.md`。

---

## 三阶段流程

### Phase A — Copilot 读取仓库内容，生成描述草稿

1. 用 `gh api` 依次读取各 repo 的 README.md、_index.md（如有）、根目录列表
2. 生成 `description`（≤25字，含3-5核心词）+ `keywords`（数组）
3. 写入 `.cache/repo-descriptions.json`（已在 `.gitignore`）

### Phase B — `scripts/push_descriptions.py`（新建）

4. 读取 `.cache/repo-descriptions.json`
5. 对每个 repo：读当前 `.repository-type.json` + SHA → 合并字段 → `gh api PUT` 写回
6. 只处理草稿文件里明确列出的 repo（不遍历完整 topics.json，避免误操作 lulufoo/lulu-workbench）

### Phase C — `update_topics_from_github.py`（更新）

7. 从 `.cache/knowledge-index.json` 读取 corpus 成员（`knowledge_index_loader.load_knowledge_index`）
8. 对每条成员写 topics 条目：`description` 来自 index；`keywords` 来自 GitHub `.repository-type.json`（best-effort）
9. 保留 `topics.json` 中无 `repo` 的虚拟条目（如 `common-tech`、`inbox`）及既有 `dir` 字段

---

## 审查发现的修订点

| # | 问题 | 修订 |
|---|------|------|
| 1 | `_is_knowledge_corpus` 改返回类型会破坏调用链 | 保留原函数，新增 `_read_repo_meta()` |
| 2 | 一级索引输出路径 | 改为 `.cache/knowledge-index.json`（非 refactor-2.0） |
| 3 | `lulufoo/lulu-workbench` 是 PERSONAL_WORKBENCH，Phase B 会误操作 | push_descriptions.py 只读草稿文件的 repo 列表 |
| 4 | `learning-ai-lmm` 的 `"dir":"ai"` 必须保留 | update_topics 合并时保留既有 `dir` |

---

## 涉及文件

| 文件 | 操作 |
|------|------|
| `.cache/repo-descriptions.json` | Phase A 生成（临时草稿，gitignored） |
| `.cache/knowledge-index.json` | corpus 权威清单（可提交） |
| `scripts/push_descriptions.py` | Phase B 新建 |
| `update_topics_from_github.py` | Phase C 更新 |
| `topics.json` | Phase C 重新生成（含 description/keywords） |
