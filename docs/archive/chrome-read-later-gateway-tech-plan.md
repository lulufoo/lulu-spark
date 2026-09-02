# Chrome 待读插件 Gateway 路由技术方案

✅ Verified（`extensions/chrome-read-later/lib/config.js`、`extensions/chrome-read-later/lib/readLaterApi.js`）：插件向本机 Gateway 的 `https://localhost:7654/read-later` 发送 `POST` 请求。

✅ Verified（`src-tauri/src/gateway/mod.rs`）：Gateway 在 `7654` 提供 HTTPS，并能将命名请求转发到 loopback Main Host。

✅ Verified（`src-tauri/src/gateway/mod.rs`）：`/read-later` 仅接受 loopback 来源的 `POST` 或 `OPTIONS`，并转发为 `127.0.0.1:8765/api/read-later`。
