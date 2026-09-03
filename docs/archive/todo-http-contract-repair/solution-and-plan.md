# Todo HTTP Contract Repair — Solution and Plan

## Solution

- ✅ Verified（commit `3c208067`）：Main Host 已移除 todo HTTP 路由；恢复 `/api/todo-*` 会违背该提交声明的边界收缩。
- ✅ Verified（`src-tauri/src/mcp_host/catalog/route_util.rs`）：当前 todo 的迁移 gate 在 MCP Host 的 `gated_todo` 调用路径执行。
- ✅ Verified（`tests/todo-task/recovery-drill.test.js`、`vocab-lock.test.js`）：两份测试仍断言已移除的 Main Host HTTP surface。
- ⚠️ Inferred：将断言迁至 MCP gate，并把 Main Host 的 todo 路由缺席作为负向契约，可保留迁移保护且与当前架构一致。

## Plan

1. ✅ Verified：把恢复演练的 host 断言改为 MCP `gated_todo` 对 `ensure_todo_api_ungated` 的调用契约。
2. ✅ Verified：把词汇锁从要求旧 `/api/todo-*` 路由改为禁止其重现，同时保留 `todo.md` 存储断言。
3. ✅ Verified：在 feature worktree 运行完整 `npm test`。
4. ⚠️ Inferred：测试全绿后，在 feature worktree 重新执行 t1，避免使用主 checkout 中未提交的误写入内容。
