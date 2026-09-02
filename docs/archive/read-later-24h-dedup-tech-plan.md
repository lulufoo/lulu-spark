# 待读 24 小时去重技术方案

[RFC 9110 — 409 Conflict](https://www.rfc-editor.org/rfc/rfc9110#section-15.5.10)

✅ Verified（`src-tauri/src/services/read_later/mod.rs`）：当前 `create_entry` 在写锁内直接写入条目，`saved_at` 使用 RFC 3339 UTC 时间。

✅ Verified（`src-tauri/src/main_host/read_later.rs`、`src-tauri/src/gateway/mod.rs`）：Chrome 请求最终进入 `create_entry`，Gateway 会保留 Main Host 的 HTTP 状态码。

⚠️ Inferred：在 `create_entry` 的同一写锁内，以修剪后的 URL 完全匹配和 `saved_at >= now - 24h` 查找已有条目；命中时返回 HTTP `409`、稳定错误码和该已有条目，不写入新条目。

⚠️ Inferred：Chrome 插件将该错误码显示为“24 小时内已保存”，其他错误维持现有失败提示。
