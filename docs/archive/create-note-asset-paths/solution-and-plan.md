# create_note 附图文件列表

**状态：** Implemented  
**关联待办：** `task_a86ae1cb28ee`

## 问题与目标

✅ Verified（`.cache/ai-assistant-acceptance-surface-overview.md`）：正文用相对路径引用 `./overview-diagram.png`。

✅ Verified（`src-tauri/src/services/notes/document.rs`）：`create_note` 只把 `.md` 写入 `notes/raw/{common_path}`，不拷贝同目录图。

✅ Verified（`frontend/src/notes/ui/viewer/body.tsx` + `src-tauri/src/services/workbench_read/notes.rs`）：阅读器按「md 所在目录 + 相对 href」取图；显示白名单为 png / jpeg / gif / webp，不含 svg。

目标：桌面 `create_note` 可传入附图绝对路径；Host 按原文件名拷到该篇 raw md 同目录，阅读器能显示。

## 方案

收敛锁定（本会话 `/converge`）：

- 可选 `asset_paths: string[]`（绝对路径，走现有 allow-list）。
- 写入白名单：png / jpg / jpeg。gif、webp 写入不收；读接口保持现状（仍不含 svg）。
- 同目录、原 basename；目标已存在 → 409，整次回滚。
- digest 不准相对图：`digest_body` 含相对图 → 400；图只拷 raw。
- 不做 `update_note`、不做手机 `content`。
- 每张 ≤ 4 MiB，每篇最多 8 张。

✅ Verified（`src-tauri/src/services/archive_parse.rs`）：`common_path` 只能是 `{project}/{theme}/{file}.md`，图不进 index，只作为 raw 同目录文件。

响应新增 `asset_paths`（如 `raw/{project}/{theme}/{filename}`）。`extra_paths` 仍只表示译文。

手机 `create_note_content` / `update_note` 若带 `asset_paths` → 400。

## Plan

1. ✅ Verified（`schema.rs` / `create_note.rs`）：桌面 MCP 增加 `asset_paths`；正文有相对图则同一趟必须带；没有则省略。Host 不扫 md。手机 schema 不含该字段。
2. ✅ Verified（`assets.rs` / `document.rs` / `digest.rs`）：校验、拷贝、回滚；digest 相对图 400。
3. ✅ Verified（本会话 `cargo test --lib services::notes`）：58 passed，含 9 条附图用例（拷贝、jpg/jpeg、svg/gif、上限、撞名、digest、content/update 拒收）。
