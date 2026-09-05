# Host LLM 空闲超时改为 120 秒

官方流式说明：<https://docs.bigmodel.cn/cn/guide/capabilities/streaming>

**状态：** Implemented  
**关联待办：** `task_0a9d788642b1`

## 问题与目标

✅ Verified（`sess_84dafc26cec6`、`agent-error.log`）：两次失败都是 `LLM timeout reading stream`，不是上下文超限。

✅ Verified（`src-tauri/src/agent/llm.rs`）：`DEFAULT_TIMEOUT` 已改为 120 秒，作用于连接/首包与每段 SSE 读的空闲。

目标：先把空闲窗口改为 120 秒，方便实机看长生成能否返回。不改条数帽，不改 Android，不改 `gateway` 的 60 秒。

## 方案

✅ Verified（`llm.rs`、`turn/run.rs`）：对话路径用的是 `llm::DEFAULT_TIMEOUT`。只改这一处常数：`60` → `120`。

✅ Verified（`frontend/src/home/commands/hub.ts`）：`agent_chat_turn` 无第二道前端超时。

单测里的超时用例继续传入短 `Duration`，不跟默认值走。

## Plan

1. ✅ Verified（`llm.rs`）：`DEFAULT_TIMEOUT` 为 `from_secs(120)`。
2. ✅ Verified（本会话 cargo）：`llm_timeout_is_typed_and_not_retried`、`llm_idle_timeout_after_first_sse_chunk`、`llm_keeps_reading_when_chunks_keep_arriving` 通过。
3. 用户重建 Mac Workbench 后再试长生成。
