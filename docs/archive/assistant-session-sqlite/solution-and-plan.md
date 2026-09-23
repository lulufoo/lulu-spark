# 助手会话存储升级：按会话拆分的 SQLite

**状态：** 已收敛，未实施  
**日期：** 2026-09-23  
**范围：** Workbench macOS 会话落盘。不实现上下文压缩，格式预留压缩。  
**图：** https://luluboard.app/#b:b_389a535a

---

## 1. 问题与目标

当前一场会话一个 JSON 文件，整份 `turns` 每次重写。Home 列表扫描目录并解析文件。界面回放和发给模型读同一份 `turns`。后续要做上下文压缩时，界面原文和模型上下文会分叉，这份结构撑不住。

目标：

- 会话改用 SQLite，一场会话一个库文件。
- 界面原文和模型历史分开存。
- 运行时不再读旧 JSON。
- 本机旧数据用单独脚本迁一次。
- 压缩功能本次不做；表先建好。

---

## 2. 锁定结论

| # | 结论 |
|---|---|
| 1 | 本次只升级存储并预留压缩表，不实现压缩任务。 |
| 2 | 运行时不兼容 `{session_id}.json`，旧数据用本机脚本迁一次。 |
| 3 | Home 列表读 `sessions-catalog.sqlite`，创建、更新、删除时同步。 |
| 4 | 压缩后早期 `model_steps` 行保留，只从 `model_turns` 去掉引用。 |

已定、不再讨论：

- 后缀是 `.sqlite`，不是 `.vscdb`。
- 一场会话一个文件，不用 Cursor 那种整机 KV。
- 不搬 `conversationState` Protobuf、加密 blob、无列表 `cursorDiskKV`。

---

## 3. 现状

会话目录是 `{cache_dir}/agent/sessions/{session_id}.json`。✅ 已验证（`src-tauri/src/agent/session/store.rs`：`sessions_dir` / `session_file_path`）

默认 `cache_dir` 是 `~/.cache/lulu-workbench`。✅ 已验证（`src-tauri/src/config/settings/types.rs`：`default_cache_dir`）

`session_id` 形如 `workbench_chat_{随机 id}`，禁止 `/`、`\`、`..`。✅ 已验证（`store.rs`：`create_session` / `session_file_path`）

文件内容：`session_id`、`turns[]`、`staged[]`。`Turn` 有 `role`、`content`、`tool_call_id`、`tool_calls`、`name`。`staged` 只记路径，不存文件体。✅ 已验证（`src-tauri/src/agent/session/types.rs`）

写入是整份 pretty JSON，先写 `.json.tmp` 再 rename。✅ 已验证（`store.rs`：`save_session`）

内存 `AIAssistantSession` 只持有当前 `session_id` 等现场，不另存一份对话。✅ 已验证（`types.rs`：`HOLDS_PARALLEL_TURN_STORAGE = false`；`live.rs`）

Home 列表扫描 `sessions/*.json`，按文件修改时间取最近 20 条；标题取第一条用户消息前 48 字。✅ 已验证（`store.rs`：`list_session_summaries`）

界面回放只返回 `role` 为 `user` / `assistant` 且 `content` 非空的 turn。工具结果留在文件里，不走这条 IPC。✅ 已验证（`store.rs`：`load_turns_value`）

发给模型时读同一份 `turns`，组请求时截断到 2000 条消息、200 个用户回合；截断结果不写回。✅ 已验证（`src-tauri/src/agent/turn/history.rs`；`src-tauri/src/agent/turn/types.rs`）

一次用户发送先追加 `user` 并 `persist`，再按循环追加带 `tool_calls` 的 `assistant` 和 `role=tool`。✅ 已验证（`src-tauri/src/agent/turn/run.rs`）

测试必须走 TestSandbox，会话目录不能落到生产 cache。✅ 已验证（`store.rs`：`reject_unisolated_sessions_dir`）

仓库已有 `rusqlite`，用于 `{cache_dir}/keyword-index.sqlite`，不是会话库。✅ 已验证（`src-tauri/Cargo.toml`；`src-tauri/src/services/keyword_index/store.rs`）

---

## 4. 目标文件

```text
{cache_dir}/agent/sessions-catalog.sqlite
{cache_dir}/agent/sessions/{session_id}.sqlite
```

`session_id` 校验不变。测试沙箱规则不变。

---

## 5. 表结构

### 5.1 `sessions-catalog.sqlite`：`sessions`

| 列 | 类型 | 说明 |
|---|---|---|
| `session_id` | TEXT PK | 与文件名一致 |
| `title` | TEXT | 列表标题 |
| `updated_at` | INTEGER | 排序用 |
| `status` | TEXT | 如 `idle` / `running` |

Home 只读这张表，按 `updated_at` 降序取 20 条。

### 5.2 `{session_id}.sqlite`：`meta`

一行。

| 列 | 类型 | 说明 |
|---|---|---|
| `session_id` | TEXT PK |  |
| `created_at` | INTEGER |  |
| `updated_at` | INTEGER |  |
| `title` | TEXT |  |
| `status` | TEXT |  |

文件即隔离。除本行外，会话库内不再重复会话外键。

### 5.3 `messages`

界面原文。压缩后不改。

| 列 | 类型 | 说明 |
|---|---|---|
| `message_id` | TEXT PK |  |
| `seq` | INTEGER | 本场顺序 |
| `role` | TEXT | `user` / `assistant` |
| `content` | TEXT | 界面看到的字 |
| `created_at` | INTEGER |  |

工具调用不进这张表。对应现状：`load_turns_value` 不返回 `tool`。

### 5.4 `model_turns`

模型历史入口。一个用户提交一行。

| 列 | 类型 | 说明 |
|---|---|---|
| `turn_id` | TEXT PK |  |
| `seq` | INTEGER |  |
| `created_at` | INTEGER |  |

对应 Cursor 的 `conversationState.turns[]`：存的是回合入口，不是一条 AI 回复。

### 5.5 `model_steps`

该回合发给模型的内容。

| 列 | 类型 | 说明 |
|---|---|---|
| `step_id` | TEXT PK |  |
| `turn_id` | TEXT | 指向 `model_turns` |
| `seq` | INTEGER | 回合内顺序 |
| `kind` | TEXT | `user` / `assistant` / `thinking` / `tool_call` / `tool_result` |
| `content` | TEXT |  |
| `tool_call_id` | TEXT NULL |  |
| `tool_name` | TEXT NULL |  |

`thinking` 列先建着。当前循环没有 Thought 步，写入时可以不产生这类行。

### 5.6 `summaries`

压缩结果，本次不写入。

| 列 | 类型 | 说明 |
|---|---|---|
| `summary_id` | TEXT PK |  |
| `replaced_from_seq` | INTEGER | 被摘要掉的起始 `model_turns.seq` |
| `replaced_to_seq` | INTEGER | 结束 seq |
| `body` | TEXT | 摘要正文 |
| `created_at` | INTEGER |  |

### 5.7 `staged`

| 列 | 类型 | 说明 |
|---|---|---|
| `staged_id` | TEXT PK | 如 `F1` |
| `path` | TEXT |  |
| `title` | TEXT |  |
| `kind` | TEXT NULL |  |

语义与现有 `StagedEntry` 相同。✅ 已验证（`types.rs`）

---

## 6. 运行时读写

内存现场不变：只持有当前 `session_id`，不另存对话正文。

| 动作 | 读 / 写 |
|---|---|
| 新建会话 | 建 `{session_id}.sqlite`（空表 + `meta` 一行）；catalog 插入一行 |
| 列表 | 只读 catalog |
| 选中 / 打开 | 打开对应会话库 |
| 界面回放 | `SELECT` `messages`，按 `seq` 排序 |
| 发给模型 | `summaries`（现为空）+ 仍在 `model_turns` 中的 `model_steps` |
| 用户发送 | `messages` 追加 `user`；新建一个 `model_turns`；`model_steps` 追加 `kind=user` |
| 助手可见回复 | `messages` 追加 `assistant`；`model_steps` 追加 `kind=assistant` |
| 工具调用 / 结果 | 只写 `model_steps`（`tool_call` / `tool_result`） |
| Stage | 只写 `staged` |
| 删除会话 | 删 `{session_id}.sqlite`（含 wal/shm）；catalog 删对应行 |
| 组请求截断 | 仍在内存做，上限仍是 2000 / 200，不写回库 |

`save_session` / `load_session` / `append_turn` / `list_session_summaries` / `load_turns_value` / `load_staged_value` 的调用方保持，内部改为 sqlite。运行时路径不再打开 `.json`。

标题规则沿用：第一条用户消息前 48 字；没有用户消息时用时间文案。✅ 已验证（`store.rs`：`session_list_title`）写入 `meta.title` 和 catalog。

---

## 7. 压缩预留（不实现）

日后压缩只做：

1. 从 `model_turns` 去掉早期回合。
2. 向 `summaries` 插入一行，覆盖被去掉的 `seq` 区间。
3. `messages` 不改。
4. 被拿掉的 `model_steps` 行保留，只是不再被 `model_turns` 引用。

发给模型变为：`summaries.body` + 仍在 `model_turns` 里的 steps。  
压缩触发条件和保留多少尾部回合本次不定。

---

## 8. 本机迁移脚本

不在启动、打开会话、列表时迁 JSON。

脚本单独执行，不进 Host 热路径。行为：

1. 扫描 `{cache_dir}/agent/sessions/*.json`。
2. 对每个文件：建 `{session_id}.sqlite`。
3. 按现有 `turns` 顺序：遇到 `user` 开一个 `model_turns`；该 `user` 和随后的 `assistant` / `tool` 写入这个 turn 的 `model_steps`。
4. `user` / `assistant` 且 `content` 非空的同时写入 `messages`。
5. `staged` 原样写入。
6. 更新 catalog。
7. 成功后再处理下一个文件。失败停在该文件，不删源 JSON。

源 JSON 是否删除由执行脚本的人决定，不写进运行时。

---

## 9. 不搬的 Cursor 细节

Cursor 的 `state.vscdb` 是 SQLite，但对话正文在 `cursorDiskKV(key, value)` 里。那是按 key 取 blob，不是按列存消息。

不搬：

- `conversationState` 整包 Protobuf
- `agentKv:blob` 加密
- 所有 chat 共用一个库
- 把文件状态、Git、子代理塞进会话状态

参考的是分层：一场会话、界面原文、模型回合入口、回合内步骤、压缩后改入口不改界面原文。

---

## 10. 实施顺序

1. 会话库 schema 与打开 / 关闭连接（测试沙箱）。
2. 替换 `store.rs` 的创建、保存、加载、删除、列表、turns/staged 读取。
3. 保持 `run.rs` 的追加语义，映射到 `messages` + `model_turns` + `model_steps`。
4. catalog 与会话 `meta` 同步。
5. 单测改到 sqlite 断言；禁止写生产 cache。
6. 本机迁移脚本另做，不阻塞运行时合并。

未授权前不改代码。
