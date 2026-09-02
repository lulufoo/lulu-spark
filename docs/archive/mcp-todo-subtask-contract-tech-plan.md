# MCP Todo 子任务合同改造技术方案

关联 Todo：`task_c2bb365298b2`

[JSON Schema — additionalProperties](https://json-schema.org/understanding-json-schema/reference/object#additionalproperties)

只改 L1 MCP 入参。L4 / Tauri / 前端不改。旧数据不迁。

---

## 现状

✅ Verified（`src-tauri/src/mcp_host/catalog/groups/todo/create_todo_task.rs`）：`create_todo_task` schema 含可选 `sub_titles`；`invoke` 解析后传给 `create_master_with_category`，可在创建时建出只有标题的子任务。

✅ Verified（`src-tauri/src/mcp_host/catalog/groups/todo/add_todo_sub.rs`）：`content` 不在 `required` 里；缺省时 `arg_str` 得到 `None`，再交给 L4。

✅ Verified（`src-tauri/src/mcp_host/catalog/groups/todo/update_todo_sub.rs`）：`content` 可选；键存在时取字符串（含空串）传给 L4。

✅ Verified（`src-tauri/src/services/todo_task/subs.rs`）：L4 `add_sub` 把空 `content` 存成 `None`；`update_sub_title` 收到空串会清掉正文。桌面走这条，本次不动。

✅ Verified（`src-tauri/src/mcp_host/catalog/route_util.rs`）：`object_schema` 带 `additionalProperties: false`。这是工具说明书。Host `invoke` 不按 schema 拒多余字段。

✅ Verified（`tests/todo-task/mcp-sub-content.test.js`）：现网测试把 MCP `content` 写成可选，并点名 L4/Tauri「可清空」语义。

---

## 目标合同

闸门只拦 MCP。L4 函数签名与行为保持原样；MCP 在调用 L4 之前截住。

| 工具 | 目标 |
|---|---|
| `create_todo_task` | schema 去掉 `sub_titles`。描述改为只建主任务。`invoke` 固定传 `None`，不读 `sub_titles`。仍传来则忽略，树为空。 |
| `add_todo_sub` | `content` 列入 `required`。缺、空、空白 → `400 Missing content`。通过后把 `trim` 后的正文交给 L4。 |
| `update_todo_sub` | `content` 仍不在 `required`。不传 = 只改标题（`None`）。传了则 `trim` 后不能空，否则 `400 Missing content`。 |

错误文案对齐现有 `missing_field`。✅ Verified（`route_util.rs`）：缺字段返回 `Missing {name}`。

---

## 改哪些文件

| 文件 | 改动 |
|---|---|
| `src-tauri/src/mcp_host/catalog/groups/todo/create_todo_task.rs` | 删 `parse_sub_titles`；始终 `create_master_with_category(title, None, todo_md, category_id)`；schema / 描述去掉初始子任务 |
| `src-tauri/src/mcp_host/catalog/groups/todo/add_todo_sub.rs` | schema `required` 加 `content`；`invoke` 校验非空后再 `add_sub` |
| `src-tauri/src/mcp_host/catalog/groups/todo/update_todo_sub.rs` | 描述改为「改正文则不能空」；`invoke` 按上表处理 `content` |
| `tests/todo-task/mcp-surface.test.js` | 断言创建路径不再解析 / 下传 `sub_titles` |
| `tests/todo-task/mcp-sub-content.test.js` | 改向：MCP `add` 必填 `content`；`update` 传空被拒。不再要求 MCP 保持「可清空」 |
| `src-tauri/src/unit-tests/mcp_host.rs` | `create_todo_task` schema 不含 `sub_titles`；`add_todo_sub` 的 `required` 含 `content` |

需要补 MCP `invoke` 行为测试（创建带 `sub_titles` 仍空树；`add`/`update` 空正文 400）。落在现有 MCP catalog / Host 单测旁，不改 L4 / Tauri 用例。

---

## 明确不改

- L4 `add_sub` / `update_sub_title` / `create_master_*`
- Tauri 命令与 `src-tauri/src/unit-tests/commands/todo_task.rs`、`unit-tests/services/todo_task/subs.rs` 里「可选 / 可清空」用例
- 前端创建弹窗、详情保存空正文
- Skill
- 历史空正文的读 / 完成 / 废弃

---

## 验收

1. MCP 创建：201，`sub_tasks` 为空；带 `sub_titles` 也是空树。
2. MCP `add_todo_sub`：缺 / 空 / 空白 `content` → 400；合法正文 → 201。
3. MCP `update_todo_sub`：不传 `content` 只改标题；传空 → 400。
4. 桌面创建带初始子任务、详情存空正文，行为与现在相同。
