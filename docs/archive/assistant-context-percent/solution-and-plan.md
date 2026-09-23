# 助手上下文占比显示

**状态：** 已实施
**日期：** 2026-09-23
**范围：** Home 当前会话里，一次用户发送的最后一次模型请求占上下文窗口的百分比。不实现压缩。

---

## 1. 问题与目标

现在发给模型的历史只按条数截断：最多 2000 条消息、200 个用户回合（`src-tauri/src/agent/turn/history.rs`、`types.rs`）。一条工具结果就可以占掉窗口的大部分，界面上看不出来。

目标：

- 按当前 Host 模型准备上下文窗口。`glm-5.2` 为 1,048,576。
- 计算这一次用户发送里，最后一次模型请求的 prompt 占了多少。最终回复不计入。
- Home 输入区只显示一个整数百分比。

压缩触发、摘要内容和保留哪些回合不在本方案里。

---

## 2. 锁定结论

| # | 结论 |
|---|---|
| 1 | 占比 = 一次用户发送里最后一次模型请求的 prompt token 数 / 该模型的上下文窗口。最终回复不在分子里。 |
| 2 | 窗口写在代码表里，按模型名查找。首行是 `glm-5.2 = 1048576`。不做成设置项。 |
| 3 | 分子用官方 `tokenizer.json` 和 `chat_template.jinja` 对那一次实际发出的请求文本计数。 |
| 4 | 那一次请求含系统提示、当时的截断历史、当时的工具定义。不含这次请求写出的最终回复，不含输入框草稿，不含输出预留。 |
| 5 | 界面只显示四舍五入后的整数百分比。超过 100 仍显示实际数字，例如 `124%`。数字在这次用户发送结束、界面显示完毕时更新。 |
| 6 | 没有当前会话、未绑定、还没有发出过模型请求、或模型名不在窗口表里时，不显示数字。 |
| 7 | 不新增 invoke。数字附在现有 `get_ai_assistant_binding` 返回值上。 |

窗口数字来自智谱 GLM-5.2 文档（上下文长度 1,048,576）。分词器文件来自 Hugging Face `zai-org/GLM-5.2`：`tokenizer.json`（20,217,442 字节）、`tokenizer_config.json`、`chat_template.jinja`。模板会把 tools、user、assistant、tool 渲染进同一段文本。

---

## 3. 现状

Host 模型名在 `config.toml` 的 `llm` 条目里。当前机器上的 Host 模型是 `glm-5.2`，平台是 `glm`，地址是 `https://open.bigmodel.cn/api/paas/v4`。

组请求在 `src-tauri/src/agent/turn/run.rs`：`build_llm_messages_from_turns` 产出 messages，当前工具目录产出 tools，再交给 `llm::chat_completions_with_timeout`。请求体带 `stream: true`。代码不读取响应里的 `usage`。

仓库没有分词器依赖（`src-tauri/Cargo.toml`）。

Home 当前会话从 `get_ai_assistant_binding` 进入界面。`select_chat_session` 和 `create_chat_session` 返回的就是这份数据（`src-tauri/src/agent/shell/mod.rs`）。前端 `applySessionPayload` 写入 `currentSessionId`、`messages`、`staged`（`frontend/src/home/commands/hub.ts`）。发送结束后 `refreshStagedFromBinding` 会再拉一次 binding，但只更新 `staged`（`frontend/src/home/commands/staged.ts`）。

界面职责按 `docs/architecture/ui-layer-constraints.md`：ui 只绘制，commands 取数并写入 state，state 只保存。

---

## 4. 计算

1. 读当前 Host 模型名。表里没有这个名字时，不产生数字。
2. 一次用户发送内部可以打多次模型。每次实际发出前，用与这次 `run.rs` 相同的 messages 和 tools 渲染并计数，把 token 数写入该会话。后一次覆盖前一次。
3. 用户发送结束、界面显示完毕时，读取留下的那一次 token 数。那就是最后一次模型请求的 prompt。最终回复在这次请求返回之后才写入历史，不在分子里。
4. 百分比 = 四舍五入(token 数 × 100 / 窗口)。重新打开会话时仍读这个已保存的 token 数。

还没有发出过模型请求时，会话里没有这个 token 数，界面不显示。

工具定义用这次请求已经组装好的目录，不为此再请求智谱。

`tokenizer.json` 与模板放进仓库，随应用读取。运行时不下载。

单测使用固定短文本和冻结后的 token 数，不访问网络。已发出 prompt 的 token 数在随后的会话保存之后仍然还在。

---

## 5. 界面

`get_ai_assistant_binding` 增加 `context_percent`。有数字时是整数，没有时字段省略。

Home state 增加 `contextPercent`。`applySessionPayload` 在选择会话、新建会话、绑定恢复时写入。发送结束后的 binding 刷新同时写入这个字段。删除当前会话且没有下一场时清空。

输入区只绘制这个数字，例如 `23%`。没有数字时不占文案。一次用户发送进行中不刷新这个数字，结束并显示完毕后再取最后一次模型请求的结果。

---

## 6. 实施顺序

1. 加入模型窗口表和分词器文件。
2. 实现请求渲染与 token 计数，并用冻结样本做单测。
3. 把 `context_percent` 放进 binding 返回值。
4. Home state、commands、输入区接上这个数字。
5. 用一次真实响应的 `usage.prompt_tokens` 对照本地计数，把差值补进本文件。

## 7. 对照

2026-09-23，对当前机器 Host 模型 `glm-5.2`（`https://open.bigmodel.cn/api/paas/v4/chat/completions`，`stream: true`）读取响应末包的 `usage.prompt_tokens`。本地计数使用 `src-tauri/assets/glm-5.2/tokenizer.json`，渲染与同目录 `chat_template.jinja` 一致。许可是该目录下的 MIT `LICENSE`（Copyright 2026 Zhipu AI）。

| 请求 | 本地 | 上游 `prompt_tokens` | 上游减本地 |
|---|---:|---:|---:|
| 系统 `You are helpful.`，用户 `Hi`，无工具 | 18 | 18 | 0 |
| 用户 `Hi`，一个 `lookup` 工具，`tool_choice` 为 `auto` | 152 | 151 | -1 |

只有系统消息的请求被上游拒绝，返回 `messages` 参数非法，没有进入上表。

界面只显示百分比，不显示这个差值。

## 8. 口径

2026-09-23 把分子从「按已保存历史重算的下一次请求」改成「这一次用户发送里最后一次模型请求的 prompt」。最终回复留到下一次请求。

对照过的其它实现：

- Cursor 课程页写明输入和输出都会进入上下文，并会提示离窗口上限还有多远（https://cursor.com/learn/context）。论坛对用量报告的说明是估算，大约按字符数除以 4，中文尤其不准；Conversation 包含用户消息、回复、思考、工具调用和结果（https://forum.cursor.com/t/abnormal-token-count-at-context-usage-report-conversation/164714）。
- Cline 较新的 `getCurrentContextSize` 取最近一次 assistant 调用的 `inputTokens`，并说明这是该次 prompt 的大小，缓存已含在 input 里不再相加（https://github.com/cline/cline/commit/8c239ab7e4e1cd074c9bed107c5a2c52aaf31aca）。更早的界面把 `tokensIn` 和 `tokensOut` 相加。
- Roo Code 的任务头百分比后来把预留输出也加进分子（https://github.com/RooCodeInc/Roo-Code/pull/11034）。

本方案保持官方分词器，不加输出预留，也不把最终回复加进分子。界面仍在整次用户发送结束时更新一次。
