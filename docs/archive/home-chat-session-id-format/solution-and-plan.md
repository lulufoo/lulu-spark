# Home Chat 新会话 ID 格式

**状态：** Implemented  
**关联待办：** `task_162b0ac4a0c8_sub_01`

## 问题与目标

✅ Verified（`src-tauri/src/agent/session/store.rs` `create_session`）：新会话 id 为 `sess_` + `random_hex12()`，形如 `sess_84dafc26cec6`。

目标：新会话改为 `workbench_chat_` + 32 位字符。磁盘上已有的 `sess_*` 不改、不迁移，继续可读。

## 方案

- 生成：`create_session` 改为 `workbench_chat_` + `random_entry_id()`。✅ Verified（`src-tauri/src/services/id.rs`）：`random_entry_id()` 已是 32 位小写 hex。
- 读写：`load_session` / `save_session` / `session_file_path` 不校验前缀。旧 `sess_*` 文件名保持原样即可打开。
- 测试：断言新 id 前缀与 32 位 hex；补一条旧 `sess_*` 存取；把 `starts_with("sess_")` 的生成断言改掉。夹具里的 `sess_*` 不动。

不改 Android，不改 Copy Session ID 入口。

## Plan

1. ✅ Verified（`src-tauri/src/agent/session/store.rs`）：`create_session` 现为 `workbench_chat_` + `random_entry_id()`。
2. ✅ Verified（本会话 cargo）：`create_session_id_is_workbench_chat_plus_32_hex`、`load_session_still_reads_legacy_sess_id` 等 6 项通过。
3. 夹具里的 `sess_*` 未改。
