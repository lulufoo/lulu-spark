# Refactor 1.1 — 执行清单

> 状态：☐ 未开始 · ⚙ 进行中 · ✅ 完成 · 🔒 门控（需人工确认）

---

## 执行顺序

### 批次 1（无依赖，可并行启动）

| ID | 状态 | 施工单 | 修改目标 |
|----|------|--------|---------|
| WO-A1 | ☐ | [wo-a1.md](wo-a1.md) | `topics.json` |
| WO-B0 | ☐ | [wo-b0.md](wo-b0.md) | `scripts/flatten_to_doc_theme.py`（新建） |

### 批次 2（WO-A1 完成后，可并行）

| ID | 状态 | 施工单 | 修改目标 |
|----|------|--------|---------|
| WO-A2 | ☐ | [wo-a2.md](wo-a2.md) | `update_topics_from_github.py` |
| WO-D1 | ☐ | [wo-d1.md](wo-d1.md) | `skills/ddm/references/ddm-p0-normalize.md` |
| WO-D2 | ☐ | [wo-d2.md](wo-d2.md) | `skills/theme-line/SKILL.md` |

### 批次 3（WO-B0 完成后）

| ID | 状态 | 施工单 | 说明 |
|----|------|--------|------|
| WO-B1 | 🔒 | [wo-b1.md](wo-b1.md) | 人工审核 dry-run 结果，需签字 PASS |

### 批次 4（WO-B1 PASS 后）

| ID | 状态 | 施工单 | 修改目标 |
|----|------|--------|---------|
| WO-B2 | ☐ | [wo-b2.md](wo-b2.md) | 全量文件迁移 + index.json |

### 批次 5（WO-B2 + WO-A1 均完成后，可并行）

| ID | 状态 | 施工单 | 修改目标 |
|----|------|--------|---------|
| WO-C1 | ☐ | [wo-c1.md](wo-c1.md) | `server.py` |
| WO-D3 | ☐ | [wo-d3.md](wo-d3.md) | `workbench-structure.md` |

### 批次 6（WO-C1 完成后）

| ID | 状态 | 施工单 | 修改目标 |
|----|------|--------|---------|
| WO-C2 | ☐ | [wo-c2.md](wo-c2.md) | `js/api.js` |

### 批次 7（WO-C2 完成后）

| ID | 状态 | 施工单 | 修改目标 |
|----|------|--------|---------|
| WO-C3 | ☐ | [wo-c3.md](wo-c3.md) | `js/components/modals/move-project-dialog.js`（新建）、`js/components/viewer.js`、`index.html` |

---

## 依赖图

```
WO-A1 ──→ WO-A2
       ├──→ WO-D1
       └──→ WO-D2
            └──→ WO-D3 (还需等 WO-B2)

WO-B0 ──→ WO-B1 🔒 ──→ WO-B2 ──→ WO-C1 ──→ WO-C2 ──→ WO-C3
                               └──→ WO-D3
```

---

## 最终验收

- [ ] `python scripts/flatten_to_doc_theme.py --dry-run` conflicts 为空
- [ ] 迁移后 index.json 所有 common_path 物理文件存在
- [ ] Web UI 随机打开文档，nav 链接无 404
- [ ] C 功能：切换项目 → viewer 关闭 → 页面刷新 → 文件在新 project 下
- [ ] C rollback：模拟中途失败，文件未部分迁移
- [ ] D：DDM P0 输出路径 `<project>/<doc-theme>/<ts>-<slug>.md`，prefix `../../../`
