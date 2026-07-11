---
name: plan-task
description: >-
  Plan 任务工作流：经 Workbench MCP plan tools 创建/查询/变更 plan 任务树。
  Use when: plan-task、plan task、任务计划、MCP plan、create_plan_task、list_plan_tasks
argument-hint: '[任务标题 | 任务 id]'
---

# plan-task — Plan 任务 MCP 工作流

> **路径约定**：Plan 数据由 Workbench `plan_task` service 唯一写入；Agent **禁止**直写 `plan_tasks/` 或绕过 Host。
>
> **HTTP SSOT**：MCP tools 经 `proxyGet`/`proxyPost` 对接 local_http，路由与语义 1:1 对齐。

## 触发后的首要动作

1. 确认 Workbench App 运行且 MCP sidecar 可用（见 [MCP 前置](#mcp-前置)）。
2. 按目标选择下方 MCP tool；**不得**自行拼接未文档化的 HTTP 路径或 FS 操作。

---

## MCP 前置

<HARD-GATE mcp="plan">
Workbench App **必须运行**，且 `knowledge-mcp` sidecar 已 spawn（默认 MCP `http://127.0.0.1:9876/mcp`）。

- **`WORKBENCH_HTTP_URL`**：sidecar 代理目标，默认 `http://127.0.0.1:8765`（local_http）。
- Host 未启动或 HTTP 不可达时，sidecar **不会**启动 plan tools；先启动 Workbench App。
- 所有 plan 读写 **仅**经 MCP → HTTP → `plan_task` service；违反 I-11。

---

## MCP Plan Tools（8）

| MCP tool | HTTP | 说明 |
|----------|------|------|
| `create_plan_task` | `POST /api/plan-task-create` | 创建 master；可选 `sub_titles` |
| `list_plan_tasks` | `GET /api/plan-tasks` | 列出全部 master |
| `get_plan_task` | `GET /api/plan-task?id=` | 按 id 获取 master 树 |
| `delete_plan_task` | `POST /api/plan-task-delete` | 删除 master 及目录 |
| `add_plan_sub` | `POST /api/plan-task-add-sub` | 新增 sub |
| `delete_plan_sub` | `POST /api/plan-task-delete-sub` | 删除 sub（非最后一个） |
| `complete_plan_sub` | `POST /api/plan-task-complete-sub` | 标记 sub 完成 |
| `link_plan_archive` | `POST /api/plan-task-link-archive` | 关联 archive entry id |

### 请求体 / 查询键（snake_case，无 camelCase 转换）

- **create**：`title`（必填）；`sub_titles`（可选 string 数组；省略或空 → `sub_tasks: []`；需 sub 时显式传入标题，或创建后再 `add_plan_sub`）
- **get**：`id`（master task id）
- **delete master**：`master_task_id`
- **add sub**：`master_task_id`, `title`
- **delete/complete sub**：`master_task_id`, `sub_task_id`
- **link archive**：`master_task_id`, `sub_task_id`, `archive_id`

HTTP 4xx/5xx 经 MCP 透传为 tool error（`isError: true`），勿将错误响应当成功解析。

---

## 典型工作流

1. `create_plan_task` — 创建 master（可带显式 sub 标题）
2. `list_plan_tasks` / `get_plan_task` — 确认状态
3. `add_plan_sub` / `complete_plan_sub` — 演进任务树
4. `link_plan_archive` — 将已完成 sub 关联 corpus archive id
5. `delete_plan_sub` / `delete_plan_task` — 清理

---

## 约束

- **I-11**：仅经 MCP 调用上表 tools；不得指导直写 `plan_tasks/` 或直调 Rust service。
- **I-3**：MCP 仅 proxy HTTP，不含 plan 业务逻辑。
- 文档路由须与已合并 local_http 契约一致；超前未发布路由不得写入本 skill。

---

## 参考

- Workbench tech plan：`MCP plan tools` feature（FM-3 HTTP + FM-6 SKILL）
- 归档关联：完成 sub 后可用 [theme-archive](../theme-archive/SKILL.md) 落盘，再 `link_plan_archive`
