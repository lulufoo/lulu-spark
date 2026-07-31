# 剥离 Host 对 session master 的泄漏

> Created: 2026-07-31  
> Status: Implemented · 待提交  
> Related todo: [`task_a927d73acb4f`](todo://task_a927d73acb4f) — `Assistant 泄漏：session master问题描述与分析`  
> Related (门闩已修): `task_507a836737a8`  
> Cycle context: `feature-20260724105005-345eefe1`（L1/L2 Binding Contract 已落地；本方案收执行面残留）
>
> ✅ Verified（实现落地，2026-07-31）：`run_loop` / `agent_chat_turn` 以 Binding.tools `ctx` + Binding.prompt 为执行真相；`ensure_ai_assistant_session` 无 master；壳仅 `Unbound`/`Bound`。回归：`cargo test --lib agent::`（80 ok）、`cargo test --lib ai_assistant`（18 ok）、相关 vitest（plan-task-binding / composer-gate / copy-switch / window / p4-smoke）通过。

**代码出处（解读依据）：**

- `frontend/js/plan-task/todos-binding.js`
- `frontend/js/ai-assistant.js`
- `src-tauri/src/commands/ai_assistant.rs`
- `src-tauri/src/services/agent/loop.rs`
- `src-tauri/src/services/agent/tools.rs`
- `src-tauri/src/services/agent/session.rs`
- `src-tauri/src/services/agent/mod.rs`（`PLAN_ASSISTANT_SYSTEM_PROMPT`）

---

## 1. 目标与非目标

### 1.1 目标

一次收成后须同时成立：

1. **Host 契约面**仍只认通用 Binding（`tools` + `prompt` + `callbacks`）与 `unbound`/`bound`；`query_binding` **永不**返回计划编号或标题。✅ 对齐 L1（已有契约约束）。
2. **对话执行面**（`agent_chat_turn` / `run_loop` / 工具派发）只从**当前 Binding**取工具配置、prompt、以及「打哪条计划」的执行语境；**不再**读 `session.bound_master_task_id` / Host runtime `bound_master_task_id`。
3. **壳 UI**默认只显示 `Unbound` / `Bound`；不依赖 `bound_title` 才能理解状态；输入门闩继续只跟 `query_binding`（✅ Verified：`07f5e96` 已落地）。
4. **Todos 消费方**仍能对「当前选中 master」完成 T-lift 读写；换选 master → 替换 Set → 执行语境随之切换。

### 1.2 非目标

- 不改 Binding Contract ops 清单（仍是 Set/Reset/query/execute + 回调）。
- 不把 `master_task_id` 升格为契约正式字段名（禁止 Todos 专用契约主键）。
- 不恢复 `open_ai_assistant(masterTaskId)` 作为 Todos 可执行主路径。
- 不做完整多会话隔离硬化（L1/L2 Out）；本方案只剥 master 泄漏。
- 不在本方案展开第二业务消费者的工具集设计。

---

## 2. 现状双通道（问题基线）

✅ Verified（`task_a927d73acb4f` 正文 + 源码核验）：

| 通道 | 行为 | 评价 |
|------|------|------|
| A · Binding Contract | Todos `set_binding`：tools/prompt/callbacks，无 master | 正确 |
| B · 旧 session | Set 后 `ensure_ai_assistant_session` → `open_ai_assistant_core` 写 session/runtime master+title | 泄漏 |
| 壳门闩 | `query_binding` → `hostBound` | 已修 |
| 壳文案 | 可选 `Bound: {bound_title}` | UI 软依赖 |
| `run_loop` | session master：存在性门闩 + `tools::dispatch` | **工具硬依赖** |
| LLM system | 常量 `PLAN_ASSISTANT_SYSTEM_PROMPT`，不含具体 id | 非 id 注入 |
| Binding.prompt | Set 后 Host 持有，但 `run_loop` 未采用 | 配置面与执行面未合流 |

剥离 = **废除通道 B 作为执行/展示依赖**，让通道 A 成为执行唯一配置来源（与 L1「bound 期间当前 Binding 是唯一配置来源」对齐）。

---

## 3. 目标架构

### 3.1 责任切分

| 角色 | 持有 | 不持有 |
|------|------|--------|
| Todos | 选中 master；装配 Binding（含执行语境）；Set/Reset；Present | 不写 Host runtime master |
| Host Binding | 当前 Binding 槽；state；generation | 业务 id 主键；壳装饰标题 |
| Host turn | 按当前 Binding 跑 LLM + 工具 | `session.bound_master_task_id` |
| Session | `session_id` + turns（消息历史） | master / title（迁移后删除或忽略） |
| 壳 | Present；composer←query；状态 Unbound/Bound | 不把 title/id 当门闩 |

### 3.2 「计划语境」放哪（选定路径）

**选定：A1 不透明 tools 句柄携带执行语境（消费方编入，Host 不解读业务 schema）。**

- Todos 装配示例（形状示意，非契约 JSON 标准）：

```json
{
  "tools": [
    { "name": "get_plan", "ctx": { "master_task_id": "task_…" } },
    { "name": "add_sub_task", "ctx": { "master_task_id": "task_…" } }
  ],
  "prompt": "<Todos 自备 prompt 字符串>",
  "callbacks": {}
}
```

- Host Set 校验：仍只做「三块存在 + tools/prompt 可应用」；**不**因缺少 `ctx` 而拼装；**不**把 `ctx` 升格为契约主键。
- 执行时：从**当前 Binding.tools**解析出本轮可用的 tool 名列表与派发语境；`dispatch` 的 master 来自句柄 `ctx`，不再来自 session。
- `query_binding` 继续只返回 state/generation，不回传 tools/ctx。

**拒绝：**

- 在 Binding 顶层加 `master_task_id` 字段（契约业务化）。
- 长期保留 `ensure_ai_assistant_session(masterId)` 作为正式供给。

⚠️ Inferred：同一 Binding 内各 tool 的 `ctx.master_task_id` 应一致（Todos 装配时保证）；Host 可不强制校验一致性，不一致时以「逐 tool 句柄为准」或执行前探针失败——实现阶段二选一写进测试即可。

### 3.3 Prompt 合流

- `run_loop` system 消息改为：**当前 Binding.prompt**（字符串或约定可应用形态），不再硬编码仅 `PLAN_ASSISTANT_SYSTEM_PROMPT`。
- Todos 继续自备与今日 T-lift 等价的 prompt（可从现有 `TODOS_PLAN_ASSISTANT_PROMPT` / 常量迁移）。
- 常量 `PLAN_ASSISTANT_SYSTEM_PROMPT`：降级为 Todos 侧默认文案来源或测试夹具，**不再**作为 Host 执行默认值（避免 Host 替业务填 prompt——对齐 L1）。

### 3.4 可执行门闩（替换「session 有 master」）

进 LLM / 工具前：

1. `query`/`current_binding` 必须为 **bound**；否则 `rejected_unbound`（或等价业务/错误终态），与 Binding Contract execute 门闩一致。
2. （可选探针）从 Binding.tools 提取执行语境；若 Todos 工具需要 master 且句柄缺失 → 本轮失败（可读错误），**不是** Host 去猜/补 id。

不再调用 `todo_task::get_by_id(session.bound_master_task_id)` 作为「有没有绑定」的定义。

### 3.5 Session 与 ensure

| 项 | 迁移后 |
|----|--------|
| `ensure_ai_assistant_session` | **删除**或改为无 master 的 `ensure_chat_session()`（仅 session_id + 空 turns）；Todos Set **不再**传 master |
| `Session.bound_master_task_id` / `bound_title` | 停止写入；读路径删除；磁盘旧字段反序列化可 ignore |
| Host runtime `bound_master_task_id` / `bound_title` | 停止作为执行真相；`get_ai_assistant_binding` 不再作为「业务绑定」API（可只返回 session_id/busy/window，或逐步废弃壳对 title 的消费） |
| `open_ai_assistant` | 保留与否另议；若保留须标注 legacy，**不得**被 Todos 主路径调用（N1 已禁） |

### 3.6 壳 UI

- 状态行：**仅** `Unbound` / `Bound`（契约态）。
- 去掉对 `bound_title` / `Bound_master_task_id` 的展示依赖与 `agent_chat_turn` 回写改标题。
- 若产品仍要「当前计划名」：由 **Todos 页**自己显示；或将来单独、非契约的「壳装饰事件」（消费方 emit，Host 不持久化业务 id）——**本方案默认不做**，避免再开泄漏口。

---

## 4. 分阶段落地

### Phase 0 — 冻结口径

- [x] 本方案 Accept；todo `task_a927d73acb4f` 链到本文路径。
- [x] 确认：tools 句柄 `ctx` 为执行语境载体；不改契约顶层字段。

### Phase 1 — 执行面改读 Binding（可双读过渡）

**触达：** `loop.rs`、`tools.rs`、相关 unit tests。

- [x] `run_loop`：bound 门闩改为「当前 Binding 存在」。
- [x] system prompt ← `current_binding.prompt`。
- [x] tool 白名单/定义 ← Binding.tools 名列表（与 openai_tool_definitions 对齐策略：仅允许 Binding 声明且 Host 认识的名字）。
- [x] `dispatch`：master ← tool 句柄 ctx；**无** session master fallback（直接落地，未做双读过渡）。
- [x] 新增/改单测：无 session master、仅 Set Binding(ctx) 时 get_plan/add_sub 成功；Reset 后失败。

### Phase 2 — Todos 装配与去掉 ensure

**触达：** `todos-binding.js`、`todos-lifecycle` 测试、ACL（若删命令）。

- [x] `assembleTodosBindingBody(masterTaskId)`：tools 带 `ctx.master_task_id`。
- [x] Set 后改为无参 `ensure_ai_assistant_session`（仅 session；不写 master）——保留命令窄化，未整段删除。
- [x] 换选替换 Set 时 ctx 随新 master 更新（已有替换 Set 生命周期）。

### Phase 3 — 壳与 Host API 去业务字段

**触达：** `ai-assistant.js`、copy-switch 测试、`get_ai_assistant_binding`、commands。

- [x] 壳只显示 Unbound/Bound；移除 title 装饰路径。
- [x] `agent_chat_turn` 请求体不再传 `masterTaskId`（或忽略）。
- [x] 窄化 `ensure_ai_assistant_session`（无 master）；ACL 仍允许同名命令。
- [x] `get_ai_assistant_binding` 停止返回 bound_master/title。

### Phase 4 — 清理与回归

- [x] Session 停止**写入** master/title（`create_session(None, None)`）；旧字段反序列化仍可 ignore（结构字段未删，非执行真相）。
- [x] `run_loop` 不再默认 `PLAN_ASSISTANT_SYSTEM_PROMPT`（常量仅作 Todos/测试夹具来源）。
- [x] P1/N2/parity：Set+Present 可对话仍绿；Host 侧 get/open/ensure 不写 master。
- [x] Checklist：壳门闩仍跟 `query_binding`（延续 `task_507a836737a8`）。

---

## 5. 验收清单（剥离完成定义）

| # | 验收项 | 通过信号 |
|---|--------|----------|
| V1 | Set 合法 Binding（含 ctx）后 turn 成功 | 不调用 ensure；session 无 master |
| V2 | Reset 后 turn/工具失败 | 与 unbound 一致 |
| V3 | `query_binding` | 无 master/title 字段 |
| V4 | 壳 | 仅 Unbound/Bound；无 `Bound: {todo标题}` 依赖 |
| V5 | Host runtime | turn 路径不读/不写 `bound_master_task_id` |
| V6 | Todos 换选 | 新 Binding ctx 生效；旧 master 工具不可再打到 |
| V7 | N1 | 仍禁止 Todos 主路径 `open_ai_assistant(masterId)` |
| V8 | 契约形状 | Binding 顶层仍无 `master_task_id` 键 |

---

## 6. 风险与缓解

| 风险 | 缓解 |
|------|------|
| 不透明 ctx 被误当成契约标准 JSON | 文档钉死：A1 句柄；Set 校验不解读 ctx；第二消费者自备自己的句柄形状 |
| Phase 1 双读导致「看起来剥了、实际仍写 session」 | Phase 2 删除 ensure 前加断言测试：Set 后 `get_ai_assistant_binding.bound_master_task_id` 为空 |
| Binding.prompt 未 Set 时 Host 空跑 | 不可：无 Binding 不得 turn；禁止 Host 默认填计划助手 prompt |
| 旧 session 文件含 master | 反序列化 ignore；不作为执行输入 |

---

## 7. 建议提交切片（实现时）

1. `test(agent): Binding ctx drives tools without session master`  
2. `feat(agent): run_loop uses current Binding prompt/tools/ctx`  
3. `feat(todos): assemble tools ctx; drop ensure_ai_assistant_session`  
4. `fix(assistant): shell Unbound/Bound only; drop title/master UX`  
5. `chore(agent): remove ensure command / ACL / dead session fields`

---

## 8. 文档与 todo 关系

| 产物 | 角色 |
|------|------|
| `task_a927d73acb4f` | 问题描述与分析（SoT 问题侧） |
| **本文** `docs/archive/assistant/strip-host-session-master-leak.md` | 完整剥离方案（SoT 方案侧） |
| `task_507a836737a8` | 壳门闩问题（已部分修复；本方案不重开门闩语义） |

实现启动前：将本文路径写回 `task_a927d73acb4f` 的 Related design 行（人工或后续 MCP 更新正文）。
