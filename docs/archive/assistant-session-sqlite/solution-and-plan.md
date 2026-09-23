# 助手会话存储升级：按会话拆分的 SQLite

**状态：** 已实施
**日期：** 2026-09-23
**范围：** Workbench macOS 会话落盘与增量写入。不实现上下文压缩、旧 JSON 迁移、UI 或 IPC 改动；格式预留压缩。
**图：** https://luluboard.app/#b:b_389a535a

---

## 1. 问题与目标

改造前一场会话一个 JSON 文件，整份 `turns` 每次重写；Home 列表扫描目录并解析文件。界面回放和发给模型读同一份 `turns`。后续要做上下文压缩时，界面原文和模型上下文会分叉，这份结构撑不住。

本次落地结果：

- 会话改用 SQLite，一场会话一个库文件。
- 界面原文和模型历史分开存。
- 运行时不再读旧 JSON。
- 每次保存先比较内存与库中仍活动的步骤，只追加新增尾部；取消或截断时只删除活动步骤的尾部。
- 压缩功能本次不做；表先建好。

---

## 2. 锁定结论

| # | 结论 |
|---|---|
| 1 | 本次只升级存储并预留压缩表，不实现压缩任务。 |
| 2 | 运行时不兼容 `{session_id}.json`，旧数据迁移不在本次范围。 |
| 3 | Home 列表读 `sessions-catalog.sqlite`，创建、更新、删除时同步。 |
| 4 | 压缩后早期 `model_steps` 行保留，只从 `model_turns` 去掉引用。 |

已定、不再讨论：

- 后缀是 `.sqlite`，不是 `.vscdb`。
- 一场会话一个文件，不用 Cursor 那种整机 KV。
- 不搬 `conversationState` Protobuf、加密 blob、无列表 `cursorDiskKV`。

---

## 3. 已落地的实现

会话目录为 `{cache_dir}/agent/sessions/{session_id}.sqlite`，目录上一级的 `{cache_dir}/agent/sessions-catalog.sqlite` 保存 Home 列表。✅ 已验证（`src-tauri/src/agent/session/store.rs`、`catalog.rs`）

`session_id` 仍形如 `workbench_chat_{随机 id}`，并拒绝 `/`、`\`、`..`。测试仍必须走 `TestSandbox`，不能读写生产 cache。✅ 已验证（`store.rs`、`schema.rs`）

`AIAssistantSession` 继续只保留当前会话现场；`append_turn` 沿用“加载、内存追加、保存”的调用面。✅ 已验证（`store.rs`、`live.rs`）

`save_session` 会读取仍由 `model_turns` 引用的 `model_steps`，计算与内存 `turns` 的公共前缀：相同前缀不写；新增尾部只插入新增行；内存删掉尾部时只删除对应活动步骤、失去步骤的回合，以及对应的界面消息尾部。它不再清空会话表。✅ 已验证（`session_db.rs`、`turn_store.rs`）

Home 列表只读 catalog，按 `updated_at DESC, rowid DESC` 取 20 条；标题沿用第一条用户消息前 48 字。✅ 已验证（`catalog.rs`、`store.rs`）

界面回放只读 `messages`；模型历史从 `model_turns` 与 `model_steps` 的连接结果重建。工具结果不走界面 IPC。✅ 已验证（`session_db.rs`、`store.rs`）

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
| 发给模型 | 通过 `model_turns` 连接仍活动的 `model_steps` 重建历史；`summaries` 现为空且不参与读取 |
| 新增 user / assistant / tool | 调用方追加内存 turn 后保存；公共前缀以后的步骤才插入。user 新建 `model_turns`，assistant / tool 追加到该回合 |
| 助手可见回复 | 同时追加 `messages` 与 `model_steps` |
| 工具调用 / 结果 | 只追加 `model_steps`（`tool_call` / `tool_result`） |
| Stage | 对当前项 upsert，并删除内存中已不存在的项 |
| 删除会话 | 删 `{session_id}.sqlite`（含 wal/shm）；catalog 删对应行 |
| 组请求截断 | 仍在内存做，上限仍是 2000 / 200，不写回库 |

`save_session` / `load_session` / `append_turn` / `list_session_summaries` / `load_turns_value` / `load_staged_value` 的调用方保持不变，内部改为 SQLite 增量同步。运行时路径不再打开 `.json`。

标题规则沿用：第一条用户消息前 48 字；没有用户消息时用时间文案。✅ 已验证（`store.rs`：`session_list_title`）写入 `meta.title` 和 catalog。

---

## 7. 压缩预留

已预留的压缩数据形态：

1. 从 `model_turns` 去掉早期回合。
2. 向 `summaries` 插入一行，覆盖被去掉的 `seq` 区间。
3. `messages` 不改。
4. 被拿掉的 `model_steps` 行保留，只是不再被 `model_turns` 引用。

触发、摘要请求和主请求怎么组见 `docs/archive/assistant-context-compress/solution-and-plan.md`。

---

## 8. 旧 JSON

运行时不读取或迁移 `{session_id}.json`。旧数据迁移脚本不在本次范围；如要补做，必须作为独立任务定义输入、失败恢复和源文件保留策略。

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

## 10. 实施结果

1. 已落地会话库 schema、catalog 和测试沙箱隔离。
2. 已替换 `store.rs` 的创建、保存、加载、删除、列表、turns/staged 读取。
3. 已保持 `run.rs` 调用面的追加语义，并映射到 `messages`、`model_turns` 与 `model_steps`。
4. 已同步 catalog 与会话 `meta`。
5. 已补 SQLite 结构和增量保存回归测试：新增回合不会删除非活动步骤，截断只移除尾部。
6. 旧 JSON 迁移和上下文压缩仍未实现。
