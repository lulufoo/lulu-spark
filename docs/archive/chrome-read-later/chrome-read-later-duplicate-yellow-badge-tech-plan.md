# Chrome 待读插件已添加黄色徽章技术方案

关联 Todo：`task_bb514ad855e4` / `task_bb514ad855e4_sub_04`

[Chrome action.setBadgeBackgroundColor](https://developer.chrome.com/docs/extensions/reference/api/action#method-setBadgeBackgroundColor)

只改 Chrome 插件徽章着色。Host / Gateway / 保存 API 不改。

---

## 现状

✅ Verified（`src-tauri/src/services/read_later/mod.rs`）：新写入 HTTP `201`；24 小时内同 URL 返回 HTTP `409` 与 `code: read_later_recent_duplicate`；缺 url 为 `400`（无 `code`）；落盘失败为 `500`（无 `code`）。

✅ Verified（`src-tauri/src/gateway/mod.rs`）：Gateway 转发时保留 Main Host 状态码；转发失败返回 `502`，正文只有 `error`。

✅ Verified（`extensions/chrome-read-later/lib/readLaterApi.js`）：`201` → `{ ok: true }`；网络失败 → `{ ok: false, status: 0 }`；其余把 HTTP 状态和可选 `code` 原样带回。

✅ Verified（`extensions/chrome-read-later/lib/feedback.js`）：文案已按成功 / 连不上 / 已添加 / 其他失败分开；徽章字除成功 `OK` 外都是 `!`。

✅ Verified（`extensions/chrome-read-later/background.js`）：底色只看 `result.ok`，非成功一律 `#ef4444`。不换工具栏 PNG。

---

## 目标合同

`badgeFeedbackForResult` 同时给出 `badgeText`、`title`、`badgeColor`。`applyFeedback` 只消费该返回值，不再用 `result.ok` 二分着色。

| 结果 | 判定 | 徽章字 | 底色 | 悬停 |
|---|---|---|---|---|
| 成功 | `result.ok` | `OK` | `#22c55e` | 已保存到待读 |
| 已添加 | `code === 'read_later_recent_duplicate'` | `!` | `#eab308` | 24小时内已保存 |
| 连不上 | `status === 0` | `!` | `#ef4444` | 请先启动 Workbench |
| 添加失败 | 其余非成功（含 `400` / `500` / `502`） | `!` | `#ef4444` | `error` 或「保存失败」 |

判定顺序与现网 `feedback.js` 相同：先 `ok`，再 `status === 0`，再 duplicate `code`，最后兜底失败。

---

## 改哪些文件

| 文件 | 改动 |
|---|---|
| `extensions/chrome-read-later/lib/feedback.js` | 返回值增加 `badgeColor` |
| `extensions/chrome-read-later/background.js` | `setBadgeBackgroundColor` 使用返回的 `badgeColor` |
| `tests/extensions/chrome-read-later.test.js` | 断言四条着色；补 `400` / `500` / `502` 为红 |

---

## 明确不改

- Host `create_entry`、Main Host、Gateway
- `readLaterApi.js`、`config.js`、`manifest.json`
- 工具栏默认 PNG
- 徽章 3 秒后清空的现有行为

---

## 验收

1. 24 小时内重复保存：黄底 `!`，悬停「24小时内已保存」。
2. Workbench 无法连接：红底 `!`，悬停「请先启动 Workbench」。
3. 添加失败（`400` / `500` / Gateway `502`）：红底 `!`。
4. 新保存成功：绿底 `OK`。
