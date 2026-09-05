# write/str_replace 说明发送前填入 scratch 路径

**状态：** Implemented  
**关联待办：** `task_0a9d788642b1` / `task_0a9d788642b1_sub_02`

收敛锁定（本对话 `/converge`）：围栏仍是 scratch 目录 + 当前精确 staged 文件；发送前只填 scratch 真实路径；Stage 只写规则、不列 F*；围栏错误保持泛句；`str_replace` 与 `write` 对齐。

## 问题与目标

✅ Verified（`src-tauri/src/agent/tools/host.rs`）：`write` 说明只写 “write-allowed scratch root”，没有本会话真实路径，也没说 staged 文件可写。

✅ Verified（同文件 `str_replace`）：说明写了 “session scratch or an exact staged file”，仍没有真实 scratch 路径。

✅ Verified（`src-tauri/src/agent/tools/catalog.rs` `from_local_tools`）：说明是编译期静态字符串，原样发给模型。

✅ Verified（`src-tauri/src/agent/turn/run.rs`）：每轮 LLM 调用直接送 `catalog.definitions`，发送前不做替换。

✅ Verified（`src-tauri/src/services/path_fence/mod.rs` `with_session_writes`）：围栏已是 scratch 目录 + 精确 staged 文件。`write` / `str_replace` 撞栏错误为 `path is outside the write fence`（`tools/fs.rs`）。

目标：模型在本轮工具说明里能看到本会话 scratch 的绝对路径；Stage 只作为规则出现。不改围栏范围，不改错误原文。

## 方案

静态说明保留占位 `{session_scratch}`。✅ Verified（`workbench_path_fence.rs`）：scratch 父目录是 `{cache_dir}/agent-scratch`。✅ Verified（`path_fence` `with_session_scratch`）：本会话目录是该父目录下的 session id 段。

发送前只替换占位，不把本轮 F* 路径写进说明。`list_staged` 仍是查 Stage 路径的办法。

```mermaid
flowchart LR
  static["host.rs 静态说明<br/>含 session_scratch 占位"]
  fill["发送前替换为真实 scratch 路径"]
  llm["本轮 tools 发给模型"]
  static --> fill --> llm
```

不改：条数帽、Android、gateway 超时、围栏范围、错误信息列路径。

## Plan

1. ✅ Verified（`host.rs`）：`write` / `str_replace` 说明对齐；scratch 用 `{session_scratch}`；Stage 写「当前精确 staged 文件」并指向 `list_staged`。
2. ✅ Verified（`catalog.rs` `fill_session_scratch`、`tools/mod.rs` `fill_turn_scratch`、`run.rs`）：`discover_and_merge` 之后、第一次 LLM 调用之前替换。路径来自 `PathFence::session_scratch_root`。
3. ✅ Verified（本会话 cargo）：`write_and_str_replace_keep_scratch_placeholder_and_staged_rule`、`fill_session_scratch_replaces_placeholder_and_does_not_list_staged`、`session_scratch_root_matches_write_allow`、`run_loop_offers_host_file_tools_and_keeps_scratch_writes_inside_cache`、围栏泛句 2 条，共 7 通过。
