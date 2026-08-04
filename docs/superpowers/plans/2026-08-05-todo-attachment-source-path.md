# Todo 附件按路径复制 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `add_todo_attachment` / `update_todo_attachment` **删除 `content` 参数**；调用方只传本地源文件绝对路径，由 Host 复制进 todo 附件目录，避免正文进入 MCP tool 参数。

**Architecture:** MCP Adapter 仍只代理 Sidecar HTTP，不读盘。Agent 只传短 JSON（`source_path`）；Host `todo_task` 校验路径白名单后 `fs::copy`，manifest 逻辑保持不变。

**Tech Stack:** Tauri Host（Rust `todo_task` + `local_http`）、`packages/knowledge-mcp`（Zod schema + proxy）、既有 unit/verify 测试、前端 invoke、todo-task SKILL。

## Global Constraints

- Protocol Adapter 只代理 Sidecar HTTP，不得直读 Corpus / 任意 FS（见 `docs/architecture/agent-orientation.md` §5）。
- 附件目标格式仍为 `.md`；落盘名规范化沿用 `normalize_attachment_file_name`（`src-tauri/src/services/todo_task/mod.rs`）。
- 落盘目录与 manifest 合同不变：`plan_tasks_task_dir/<id>/attachments/` + `attachments.json`。
- **硬切：全栈删除 `content`。** MCP / Sidecar / Tauri command / 单测 / SKILL / 前端调用一律不再接受正文。
- 源文件参数为绝对路径 `source_path`；拒绝相对路径、符号链接逃逸、白名单外路径。
- 附件落盘名：默认取 `source_path` 的 basename（须为 `.md`）；**不另设可选改名参数**（合同最小化：只传路径）。

---

## 1. 问题与目标

### 1.1 现状

| 层 | 行为 |
|----|------|
| MCP `add_todo_attachment` | 必填 `content: string`，`proxyPost` → `/api/todo-task-add-attachment`（`packages/knowledge-mcp/index.mjs`） |
| Sidecar | `handle_todo_task_add_attachment_payload` 要求 `content`（`local_http/mod.rs`） |
| 领域 | `todo_task::add_attachment(..., content)` → `fs::write(dest, content)` |
| Tauri | `add_todo_attachment(master_task_id, file_name, content)`（`commands/todo_task.rs`） |

Agent 上传大附件时，正文进入 tool 入参，占用对话 token。

### 1.2 目标行为

```text
SKILL/Agent/UI：本地已有 .md → 只传 source_path（绝对路径）
    → MCP / Sidecar / command（短参数，无正文）
    → Host：白名单校验 → 复制到 attachments/ → 更新 manifest
    → 落盘名 = 源 basename（冲突时沿用现网 numeric suffix）
```

成功响应形状与现网一致：`file_name` / `original_file_name` / `added_at`（及 HTTP `_status` 201）。

### 1.3 非目标（本方案不做）

- 不保留 `content` 过渡期 / 双模式。
- 不改 `get_todo_attachment`（读附件仍返回正文；另案可做「按需读 / 截断」）。
- 不引入远程 URL 拉取、multipart 上传。
- 不让 knowledge-mcp 进程读文件。
- 不改附件存储布局 / manifest schema。

---

## 2. 合同设计

### 2.1 Sidecar HTTP（权威）

`POST /api/todo-task-add-attachment`

| 字段 | 类型 | 规则 |
|------|------|------|
| `master_task_id` | string | 必填 |
| `source_path` | string | **必填**；绝对路径；指向已存在的普通 `.md` 文件 |
| ~~`content`~~ | — | **删除**；出现则 `400`（或 schema 层直接拒识） |
| ~~`file_name`~~ | — | **删除**（add）；落盘名 = 源 basename |

`POST /api/todo-task-update-attachment`

| 字段 | 类型 | 规则 |
|------|------|------|
| `master_task_id` | string | 必填 |
| `file_name` | string | **必填**；要覆盖的**已有附件名**（manifest 内 basename，非源路径） |
| `source_path` | string | **必填**；新内容的源文件绝对路径 |
| ~~`content`~~ | — | **删除** |

错误（建议文案稳定，供 MCP 透传）：

| 条件 | `_status` | `error`（建议） |
|------|-----------|-----------------|
| 缺 `master_task_id` | 400 | `Missing master_task_id` |
| 缺 `source_path` | 400 | `Missing source_path` |
| 请求体仍含 `content` | 400 | `content is not supported; use source_path` |
| update 缺 `file_name` | 400 | `Missing file_name` |
| 路径非绝对 / 非法 | 400 | `Invalid source_path` |
| 不在白名单根下 | 403 | `source_path not allowed` |
| 源不存在或非普通文件 | 404 / 400 | `Source file not found` / `Source is not a regular file` |
| 源非 `.md` | 400 | 沿用 invalid file name 语义 |
| 超过大小上限 | 413 | `Attachment too large` |
| task / 目标附件不存在 | 404 | 同现网 |

### 2.2 MCP tool schema

`add_todo_attachment`：

```text
master_task_id: string  (required)
source_path: string     (required)  // absolute path to .md
```

`update_todo_attachment`：

```text
master_task_id: string  (required)
file_name: string       (required)  // existing attachment basename
source_path: string     (required)
```

- **删除** `content` 字段（Zod / JSON Schema 均不出现）。
- description：Host 按 `source_path` 复制；Adapter 不读盘。

Adapter **不得** `fs.readFile`；继续 `proxyPostHandler`。

### 2.3 Tauri command / 前端（硬切）

- `add_todo_attachment`：`(master_task_id, source_path)` — 删除 `file_name`/`content` 参数。
- `update` / `save` 类覆盖命令：改为 `(master_task_id, file_name, source_path)`，删除 `content`。
- **前端**凡 `invoke` 传正文处同步改为：先落本地临时/缓存 `.md`，再传绝对路径（本方案范围内必须改，否则编译/运行失败）。

---

## 3. Host 路径白名单与复制

### 3.1 解析与校验顺序

1. `trim`；拒绝空。
2. `PathBuf`：必须 `is_absolute()`。
3. `canonicalize`（源必须已存在；不存在 → 404）。
4. 拒绝非 `is_file()`。
5. symlink：以 canonicalize 后路径为准，必须仍在白名单内。
6. 前缀匹配任一 **allow root**。
7. basename 经 `normalize_attachment_file_name`，须为 `.md`。
8. `metadata().len() <= MAX`（建议默认 **2 MiB**）。

### 3.2 Allow roots（v1）

Host 运行时组装（canonicalize；某根不可用则跳过并打日志）：

1. `knowledge_corpus_root()`（`config/paths.rs`）
2. `plan_tasks_dir()` 相关知识根（与 1 去重）
3. `std::env::temp_dir()`
4. 若可得：`{workspace}/.cache`

**不开放：** `$HOME` 整棵、`/`。SKILL/UI：先写到 allow 根内再调 API。

### 3.3 复制与落盘

1. `basename` = 源路径 file_name → `normalize_attachment_file_name`。
2. `resolve_unique_attachment_name` → `dest`（同现网）。
3. `fs::create_dir_all` + **`fs::copy(canonical_source, dest)`**。
4. 写 manifest；失败回滚（同现网）。

update：校验 `file_name` 已在 manifest → `fs::copy` 覆盖该附件文件。

### 3.4 函数形状

```text
todo_task::add_attachment(master_task_id, source_path) -> Value
todo_task::update_attachment(master_task_id, file_name, source_path) -> Value
  // 删除一切 content 参数与 fs::write(content) 入口

fn resolve_allowed_source_file(source_path: &str) -> Result<PathBuf, AttachPathError>
```

---

## 4. 影响面

| 区域 | 文件 | 变更 |
|------|------|------|
| 领域 | `src-tauri/src/services/todo_task/mod.rs` | 删 content API；path 校验 + copy |
| Sidecar | `src-tauri/src/services/local_http/mod.rs` | payload 仅 `source_path` |
| Command | `src-tauri/src/commands/todo_task.rs` | 签名硬切 |
| 前端 | 所有 `add_todo_attachment` / 附件 save invoke | 改路径模式 |
| 单测 | `unit-tests/services/todo_task.rs` 等 | 全量改写 content 用例为 path |
| 单测 | `unit-tests/services/local_http.rs` | 同上 |
| 单测 | `unit-tests/commands/todo_task.rs` | 同上 |
| MCP | `packages/knowledge-mcp/index.mjs` | 删 `content`，加必填 `source_path` |
| Verify | `packages/knowledge-mcp/scripts/verify.mjs` | 合同改为 path-only |
| SKILL | todo-task / dialogue-archive 附件说明 | 只允许 path |

---

## 5. 兼容与迁移

| 策略 | 说明 |
|------|------|
| **无过渡期** | 不保留 `content`；旧调用方一次性改完 |
| 调用方 | 先 `write` 本地 `.md`（allow 根内）→ 再 `source_path` |
| 破坏性 | MCP / HTTP / Tauri / 前端 / 测试同步发版 |

---

## 6. 验收标准

1. add 仅接受 `master_task_id` + `source_path`；传 `content` 失败。
2. allow 根内源文件可复制进 attachments + manifest；工具参数无正文。
3. allow 外 → 403；非 md → 400；超大 → 413。
4. 仓库内无附件写入路径再要求 `content` 参数（grep 清零，测试与前端除外的遗留注释也清掉）。
5. knowledge-mcp 该 tool 无 `fs.readFile`。
6. verify + 相关 Rust 单测全绿。

---

## 7. 实现任务

### Task 1：Host `add_attachment(source_path)` + 删 content

**Files:** `src-tauri/src/services/todo_task/mod.rs`、service 单测

- [x] 单测：path 成功、白名单外、相对路径、非 md、超大、缺 path。
- [x] 实现 `resolve_allowed_source_file` + allow roots + `fs::copy`。
- [x] 删除 `add_attachment(..., content)` / `fs::write(content)` 对外入口。
- [x] 改写所有 service 层附件单测至绿。

### Task 2：update + Sidecar 硬切

**Files:** `todo_task/mod.rs`、`local_http/mod.rs`、local_http 单测

- [x] update 仅 `file_name` + `source_path`。
- [x] payload：无 `content`；add 无 `file_name`。
- [x] 单测覆盖成功与 `content` 被拒（若仍解析到未知字段可忽略或 400——建议对显式 `content` 键返回 400）。

### Task 3：Tauri command + 前端

**Files:** `commands/todo_task.rs`、command 单测、frontend 调用点

- [x] 命令签名硬切。
- [x] 前端：写临时 md → 传绝对路径；去掉正文参数。
- [x] 相关单测 / 手工点一点 UI 加附件。

### Task 4：MCP + verify

**Files:** `packages/knowledge-mcp/index.mjs`、`scripts/verify.mjs`

- [x] schema 删除 `content`；`source_path` 必填。
- [x] verify 按 path-only 重写。
- [x] verify 全绿。

### Task 5：SKILL

- [x] todo-task：附件只允许 `source_path`。
- [ ] dialogue-archive 挂附件：先落盘再 path（调用约定已随 todo-task；skill 正文可另补）。

### Task 6：烟测

- [ ] 真机 MCP add 大 md（仅 path）；list/get 正常；grep 确认无 content 合同残留。

---

## 8. 风险与决策记录

| 风险 | 处理 |
|------|------|
| 破坏性变更面大 | 本方案接受硬切；测试与前端同 PR |
| 路径穿越 | canonicalize + allow roots |
| UI 无「先落盘」习惯 | 前端选文件后写入 temp/cache 再调 command |
| Adapter 误读盘 | 禁止；仅 proxy |

**已锁定决策：**

1. 复制在 Host，不在 MCP Adapter。  
2. **`content` 必须删除**；无双模式、无过渡期。  
3. **add 只传** `master_task_id` + `source_path`（落盘名 = 源文件名）。  
4. update 另需目标附件 `file_name` + 新源 `source_path`。  
5. v1 白名单不含任意家目录。

---

## 9. 参考锚点

- 章程：`docs/architecture/agent-orientation.md`（Adapter = proxy）
- 现网写入：`todo_task::add_attachment`（现为 `fs::write`；将改为 `fs::copy`）
- 现网 HTTP：`/api/todo-task-add-attachment`
- 现网 MCP：`add_todo_attachment` in `packages/knowledge-mcp/index.mjs`
