# Host 历史条数与用户轮次上限上调技术方案与实施计划

官方对话消息字段：<https://docs.bigmodel.cn/cn/guide/develop/openai/introduction>

**状态：** Implemented  
**关联待办：** `task_bcb571153caf_sub_01`（父任务 `task_bcb571153caf`）

## 问题与目标

✅ Verified（`src-tauri/src/agent/turn/history.rs`）：`truncate_turns` 按 `session.turns.len()` 对照 `MAX_HISTORY_MESSAGES`，按 `role == "user"` 条数对照 `MAX_USER_TURNS`。超标时从最早一条 `user` `drain` 到下一条 `user` 之前；只有一条 `user` 时 `end = kept.len()`，副本被整段清空。

✅ Verified（改前 `src-tauri/src/agent/turn/types.rs`）：`MAX_HISTORY_MESSAGES = 20`，`MAX_USER_TURNS = 8`。20 计数的是 `user` / `assistant` / `tool` 每条 `Turn`，不是用户说了几句。

✅ Verified（`src-tauri/src/agent/turn/run.rs`）：一次用户发送只追加 1 条 `user`；每轮工具再追加 1 条带 `tool_calls` 的 `assistant`，以及每个工具结果 1 条 `tool`。

用户要求：把历史条数上限改为 2000，用户轮次上限改为 200。不改 `truncate_turns` 算法，不改工具回合上限。

## 方案

✅ Verified（落地后 `src-tauri/src/agent/turn/types.rs`）：`MAX_HISTORY_MESSAGES = 2000`，`MAX_USER_TURNS = 200`。只改常数；`history.rs` 判断与 `drain` 行为不变。

✅ Verified（`src-tauri/src/agent/turn/types.rs`）：`MAX_MCP_TOOL_ROUNDS = 25` 未改。单次用户发送最多约 `1 + 25 + 工具调用条数 + 1` 条 `Turn`，低于 2000，现行工具帽下不会再因「一句用户 + 工具来回」单独撞上历史条数帽。

## 验收

✅ Verified（`history_truncation_keeps_system_and_dual_hard_caps`）：用常数本身造 `MAX_USER_TURNS + 1` 对短 `user`/`assistant`，断言系统提示仍在最前、用户条数不超过新上限、最早一轮被丢掉、最新一轮仍在。
