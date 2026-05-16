---
name: Software Dev Skills UI
overview: "Release v3.0：Software Dev Skills 与 lulu-dev-skills README 对齐；含执行顺序、本地启动命令、Task 1/2/3 与验收步骤。"
todos:
  - id: task-a-data
    content: Task 1 — 更新 _SKILLS_CONTENT.softwareDev 三组与 lulu-dev-skills URL
    status: pending
  - id: task-b-tooltip
    content: Task 2 — skill-item-rich 的 title 使用 desc
    status: pending
  - id: task-c-lulu-dedup
    content: Task 3 — Lulu Skills 7a 去重（删工具组、加说明组）
    status: pending
isProject: false
---

# Software Dev Skills 弹层与 lulu-dev-skills README 对齐 — 实现计划

> 模板依据：[lulufoo/ai-software-dev `20-plan-template.md`](https://github.com/lulufoo/ai-software-dev/blob/main/ai-dev-workflow-framework/20-plan-template.md)（`gh api` 已读）

**日期：** 2026-05-16  
**版本：** v3.0（Release）  
**对应规格：** [lulufoo/lulu-dev-skills `README.md`](https://github.com/lulufoo/lulu-dev-skills/blob/main/README.md)（✅ 数据源；无独立 `docs/superpowers/specs/...` 设计文档时以 README 表结构为准）  
**状态：** **Release**（本版已收口 TPQA 复审中「可执行性 / 依赖显式化」问题，可作实施底稿）

> **Release 说明：** v3.0 相对 v2.x 的变更见「变更记录」v3.0；实施时按「执行顺序与同文件合并」一节操作即可无推断完成编码与手动验收。

---

## 文件一览

| 文件 | 职责 |
|------|------|
| [`js/main.js`](/Users/lulu/Code/lulu-workbench/js/main.js) | `_SKILLS_CONTENT.softwareDev` 分组与条目；`_openSkillsDialog` 渲染 Skills 弹层（可选：rich 项 `title`） |
| [`app.css`](/Users/lulu/Code/lulu-workbench/app.css) | 仅当选择「可见副标题」方案时增加 `.skill-item-desc` 等样式 |
| [`software_dev_skills_ui_17133820-review.md`](./software_dev_skills_ui_17133820-review.md) | **独立**：SPCA / TPQA 审查与 GAP（**仅问题**，不含解决方案） |

---

## 架构速查（执行任何 Task 前必读）

### 模块边界与职责

```
Skills 静态配置（_SKILLS_CONTENT）  ←  仅数据：title、groups[].name/url/items
         对外：无函数，供 _openSkillsDialog 读取 key

Skills 弹层渲染（_openSkillsDialog）  ←  将 groups 转为 HTML；绑定点击复制到剪贴板
         对外：_openSkillsDialog(key) — key ∈ workbench | softwareDev | lulu
```

### 依赖方向（单向，禁止反向）

```
index.html（菜单按钮）  →  main.js（事件 → _openSkillsDialog）
_openSkillsDialog      →  DOM `#skills-dialog`（仅渲染，不回调配置层）
```

### 数据流

```
用户点击「Software Dev Skills」 → _openSkillsDialog('softwareDev')
  → 读取 _SKILLS_CONTENT.softwareDev
  → 拼接 skill-group / skill-item HTML → 注入 #skills-dialog-body → 绑定 copy
```

### 命名合约（跨 Task 必须一致）

| 约束 | 值 |
|------|---|
| GitHub 组织/仓库 | `lulufoo/lulu-dev-skills` |
| 分组链接 pattern | `https://github.com/lulufoo/lulu-dev-skills/tree/main/<subdir>` |
| 指令字符串（`cmd`） | 与各 skill `SKILL.md` 的 YAML `name` 一致：`sync-rules`、`bug-analysis`、`cursor-rule-guard` |
| 分组标题（与 README 一致） | `工具类` / `质量与缺陷分析` / `研发与开发过程` |

---

## Task 列表与依赖

| Task | 模块 | 依赖 | 被依赖 | 估计步骤数 |
|------|------|------|--------|----------|
| Task 1 | `js/main.js` 中 `_SKILLS_CONTENT.softwareDev` | 无 | Task 2（数据契约） | 4 |
| Task 2 | `js/main.js` 中 `_openSkillsDialog`；`app.css`（可选） | Task 1 Step 1 完成（`softwareDev` + `desc` 已填） | 无 | 3 |
| Task 3 | `js/main.js` 中 `_SKILLS_CONTENT.lulu`（7a） | 无 | Task 1 Step 3 中 Lulu 回归验收 | 2 |

---

## 执行顺序与同文件合并（`main.js` · Release 固定）

三个 Task 均修改 [`js/main.js`](/Users/lulu/Code/lulu-workbench/js/main.js)。**请按下列顺序编辑并保存同一文件**，再进入 Task 1 的 Step 2–4（浏览器验收与提交），避免 TPQA **NEW-1** 类顺序歧义：

| 次序 | 动作 | 说明 |
|------|------|------|
| 1 | **Task 1 · Step 1** | 写入 `softwareDev` 三组与 `items[].desc` |
| 2 | **Task 3 · Step 1–2** | 删除 Lulu「工具」组，prepend「说明」组（Task 1 **Step 3** 检视 Lulu 前必须完成） |
| 3 | **Task 2 · Step 1–2** | 修改 `_openSkillsDialog` 内 `skill-item-rich` 的 `title`（及可选 `app.css`） |
| 4 | **Task 2 · Step 3** + **Task 1 · Step 2–3** | 按手动验收步骤 1–5 与 Step 3 回归；最后 **Task 1 · Step 4** 提交 |

**合并提示：** 若单次提交，可在完成 1–3 步编辑后运行一次页面自测，再 `git add js/main.js`（及若改动的 `app.css`）。

---

## Task 1: 对齐 softwareDev 分组与仓库链接

**目标：** 「Software Dev Skills」弹层展示与 README 一致的三类技能、三条 GitHub 目录链接、三条可复制指令。

**为什么这样划分：** 数据与展示契约集中在 `_SKILLS_CONTENT.softwareDev`，与 README 表一一对应，避免散落多处硬编码。

**模块职责：** 见「架构速查」中静态配置与渲染边界。

**Files:**

- Modify: [`js/main.js`](/Users/lulu/Code/lulu-workbench/js/main.js)（替换 `softwareDev` 对象整块）

**验收标准：**

- 弹层内出现三个分组标题，文案为：`工具类`、`质量与缺陷分析`、`研发与开发过程`。
- 每组标题链到：
  - `.../tree/main/sync-rules`
  - `.../tree/main/bug-analysis`
  - `.../tree/main/cursor-rule-guard`
- 三条 skill 复制分别为 `sync-rules`、`bug-analysis`、`cursor-rule-guard`。
- 不再引用 `ai-software-dev/.../skills/bug-analysis` 作为 Software Dev Skills 的组链接。

**依赖：** 无  
**被依赖：** Task 2（已纳入；见「逐条确认」第 4 条）

> 本需求为**静态配置更新**，仓库内若无既有 E2E/单测，不强行套用模板中的 Jest 代码块；下列 Step 按「可观测结果」改写。

- [ ] **Step 1: 修改 `softwareDev.groups`**

按 README 填充三组 `name`、`url`、`items`。每条 **`items[]` 必填 `cmd`、`name`、`desc`**：`desc` 与 README「触发场景」列一致（**逐条确认第 4 条**：做 Task 2，`desc` 用于原生 `title` tooltip；若某条缺 `desc` 则回退为「点击复制指令」——见「错误处理汇总」，应仅在数据疏漏时出现）。

- [ ] **Step 2: 本地打开 Workbench 并验收**

**工作目录（固定）：** [`lulu-workbench`](/Users/lulu/Code/lulu-workbench) 仓库根（含 [`package.json`](/Users/lulu/Code/lulu-workbench/package.json) 的目录）。

**本地启动（固定，二选一；依据：✅ `package.json` 仅有 `"test": "vitest run"`，无 `dev` / `start`）：**

```bash
cd /Users/lulu/Code/lulu-workbench
npx --yes serve . -l 8080
# 或
python3 -m http.server 8080
```

浏览器打开 **`http://127.0.0.1:8080/`**（若端口冲突，改用终端提示的端口并替换下文中的 URL）。

**前置：** 须已按「执行顺序与同文件合并」完成 Task 1 Step 1、Task 3、Task 2 中于 `main.js`（及可选 `app.css`）的编码，否则以下**手动验收步骤 1～5** 无法通过。

按下述 **手动验收步骤** 执行，并对照上文「验收标准」。相对历史审查中的初判 TPQA-1 / TPQA-2：本 Release Plan 已内联启动命令与步骤，**不得以「惯例」代替下文**。

**手动验收步骤（与验收标准对齐）：**

1. 打开页面内 **Skills** 菜单，选择 **Software Dev Skills**，弹出对话框。
2. 弹层内出现三个分组标题，依次为：`工具类`、`质量与缺陷分析`、`研发与开发过程`。
3. 依次点击每组标题旁的「↗」链，确认打开 GitHub，且路径分别为 `lulufoo/lulu-dev-skills` 仓库下 `tree/main/sync-rules`、`tree/main/bug-analysis`、`tree/main/cursor-rule-guard`。
4. 依次点击三条 skill 行（触发复制），确认剪贴板内容依次为 `sync-rules`、`bug-analysis`、`cursor-rule-guard`（以浏览器/环境支持剪贴板为准）。
5. **（Task 2）** 依次将鼠标悬停在三条 skill 行上，确认浏览器原生提示（`title`）与 README「触发场景」语义一致（即与对应 `desc` 一致；因 **TPQA-3 · 3a**，以 README 为对照源）。

- [ ] **Step 3: 回归其它 Skills 入口**

打开 **Lulu Workbench Skills**，确认与改前一致。打开 **Lulu Skills**：**逐条确认第 7 条 · 7a** — 已无原「工具 → 规则同步」组；弹层内应有指向 **Software Dev Skills** 的说明（实现方式见 **Task 3**）；其余 `lulu` 分组未被误删。

- [ ] **Step 4: 提交（由你触发时）**

```bash
git add js/main.js app.css
git commit -m "$(cat <<'EOF'
fix(ui): Software Dev Skills, title tooltips, Lulu 7a dedup

EOF
)"
```

（若未改 `app.css`，可只 `git add js/main.js`。）

---

## Task 2: 触发场景以 tooltip（`title`）呈现

**目标：** 悬停每条 `skill-item-rich` 时，浏览器原生提示显示 README「触发场景」摘要（不改变行内布局，优先仅用 `title`）。

**为什么这样划分：** 与 Task 1 数据分工：Task 1 提供 `desc`，Task 2 负责渲染进 DOM；**逐条确认第 4 条已采纳执行本 Task**。

**模块职责：** `_openSkillsDialog` 内生成 `skill-item-rich` 时写入 `title`；不强制改 `app.css`（无可见副标题方案时跳过额外样式）。

**Files:**

- Modify: [`js/main.js`](/Users/lulu/Code/lulu-workbench/js/main.js)
- Modify（可选）: [`app.css`](/Users/lulu/Code/lulu-workbench/app.css)

**验收标准：**

- 悬停三条 skill 时，`title` 为对应 `desc`；与 README「触发场景」可对照（**3a 瘦计划**下以 README 为规格源）。

**依赖：** Task 1 Step 1（`softwareDev` 与 `desc` 已落地）  
**被依赖：** 无

- [ ] **Step 1:** 渲染 `skill-item-rich` 时：`title` 取 `i.desc`，若缺省则 `点击复制指令`（与错误处理表一致）。

- [ ] **Step 2:** 若需可见副标题（非仅原生 tooltip），再在模板增加节点并改 `app.css`；否则跳过。

- [ ] **Step 3:** 与 Task 1 **手动验收步骤** 第 5 步一起核对悬停与复制；若 **Task 3** 已完成，**Lulu Skills** 弹层按 Task 1 Step 3 一并检视。

---

## Task 3: Lulu Skills 去重（7a）

**目标：** 从 `lulu` 中移除与 **Software Dev Skills** 重复的「工具 / 规则同步（sync-rules）」入口；在弹层顶部用**说明组**引导用户改用 **Skills → Software Dev Skills**。

**依据：** 「逐条确认」第 7 条采纳 **7a**。

**Files:**

- Modify: [`js/main.js`](/Users/lulu/Code/lulu-workbench/js/main.js)（`_SKILLS_CONTENT.lulu.groups`）

**验收标准：**

- `groups` 中**删除**原「工具」组（含 `lulu-skills/.../sync-rules` 链接与 `sync rules` 指令）。
- **最前**插入一组（建议标题 `说明`）：`items` 至少含**一条字符串项**，文案语义为「规则同步等开发类技能请通过 **Software Dev Skills** 使用」；`url` 可指向 [lulu-dev-skills](https://github.com/lulufoo/lulu-dev-skills) 仓库根或 README。
- 其余分组（通用模型、AADL 等）仍在且未被误删。

**依赖：** 无  
**被依赖：** Task 1 **Step 3** 中「Lulu Skills」回归（须先完成本 Task 编码）

- [ ] **Step 1:** 删除「工具」整组。
- [ ] **Step 2:** prepend「说明」组与提示字符串项（实现时可微调措辞，须满足上表）。

---

## 错误处理汇总

| 场景 | 处理方式 | 实现 Task |
|------|---------|-----------|
| `desc` 缺失 | 回退 `title` 为「点击复制指令」 | Task 2 |
| 剪贴板 API 失败 | 维持现有行为（浏览器未逐项在本需求扩展） | 不纳入 |

---

## 审查记录

**独立成文**：原审查见开发机 Cursor 目录下 `software_dev_skills_ui_17133820-review.md`（SPCA、TPQA、仅问题清单；**本仓库不副本**，以免与 Plan 双源）。

---

## 逐条确认（问答应）

| 条 | 主题 | 状态 |
|----|------|------|
| 1 | TPQA-1：Plan Step 2 是否写死本地预览命令与工作目录 | **Release 已收口**：见 Task 1 Step 2（`cd` + `npx serve` / `python -m http.server` + URL） |
| 2 | TPQA-2：是否在 Plan 内增加逐步「手动验收步骤」 | **已采纳**：见 Task 1 Step 2 |
| 3 | TPQA-3：Plan 是否内嵌完整 `softwareDev` / Task 2 验收措辞 | **已采纳 · 3a 瘦计划**：不内嵌 JSON；规格以 [README](https://github.com/lulufoo/lulu-dev-skills/blob/main/README.md) 为准；Task 2 验收用语收紧列入后续迭代 |
| 4 | Task 2 / 维度 7：`desc` 与 `title` tooltip 是否实施 | **已采纳**：实施 Task 2；Task 1 中 `desc` 必填（缺省回退见错误处理表） |
| 5 | 维度 10：剪贴板 API 失败是否纳入 | **已采纳 · E**：本期不扩展，维持「错误处理汇总」中不纳入 |
| 6 | SPCA：执行后在哪记录「已核对」 | **已采纳 · F**：以 Plan 的 Task 1 手动验收 + 与 README 对照即算完成；**不回写** `*-review.md` 的 SPCA 表 |
| 7 | Lulu 与 Software Dev 中 sync-rules 是否去重 | **已采纳 · 7a**：Task 3 删除 Lulu「工具」组并加说明组指向 Software Dev Skills |

---

## 变更记录

| 版本 | 日期 | 变更内容 |
|------|------|---------|
| v1.0 | 2026-05-16 | 初稿（README 对照 + 实现要点） |
| v1.1 | 2026-05-16 | 按 `20-plan-template.md` 重组：文件一览、架构速查、Task 表、验收、错误处理、审查、变更记录 |
| v1.2 | 2026-05-16 | 按 `21-tpqa-plan-evaluation-framework.md` 补充十维审查、GAP 与修复建议 |
| v1.3 | 2026-05-16 | TPQA：补充附录 A/B（本地启动命令与验证序列），收紧 GAP-1/2 的可执行性 |
| v1.4 | 2026-05-16 | 审查记录迁至 `software_dev_skills_ui_17133820-review.md`；Task 1 Step 2 链至该文件附录 |
| v1.5 | 2026-05-16 | 审查记录仅保留问题清单（删除一切方案/附录）；计划 Step 2 与文件一览描述同步调整 |
| v1.6 | 2026-05-16 | 逐条问答：第 1 条（TPQA-1 写进 Plan）暂缓；新增「逐条确认」表 |
| v1.7 | 2026-05-16 | 第 2 条（TPQA-2）采纳：Task 1 增加「手动验收步骤」；Step 3 明确回归另两个 Skills |
| v1.8 | 2026-05-16 | 第 3 条（TPQA-3）采纳 **3a 瘦计划**：不内嵌 `softwareDev` JSON；Task 2 验收措辞后续迭代 |
| v1.9 | 2026-05-16 | 第 4 条：采纳 Task 2；Task 1 契约 `desc` 必填；手动验收增第 5 步；Task 2 章节改为必选 |
| v2.0 | 2026-05-16 | 第 5 条：剪贴板失败本期不扩展（维持错误处理表） |
| v2.1 | 2026-05-16 | 第 6 条：SPCA 以 Plan 验收为准，不回写审查文件 |
| v2.2 | 2026-05-16 | 第 7 条 **7a**：新增 Task 3（Lulu 去重）；更新 Task 列表与回归说明；范围外改写 |
| v2.3 | 2026-05-16 | TPQA 复审：补回 frontmatter 中 Task 2 todo（与 Plan 正文一致） |
| **v3.0** | **2026-05-16** | **Release：写死本地启动与工作目录（关 TPQA-1）；新增「执行顺序与同文件合并」（关 NEW-1）；更新 Task 表「被依赖」列；逐条确认第 1 条改为已收口；Step 4 支持 `app.css`** |

---

## 范围外（需单独决策）

- （原「Lulu sync-rules 与 Software Dev 去重」已通过 **逐条确认第 7 条 · 7a** 纳入 **Task 3**；若改走 7b/7c 策略另开迭代。）

