# FilePopup 通用文件读写与 Todo 附件打开

关联 Todo：`task_ffe34e3bbaee`

[std::path::Path::join](https://doc.rust-lang.org/std/path/struct.Path.html#method.join)（绝对路径会替换前一段）

桌面 FilePopup 改成只对调用方给出的绝对路径做通用读/写；Todo 附件用这张弹框打开，删除 `AttachmentEditor`。高亮仍走现有 `doc-editor` identity。不改 Notes 阅读器自己的 `/api/notes-file`、`/api/save`。

---

## 锁定（对话收敛）

| 项 | 选择 |
|---|---|
| 范围 | 附件打开改走 FilePopup，删除 `AttachmentEditor`。不做子任务正文，不做附件列表/添加/删除的产品改动（list 只加绝对路径字段）。 |
| 弹框 | 独立组件：只收路径 + 可选 title + `identityKey`。打开/保存是标准文件读写，不认业务。 |
| 保存 | 直接写调用方给的路径。附件编辑不再走 `save_todo_attachment`。Home staged 同样。 |
| 高亮 | 调用方传入已拼好的 `identityKey`。`doc-editor` 的 `notesDocKey` / `knowledgeDocKey` / `todosDocKey` 与 Host 缓存不变。 |
| 附件路径 | `list` 每条带绝对路径。 |
| 文件口 | 仍用 `/api/file`，后端改为按绝对路径读写，不再映射 `get_notes_file` / `save_entry`。 |
| 根限制 | 不设白名单；给哪条路径就读/写哪条。 |
| 画画 | 删除 Notes 式相对链接/相对图后处理。只留 `renderDocMarkdown`、通用 mermaid、传入钥匙的高亮。 |
| Home 钥匙 | Stage 记住文档类型；打开时再生成 `identityKey`。 |
| Android | 本方案不改。 |

---

## 现状

✅ Verified（`frontend/src/file-popup/commands/popup.ts`）：`openFilePopup` 只收 `path` + 可选 `title`。读 `GET /api/file`，写 `POST /api/file`。无 `identityKey`，无业务回调。

✅ Verified（`frontend/src/host/readApiInvokeMap.ts`、`writeApiInvokeMap.ts`）：`/api/file` 读映射 `get_notes_file`，写映射 `save_entry`。

✅ Verified（`frontend/src/host/api/workbench.ts`）：Notes 阅读器读 `/api/notes-file`、写 `/api/save`，与 FilePopup 不是同一 URL，但落到同一对 Host 命令。

✅ Verified（`src-tauri/src/services/workbench_read/notes.rs`）：`get_notes_file` 先 `notes.join(layer).join(path)`。`path` 若是绝对路径，结果就是该绝对路径；若落在 `knowledge_root` 下则放行。这是 Knowledge staged 能被 FilePopup 读到的旁路。

✅ Verified（`src-tauri/src/services/entry_write.rs`）：`save_entry` 只允许写 Notes 树。Knowledge staged 保存会被拒。`todo_tasks/` 读/写都会被拒。

✅ Verified（`src-tauri/src/agent/tools/stage.rs`、`session/types.rs`）：`get_note_content` / `get_knowledge_content` 成功后把绝对路径 `register_staged`。`StagedEntry` 只有 `id` / `path` / `title`，没有类型。

✅ Verified（`frontend/src/home/commands/staged.ts`）：生产路径里只有 Home staged 调用 `openFilePopup`。

✅ Verified（`frontend/src/file-popup/ui/paint.ts`）：高亮写死 `notesDocKey(commonPath)`；没有 `/notes/raw|digest/` 则不做高亮。另有 Notes 相对链接/图后处理。

✅ Verified（`frontend/src/doc-editor/identity.ts`、`src-tauri/src/services/doc_highlights.rs`）：钥匙格式 `notes:` / `knowledge:` / `todos:`；缓存 `{cache_dir}/doc-highlights/{md5(key)}.json`。

✅ Verified（`src-tauri/src/services/todo_task/attachments.rs`）：附件文件在 `todo_tasks/tasks/{id}/attachments/{file_name}`。打开走 `read_todo_attachment`；保存先 stage 再 `save_todo_attachment`。list 不返回绝对路径。

✅ Verified（`frontend/src/todo-task/ui/attachments.tsx`、`commands/attachments.ts`）：`AttachmentEditor` 是独立弹框。`page.tsx` `paint()` 会拆下并重挂该节点。

---

## 目标合同

### FilePopup

```text
openFilePopup({ path, title?, identityKey? })
```

- `path`：绝对路径。空则不打开。
- `identityKey`：已拼好的文档钥匙。缺省则不挂高亮。
- 打开：`GET /api/file?path=`（不再传 Notes `layer` 作为业务语义）。
- 保存：`POST /api/file`，body 为 `path` + `content`，直接覆盖该文件。
- 画画：`renderDocMarkdown` + mermaid +（有钥匙时）`applyCachedHighlights`。删除 `commonPathFromAbsPath`、`postProcessLinks`、`fetchNotesAssetAsBlobUrl` 这条 Notes 链。

### `/api/file`

L3 改映射到新的通用读/写（按绝对路径）。`get_notes_file` / `save_entry` 只留在 `/api/notes-file` 与 `/api/save`。

不设根白名单。路径非法、不是文件、IO 失败时返回现有风格的 4xx/5xx。⚠️ Inferred（具体命令名与错误文案实现时对齐现有 Host 错误形状）

### Home staged

`StagedEntry` 增加类型（至少 `notes` | `knowledge`）。Workbench 从 `get_note_content` / `get_knowledge_content` 登记时写入。打开时 Home 用类型 + 路径字段调用 `identity.ts`，把生成的钥匙传给 FilePopup。

裸 `stage` 工具若仍只给路径、没有类型：打开不带钥匙，弹框无高亮。✅ Verified（当前 `register_staged` 不收类型）

### Todo 附件

- `list_attachments` 每条增加该文件的绝对路径。
- 点附件：`openFilePopup({ path, title: file_name, identityKey: todosDocKey(masterId, 'att:' + file_name) })`。
- 删除 `AttachmentEditor` 及其在 `paint()` 里的拆挂、`tests/todo-task/attachment-editor.test.js` 中锁旧弹框的断言。
- 添加 / 删除附件、挑选本地文件：不改产品行为。

---

## 改哪些文件

| 区域 | 文件 | 改动 |
|---|---|---|
| L3 | `frontend/src/host/readApiInvokeMap.ts`、`writeApiInvokeMap.ts` | `/api/file` 改映射通用读/写 |
| L4 / 命令 | 现有 `get_notes_file` / `save_entry` 旁新建按路径读写；挂到 Tauri + ACL | 读/写绝对路径；不走 Notes 根拼接 |
| FilePopup | `commands/popup.ts`、`ui/paint.ts`、`state/store.ts`、`index.ts`、`tests/file-popup/*` | 入参加 `identityKey`；读写去 `layer`；删 Notes 后处理 |
| Stage | `agent/session/types.rs`、`agent/tools/stage.rs`、Home `HubStagedEntry` | 登记类型；打开时生成钥匙 |
| Todo | `attachments.rs` list、`todo-task` 附件 UI/commands/`page.tsx`、attachment-editor 测试 | list 带路径；打开改 FilePopup；删旧弹框 |

---

## 明确不改

- Notes / Knowledge 整页阅读器的打开、保存、高亮
- `identity.ts` 三种钥匙格式与 Host 高亮缓存前缀
- 子任务 `content` 字段与行内 textarea
- 附件添加 / 删除 / 源路径拷入
- MCP 附件工具合同（仍 `source_path`）
- Android Stage 阅读

---

## 验收

1. FilePopup 打开 Notes staged、Knowledge staged、Todo 附件，都能读到正文。Knowledge 在弹框里可以保存（写回该绝对路径）。
2. `/api/file` 不再调用 `get_notes_file` / `save_entry`。Notes 阅读器仍走 `/api/notes-file`、`/api/save`。
3. 打开附件不再出现 `AttachmentEditor`。保存后磁盘上该附件文件已更新，不经过 `save_todo_attachment`。
4. 同一附件在详情高亮与 FilePopup 使用同一把 `todos:{id}:att:{file}`，划线能对上。✅ Verified（现详情已用此钥匙，见 `todo-bind.ts`）
5. Home 打开 Notes / Knowledge staged 时，分别带 `notesDocKey` / `knowledgeDocKey`。无类型的 Stage 条目不挂高亮。
6. FilePopup 不再请求 Notes 资源、不再把相对链接改成 GitHub。
