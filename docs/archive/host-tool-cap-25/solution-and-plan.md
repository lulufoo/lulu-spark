# Host 工具回合上限改为 25 技术方案与实施计划

**状态：** Implemented  
**关联待办：** `task_a697e8487834`（本条为子任务记录）

## 问题与目标

✅ Verified（`sess_b1bb2064cd67`）：删 Stage 文档 Turn 标记时，Agent 用默认 50 行 `read` 翻了 7 页后撞上 `MAX_MCP_TOOL_ROUNDS = 8`，回复「工具调用次数已达上限，未继续执行。」，未 `write`/`edit`。

✅ Verified（`src-tauri/src/agent/turn/types.rs` 改前）：上限为回合 8、总调用 16。卡住的是回合 8。

⚠️ Proposed → 落地：把两个上限都改成 25，对齐 Cursor 论坛记录的默认约 25 次工具调用（到点停住，不是本机这种硬失败）。不改 Continue / Resume，不改 `read` 默认 50 行。

## 方案

✅ Verified（落地后 `src-tauri/src/agent/turn/types.rs`）：`MAX_MCP_TOOL_ROUNDS = 25`，`MAX_MCP_TOOL_CALLS = 25`。只改常数；`run.rs` 判断与错误文案不变。

只改回合、不改次数的话，1 工具/回合会在第 17 次先撞上 16。两条一起改。✅ Verified（`run.rs` 里 `tool_rounds` 与 `tool_call_count` 任一越界即停）

## 验收

✅ Verified：`run_loop_stops_after_bounded_mcp_tool_rounds` 用常数本身断言封顶，改 25 后仍应在第 26 次模型请求处停。
