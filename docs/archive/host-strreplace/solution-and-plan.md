# Host `edit` 升级为 `str_replace` 技术方案与实施计划

官方 Cursor Agent 能力分类：<https://cursor.com/docs/agent/overview>

**状态：** Implemented  
**关联待办：** `task_bcb571153caf_sub_02`（父任务 `task_bcb571153caf`）

## 问题与目标

✅ Verified（`src-tauri/src/agent/tools/host.rs`、`fs.rs` 改前）：Host 工具名是 `edit`，参数是 `old_text` / `new_text`，命中超过一次就报错，没有 `replace_all`。

✅ Verified（本 Cursor 会话 `StrReplace` schema）：独立工具，参数 `path` / `old_string` / `new_string` / `replace_all`（默认 false）。

目标：把 Host `edit` 升级成同一份契约，使「删光全文相同片段」不必整文件 `write`。

## 方案

✅ Verified（落地后 `host.rs`）：工具名改为 `str_replace`，与 `grep` / `read` / `write` 同一套小写+下划线。不保留 `edit` 或 `StrReplace` 别名。契约对齐 Cursor `StrReplace`（`old_string` / `new_string` / `replace_all`），只是名字按 Host 统一。

✅ Verified（落地后 `host.rs` schema）：必填 `path`、`old_string`、`new_string`；可选 `replace_all`，默认 false。

✅ Verified（落地后 `fs.rs`）：

- `replace_all` 缺省或 false：`old_string` 必须恰好一次，否则报错。
- `replace_all: true`：替换全部非重叠相同字面量；0 次仍报 `old_string not found`。
- 不是正则。写围栏不变（scratch 或精确 Stage 文件）。

不改 `write`、不改 `read` 默认 50 行。

## 验收

✅ Verified：单测覆盖默认唯一匹配、重复失败、`replace_all: true` 全量替换、Stage 文件仍可写。
