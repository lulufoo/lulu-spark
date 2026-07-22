# 恢复演练清单

Source: tech-doc T11 / AC-恢复（cycle `feature-20260721160725-475641f1`）。

本清单记录 plan→todo 硬切后的**操作恢复**步骤。正向迁移脚本不提供自动反向；自动化否证见 `tests/todo-task-recovery-drill.test.js`。

---

## 前提

- ✅ Verified（`scripts/migrate-plan-tasks-to-todo-tasks` 文件头）：迁移脚本声明 `Not invoked at application startup`，须人工在 Host 升级后、开放 todo API 前执行。
- ✅ Verified（`src-tauri/src/services/local_http/mod.rs`）：HTTP 层对 todo API 做 durable gate 检查，注释写明 `no auto-migrate here`。
- ✅ Verified（`scripts/` 目录列举）：仅有正向脚本 `migrate-plan-tasks-to-todo-tasks`，无 `todo_tasks`→`plan_tasks` / `todo.md`→`plan.md` 反向迁移入口。

---

## 演练 A — 代码回滚回到旧契约

目标：Host/MCP 代码回到硬切前契约（`plan_*` / `/api/plan-*`）。

1. 在交付分支上用 git 回滚（`git revert` 或检出硬切前 revision）Host 与 knowledge-mcp 相关提交。
2. 重新构建并启动 App。
3. 通过信号：旧 `plan_*` HTTP/tool 可用；新 `todo_*` 契约不再作为交付面。

- ✅ Verified（tech-doc Phase 表 SK-1/SK-2「可回退性」）：契约侧回退姿态为代码回滚。
- ⚠️ Inferred：具体 revert 范围随合入历史变化，演练时按当前分支 `git log` 选定硬切起点。

---

## 演练 B — 已升级客户端回装旧 SKILL + Reload MCP

目标：Agent 侧不再驱动 `todo-task` / `todo_*` tool。

1. 回装硬切前的 `plan-task` SKILL（覆盖当前 `todo-task` 安装）。
2. 在 Cursor（或当前平台）执行 Reload MCP（或重启 App）使 tool 注册刷新。
3. 通过信号：MCP tool 目录回到旧 `plan_*`；SKILL 再次指向 `/plan-task`。

- ✅ Verified（tech-doc SK-3「可回退性」、验证章）：契约变更后须 Reload MCP；恢复姿态含回装旧 SKILL。
- ⚠️ Inferred：平台 Reload 入口名称因 IDE 而异，以本机 Cursor MCP 面板为准。

---

## 演练 C — 磁盘旧布局走运维备份恢复（无自动反向）

目标：磁盘从 `todo_tasks/` + `todo.md` 回到 `plan_tasks/` + `plan.md`。

1. **不**运行任何「反向迁移」脚本（仓库不提供；若出现则否证本 feature）。
2. 从运维备份恢复迁移前的 knowledge 根目录布局（含 `plan_tasks/`、各任务 `plan.md`、`index.json` 等）。
3. 若曾写入 `todo_tasks/.migration_gate_passed`，恢复备份后该门闩随备份状态而定；勿手写伪造反向迁移。

- ✅ Verified（tech-doc 非目标 / AC-恢复）：不提供自动 `todo_tasks`→`plan_tasks` / `todo.md`→`plan.md` 反向迁移；旧布局走运维备份恢复。
- ✅ Verified（`scripts/migrate-plan-tasks-to-todo-tasks`）：`OLD_ROOT = "plan_tasks"` → `NEW_ROOT = "todo_tasks"` 仅正向。

---

## 边界 — 单条迁移失败默认不阻断 App 启动

与 API 门闩同时成立：

| 现象 | 期望 | 依据 |
|---|---|---|
| 迁移脚本部分失败（exit ≠ 0） | 不写 / 清除 `.migration_gate_passed` | ✅ Verified（脚本 `clear_gate` + 非零退出；`tests/migrate-plan-tasks-to-todo-tasks.test.js`） |
| App 冷启动 | **不阻断 App 启动**；进程不因缺门闩而 abort | ✅ Verified（`ensure_todo_api_ungated` 返回 503 JSON，无 `process::exit` / `panic!`） |
| 调用 todo API 且无门闩 | todo API 不可用（gated），其它 App 能力仍可启动 | ✅ Verified（`local_http` 仅对 todo API path 调用 `ensure_todo_api_ungated`） |

演练检查：

1. 故意制造单条 body 冲突使迁移失败。
2. 确认 App 仍能启动。
3. 确认 `/api/todo-*` 返回门闩错误；非 todo 路径不受影响。

---

## 否证信号

任一成立则本恢复模型被否证，须停演练并升级缺陷：

- 仓库出现 `todo_tasks`→`plan_tasks`（或 `todo.md`→`plan.md`）自动反向迁移实现/脚本。
- Host 在启动路径自动调用 `migrate-plan-tasks-to-todo-tasks`。
- 缺 `.migration_gate_passed` 时 App 无法启动（而非仅 todo API gated）。

自动化门闩：`npm test` 含 `tests/todo-task-recovery-drill.test.js`。
