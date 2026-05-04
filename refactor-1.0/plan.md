# 执行计划 v1.0

> 配套文件：framework.md（框架）、checklist.md（验证清单）
> 执行原则：每个 Phase = 一个 Git commit，commit 前必须通过验证门。

---

## 总览

| Phase | 内容 | 时长 | 风险 | 详情 |
|---|---|---|---|---|
| 0 | 提取 app.css | 20min | 零 | [phase-0.md](phase-0.md) |
| 1 | 提取 utils.js + api.js + 接入 Vitest | 2-3h | 极低 | [phase-1.md](phase-1.md) |
| 2 | state.js + main.js 骨架 | 1-2h | 低 | [phase-2.md](phase-2.md) |
| 3 | 迁移 Archive 核心（sidebar + cards） | 2-3h | 中高 | [phase-3.md](phase-3.md) |
| 4 | 迁移 Viewer + Dialogs | 2-3h | 中 | [phase-4.md](phase-4.md) |
| 5 | 新增 Feed 模块 | 1-2h | 零 | [phase-5.md](phase-5.md) |
| | **总计** | **约2天** | | |

---

## ⚙️ 执行协议（AI 必须遵守）

**适用范围**：所有 Phase 文件中的所有 Step。

1. **每次只执行一个 Step**，执行完后立即停止
2. **停下来，输出停止信号**：每个 Step 末尾标注的停止信号文本，AI 必须原样输出
3. **等待用户明确说"通过"或"继续"**，才执行下一个 Step
4. **遇到验证失败**：立即停止并报告具体错误，不自行修复后跳过
5. **禁止预判**：不能说"接下来我将执行 Step X.Y"然后自动执行

---

执行时，直接打开对应的 Phase 文件，从第一个 Step 开始。
