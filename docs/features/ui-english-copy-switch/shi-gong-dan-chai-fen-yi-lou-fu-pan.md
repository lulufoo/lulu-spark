# 施工单拆分遗漏复盘

**Feature：** `feature-20260718130642-22a23648`（Workbench UI 英文化一次切换）  
**日期：** 2026-07-19  
**性质：** 合入后对照三表审查发现的过程缺陷记录（非产品需求变更）

---

## 一句话结论

✅ Verified：缺口主因是 **lulu-tasks 拆分未把表 B2 的文件落点挂进 `target_files`**，不是「清单没写」或「task 写了没做」。

---

## 现象

合入 `main` 后，按三份命名清单对照 `frontend/`（排除 Skills 目录文件）复查，主路径（shell / Home Hub / Plan Tasks / components / Assistant / modals）已英文化，但仍有用户可见中文残留。

✅ Verified（扫描工具输出，2026-07-19）：残留集中在：

| 文件 | 清单依据（摘要） |
|------|------------------|
| `frontend/js/feed.js` | 表 B2 #74–75；表 B「加载中… / 原文 / 展开全文…」等 |
| `frontend/js/header-sync.js` | 表 B「↓ 更新项目 / ⟳ 本地刷新」；表 B2 #59–60 |
| `frontend/js/api.js` | 表 B2 #92；表 B「请求失败」 |
| `frontend/js/comment-reorder.js` | 表 B2 #93 |
| `frontend/js/mermaid-render.js` | 表 B2 #94 |
| `frontend/js/builders-assistant.js` | 表 B「打开 / 关闭」 |
| `frontend/js/corpus-index.js` | 表 B 补全约定类报错 |

准确性偏差（已一并修）：`utils.js` 中 `→ Med` 相对表 B2 #65 的 `→ Medium`。

清单来源路径：

- 表 A：`.cache/.../lulu-spec/revision1/xu-tao-lun-que-ren-fan-yi-qing-dan.md`
- 表 B：`.cache/.../lulu-spec/revision1/pu-tong-ming-ming-qing-dan.md`
- 表 B2：`.cache/.../lulu-design/revision1/pu-tong-ming-ming-qing-dan-b2.md`

---

## 根因

### 1. 施工单按「点名文件」拆，未按三表落点拆

✅ Verified：`lulu-tasks/r1/task-list.md` 中 t1–t6 目标为：

- t1 `index.html`
- t2 `home-hub.js`
- t3 `plan-task/*`
- t4 `components/*.js`（排除 skills-*-content）
- t5 具名 assistant HTML/JS
- t6 `main.js` + `modals/` + `settle-dialog`

同目录下对 `feed.js` / `header-sync.js` / `api.js` / `comment-reorder.js` / `mermaid-render.js` / `corpus-index.js` **无** task 目标条目。

✅ Verified：表 B2 正文已写明上述文件为出现位置（如 #74 `feed.js`、#60 `header-sync.js`、#92 `api.js`）。

⚠️ Inferred：tech-doc 任务表（T0–T8）本身也以高频/点名文件为主，施工单基本镜像 tech-doc，未再做「B2 落点 → task」闭合。

### 2. 通配写在索引、具名写在 task.md，执行跟具名走

✅ Verified：`task-list.md` t5 写了 `frontend/js/*-assistant.js`，但 `tasks/t5/task.md` 的 `target_files` 只列 ai / read-later / plan-task 三个助手；`builders-assistant.js` 可匹配通配却未进具名列表，执行时漏改。

### 3. P4 验收未形成全量中文闭合

✅ Verified：t8 曾热修 `utils.js`、`note-assistant.js`（见 P4 验证记录），但未扫到 feed / header-sync 等缺口文件。

### 非本批失败项

✅ Verified：`main.js` Skills 对话框内模型中文名/描述属「Skills 另议」；`task-list.md` Exclusion 已排除 Skills 目录展示名。不计入本缺陷。

---

## 已做补全（合入后 hotfix）

✅ Verified（工作区文件与针对性 vitest，2026-07-19）：上述缺口文件已按表 B/B2 英文化；`utils.js` 对齐 `Medium` / `Cycle importance`；相关测试（含 `header-sync` / `builders-assistant` / `comment-reorder` / P4 抽检）已同步。

再扫非注释用户文案：除 Skills 模型目录外，前述缺口文件已无 CJK 用户串。

---

## 流程教训（可复用）

1. **清单有落点文件时，施工单 `target_files` 必须覆盖，或显式写入 Exclusion 并说明原因。**  
   ⚠️ Inferred：否则会出现「B2 已审查、代码未改」的结构性漏改。

2. **task-list 通配与 task.md 具名必须一致；执行以具名为准时，通配不可视为已覆盖。**

3. **P4 / 切换后验收应对照表 A∪B∪B2 做文件级闭合（或自动化 CJK 扫描），不能只做关键路径抽检。**

---

## 相关产物

| 产物 | 路径 |
|------|------|
| 施工单索引 | `.cache/.../lulu-tasks/r1/task-list.md` |
| tech-doc | `.cache/.../lulu-plan/revision1/tech-doc.md` |
| P4 验证记录 | `docs/features/ui-english-copy-switch/p4-post-switch-verification.json` |
