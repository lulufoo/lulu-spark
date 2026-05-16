# Refactor 1.1 — Topics & Directory Restructure

## TL;DR

四部分重构。新路径公式 `<project>/<slug>/<ts>-<slug>.md`（3级，prefix 固定 `../../../`）。

- **A**：topics.json 只保留 repo 列表，新增 `dir` 字段解决遗留目录名；移除 `m`/`bl`
- **B**：全量迁移脚本自动执行（slug 从文件名提取，diagnose 额外扫描，zh 文件同步 mv）
- **C**：Web UI "切换项目"（本地 mv，带原子性保护）
- **D**：Skill 新建文档策略 = AI 语义推断；迁移策略 = slug 自动提取（两条明确分开）

---

## 路径核心规则

```
new_cp  = <project>/<slug>/<ts>-<slug>.md
slug    = 文件名去掉 12位ts- 前缀和 .md 后缀
project = 当前 top-level 目录名（迁移保持原样）
prefix  = "../../../"（固定，N 恒=2）
zh 文件 = <project>/<slug>/<ts>-<slug>-zh.md
```

---

## 路径异常映射表（脚本内置）

| 当前 top-level 目录 | 问题 | 迁移策略 |
|---|---|---|
| `ai/` | topics.json 无对应 repo 短名 | project 保持 `ai`，topics.json 新增 `{"dir":"ai"}` 虚拟条目 |
| `common-tech/` | 非独立 repo（属 learning-with-ai 子主题） | project 保持 `common-tech`，同上 |
| `learning-with-ai/learning-with-ai/` | 4级嵌套同名 | 第1段=project，slug=filename，通用规则处理 |
| `*/*/subtopic/file`（4级路径） | 超深嵌套（如 `android-dev-docs/system-principles/binder/xxx`） | 忽略中间层，slug=filename |

---

## Phases & Work Orders

| WO | 标题 | 依赖 |
|----|------|------|
| WO-A1 | topics.json 重构 | — |
| WO-B0 | 创建迁移脚本 `flatten_to_doc_theme.py` | — |
| WO-A2 | 简化 `update_topics_from_github.py` | WO-A1 |
| WO-D1 | DDM P0 normalize Step 1 改写 | WO-A1 |
| WO-D2 | theme-line SKILL Step 1 改写 | WO-A1 |
| WO-B1 | Dry-run + 人工审核 🔒 | WO-B0 |
| WO-B2 | 执行全量迁移 | WO-B1 通过 |
| WO-C1 | server.py `/api/move-project` | WO-B2 |
| WO-D3 | workbench-structure.md 规范更新 | WO-A1 + WO-B2 |
| WO-C2 | `js/api.js` 新增函数 | WO-C1 |
| WO-C3 | 前端弹窗 + viewer 按钮 + index.html | WO-C2 |

---

## 并行执行分组

```
批次1（无依赖，可并行）: WO-A1, WO-B0
批次2（WO-A1 完成后）:   WO-A2, WO-D1, WO-D2
批次3（WO-B0 完成后）:   WO-B1（人工审核）
批次4（WO-B1 PASS 后）:  WO-B2
批次5（WO-B2+A1 均完成）: WO-C1, WO-D3
批次6（WO-C1 完成后）:   WO-C2
批次7（WO-C2 完成后）:   WO-C3
```

---

## Relevant Files

| 文件 | 变更类型 |
|------|---------|
| `topics.json` | 删除 m/bl，version→4，新增 dir 字段 |
| `update_topics_from_github.py` | 移除 build_topics_map、bl、目录遍历 |
| `scripts/flatten_to_doc_theme.py` | 新建 |
| `index.json` | 全量 common_path + translations.zh 更新 |
| `annotations/**/*.json` | 文件重命名 |
| `diagnose/**/*.md` | 额外扫描批量 mv（不在 index layers 中） |
| `raw/distilled/digest/trace/**/*.md` | mv + 内部导航链接更新 |
| `server.py` | 新增 `/api/move-project`（含 rollback） |
| `js/api.js` | 新增 `moveToProject()` + `fetchTopics()` |
| `js/components/viewer.js` | 新增 "↷ 切换项目" 按钮 |
| `js/components/modals/move-project-dialog.js` | 新建 |
| `index.html` | 新增弹窗 HTML 结构 |
| `skills/ddm/references/ddm-p0-normalize.md` | Step 1 改写 |
| `skills/theme-line/SKILL.md` | Save to Archive Step 1 改写 |
| `workbench-structure.md` | 路径规范三节更新 |

---

## Verification Checklist

1. `python scripts/flatten_to_doc_theme.py --dry-run` → 审核 `.cache/migration-preview.json`，`conflicts` 为空，zh 路径正确
2. 执行迁移后：验证 index.json 所有 common_path 物理文件存在（含 zh 文件）
3. Web UI：随机打开文档，各 layer badge 无 404
4. C 正常流：切换项目 → 文件 mv → index 更新 → viewer 关闭 → 页面刷新
5. C 失败流：模拟中途失败，确认 rollback 生效
6. D：执行 DDM P0，确认输出路径为 `<project>/<doc-theme>/<ts>-<slug>.md`，prefix 为 `../../../`

---

## Key Decisions

- B 迁移：doc-theme = slug（自动提取，完全无需 AI 推断）；D 新建：doc-theme = AI 语义推断——两条策略显式分开
- `ai/` / `common-tech/` 遗留目录保留；topics.json 用 `dir` 字段桥接；未来合并到正式 repo 为独立任务
- diagnose/ 层文件不在 index.json `layers[]` 中，迁移脚本需按 old_cp 路径额外扫描
- `/api/move-project` 与 `/api/gh-move`（GitHub 跨 repo）独立，新 API 只做本地 mv
- `bl` 字段经确认仅在 `update_topics_from_github.py` 内部使用，安全移除
- prefix 改为固定写法 `../../../`，Skill 文档中移除 N 动态计算公式
- 语义层级折叠（如 `system-principles/binder/` → `slug/`）是有意权衡，目录简化优先
- `skills/ddm/SKILL.md` 主文件无 topic-path 逻辑，不需修改
- `normalize.py`、`dialogue-knowledge-management-guide.md`、`output-templates.md` 均无路径逻辑，不受影响
