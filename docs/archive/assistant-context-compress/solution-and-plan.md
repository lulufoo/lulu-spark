# 助手上下文压缩

**状态：** 已实施
**日期：** 2026-09-24
**范围：** Workbench macOS。上一笔快照达到窗口 70% 时，把当前仍活动的模型回合收成一行摘要，再组下一次主请求。不改 Home 气泡，不改 `messages`，不删 `model_steps` 行。
**图：** https://luluboard.app/#b:b_517ac3a0
**存储预留：** `docs/archive/assistant-session-sqlite/solution-and-plan.md`
**占比口径：** `docs/archive/assistant-context-percent/solution-and-plan.md`

---

## 1. 问题与目标

发给模型的历史现在只按条数截断：最多 2000 条消息、200 个用户回合。一条工具结果就可以占掉窗口的大部分。占比已经按「一次用户发送里最后一次模型请求的 prompt」落盘，圆环能看见满了，还没有把早期回合换成摘要。

目标：

- 下一笔用户发送组主请求之前，若上一笔快照已经达到窗口的 70%，先压再组。
- 被压掉的几次发送，主请求里只放摘要，不放原文。
- 界面回放仍读 `messages`，字不变。

---

## 2. 锁定结论

| # | 结论 |
|---|---|
| 1 | 触发看上一笔已存快照。`round_percent(last_prompt_tokens, window) >= 70` 就压。窗口与占比同一张表，`glm-5.2 = 1048576`。 |
| 2 | 压的是当前仍挂在 `model_turns` 上的全部回合，从最小 `seq` 到最大 `seq`。本次用户句还没入列，不在这段里。 |
| 3 | 一行 `summaries` 盖住这一段。`replaced_from_seq` / `replaced_to_seq` 是 `model_turns.seq`，不是 `model_steps.seq`，也不指向某一个 `turn_id`。 |
| 4 | 被盖住的 `model_turns` 行删掉。对应 `model_steps` 行留下，不再被回合引用。`messages` 不改。 |
| 5 | 摘要来自单独一次模型请求。把被盖住的几次发送的步骤打包发出，返回写入 `body`。 |
| 6 | 打摘要这次之前，用同一套渲染和分词器看打包请求会不会达到窗口。达到则先截最长的 `tool_result`，再截其余步骤正文，直到低于窗口。材料只用于这次摘要请求。 |
| 7 | 打摘要这次不写 `last_prompt_tokens` / `last_prompt_breakdown`，不改圆环。 |
| 8 | 主请求：系统提示，然后按 `replaced_from_seq` 升序插入每行 `body`（`role=user`），再拼仍活动的回合。被压过的原文不再进入主请求。 |
| 9 | 工具往返中途不压。摘要失败则不改表，主请求仍用当前活动回合。 |
| 10 | 不新增 invoke。不增加「正在压缩」气泡。条数上限 2000 / 200 仍在内存里，不写回库。 |

已定、不再讨论：

- 被压过的几次，主请求里不把原文和摘要一起放。
- 不另定「最近留几轮原文」。触发时刻还挂在 `model_turns` 上的都是要压的前缀；这次发送和之后的发送不在前缀里。
- 摘要只给主请求用，不是给界面看的。

---

## 3. 现状

会话库已有 `summaries` 表，五列未改：`summary_id`、`replaced_from_seq`、`replaced_to_seq`、`body`、`created_at`。当前没有写入。✅ 已验证（`src-tauri/src/agent/session/schema.rs`）

模型历史由 `model_steps` 连接 `model_turns` 重建。连不上的步骤不进入内存现场。✅ 已验证（`src-tauri/src/agent/session/turn_store.rs`：`load_steps`）

界面回放只读 `messages`。工具调用不进这张表，也不走 `load_turns_value`。✅ 已验证（`store.rs`、`schema.rs`）

一次用户发送在 `run_loop_with_progress`：绑定和系统提示通过后先 `maybe_compress`，再 `push` 用户句并 `persist`。循环里 `build_llm_messages` 先插入 `summaries.body`，每次实际上游调用前 `record_sent_prompt`。✅ 已验证（`src-tauri/src/agent/turn/run.rs`）

`truncate_turns` 仍按 2000 条消息、200 个用户回合丢掉最老一轮，只作用于内存里的 `session.turns`，不写回库。✅ 已验证（`history.rs`、`types.rs`）

占比分子是上一笔用户发送里最后一次主请求的 prompt token。百分比四舍五入，半入。没有快照、未绑定、模型不在窗口表里时没有数字。✅ 已验证（`usage.rs`、`window.rs`）

`save_session` 只同步仍由 `model_turns` 引用的步骤。内存与库中活动步骤对不上时，会删掉对不上的活动步骤，并删掉没有步骤的回合行。压缩不能走这条截尾路径去删 `messages`。✅ 已验证（`turn_store.rs`：`sync` / `delete_suffix`）

---

## 4. 触发

位置：`run_loop_with_progress` 在绑定和系统提示检查通过之后、`session.turns.push` 本次用户句之前。`maybe_compress` 返回后，用当前 `session.turns.len()` 重写 `turns_checkpoint`。后面的取消裁回这个长度。

条件，同时满足：

1. 当前会话有 `last_prompt_tokens`，且 `>= 0`。
2. Host 模型在窗口表里。
3. `round_percent(last_prompt_tokens as u64, window) >= 70`。

缺快照、未绑定、模型没有窗口行：不压，直接入列本次用户句。

一次用户发送最多压一次。循环里的工具往返不再检查。

---

## 5. 切哪些回合

读取当前 `model_turns`，按 `seq` 升序。

- 前缀：全部现有行。`replaced_from_seq = MIN(seq)`，`replaced_to_seq = MAX(seq)`。
- 本次用户句此时还没有 `model_turns` 行，不在前缀里。
- `model_steps.seq` 是回合内步骤号，不进入这两个整数。

没有活动回合时不压。

一场会话可以压多次。下一次触发时，前缀是当时仍活动的回合，再插入一行 `summaries`。旧行留下。

---

## 6. 打摘要这次

材料：前缀里每个 `turn_id` 的 `model_steps`，按 `model_turns.seq`、`model_steps.seq` 排序。`kind` 为 `user` / `assistant` / `tool_call` / `tool_result` 的都带。

请求：

- 与主对话同一个 Host 模型。
- 不带工具定义。
- 系统提示只服务这次摘要：把材料收成一段给后续主请求用的摘要，保留已做的决定、结论和未完成事项。
- 材料放在一条 `role=user` 消息里。

发出前用现有 `render` + 分词器计这次请求的 token。

- 低于窗口：完整材料。
- 达到或超过窗口：先截最长的 `tool_result.content`，再截其余步骤正文，再计；重复直到低于窗口。材料只用于这次摘要请求，不写回 `model_steps`。
- 材料都截空，只剩模板仍达到窗口：这次发不出摘要请求。`glm-5.2` 窗口下不会发生。不因「摘要请求达到窗口」放弃压缩。

成功：返回文本写入 `body`。失败、超时、空返回：同「不压」。

这次调用不走 `record_sent_prompt`。

---

## 7. 落盘

独立事务，不复用 `save_session` 的截尾：

1. `INSERT` 一行 `summaries`。
2. `DELETE FROM model_turns WHERE seq BETWEEN replaced_from_seq AND replaced_to_seq`。
3. 不删 `model_steps`，不改 `messages`，不改 `last_prompt_*`。

事务成功后，内存 `session.turns` 改成当前 `load_turns` 的结果（只剩仍活动的步骤）。重载失败则清空活动回合，避免把已摘掉的原文再 `persist` 回去。然后再 `push` 本次用户句并 `persist`。

被摘掉引用的 `model_steps` 仍留着 `turn_id`。之后的 `JOIN` 读不到它们。`save_session` 也看不到它们，不会当截尾删掉。

---

## 8. 主请求怎么组

`build_llm_messages_from_turns` 改为：

1. 第一条仍是系统提示。
2. `SELECT body FROM summaries ORDER BY replaced_from_seq ASC`，每行一条 `role=user`。
3. 再追加 `truncate_turns(session.turns)`。此时 `session.turns` 只有仍活动的步骤，加上刚入列的本次用户句。

被盖住的原文不再进入第 3 步。2000 / 200 只作用在第 3 步。

---

## 9. 圆环

打摘要这次不改快照。

本次用户发送结束时，最后一次主请求仍走现有 `record_sent_prompt`。圆环和面板读这一笔。Conversation 变短后，下一笔快照会反映压过的请求。

---

## 10. 不做

- 不改 `summaries` 五列。
- 不删 `model_steps` 行，不改 `messages`。
- 不增加 Home 气泡、状态行、手动压缩按钮。
- 不新增 Tauri invoke。
- 不把 Rules / Skills / Subagents / Summarized conversation 补进用量面板。
- 不改 2000 / 200 条数上限，不把这条上限写回库。
- 不预留主请求的输出 token。
- 旧会话没有 `last_prompt_tokens` 的，等下一次主请求写出快照后再判断。

---

## 11. 实施顺序

1. 用上一笔快照和 `round_percent` 判断是否该压；单测 69 / 70。
2. 打包前缀步骤；超窗口时截 `tool_result`；单测「刚好低于窗口不截」和「超了从最长结果截」。
3. 事务写入 `summaries` 并删除对应 `model_turns`；断言 `messages`、`model_steps` 行数、`last_prompt_*` 不变。
4. 组主请求时插入 `body`；断言被盖住的原文不在 messages 里。
5. 挂进 `run_loop_with_progress`：用户句入列之前。摘要失败则跳过。
6. 摘要调用不调用 `record_sent_prompt`。

---

## 12. 验证

- 上一笔快照 69%：不写 `summaries`，`model_turns` 行数不变。
- 上一笔快照 70%：多一行 `summaries`，前缀 `model_turns` 消失，`messages` 条数和正文不变，前缀 `model_steps` 仍在库里。
- 界面 `load_turns_value` 在压前压后相同。
- 主请求 messages：系统提示、各行 `body`、仍活动回合和本次用户句；没有被盖住回合的原文。
- 摘要请求达到窗口：`tool_result` 被截，用户句和助手回复仍在材料里。
- 摘要失败：表和内存与压前一致，主请求仍带原文。
- 打摘要这次之后、主请求发出之前，圆环仍是上一笔快照。
