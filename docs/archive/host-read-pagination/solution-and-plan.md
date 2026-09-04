# Host `read` 分页返回技术方案与实施计划

**状态：** Draft  
**关联待办：** `task_6c753240063e`

## 问题与目标

✅ Verified（`src-tauri/src/agent/tools/fs.rs`）：Host `read` 未传 `limit` 时默认最多返回 2,000 行；显式传入的 `limit` 当前没有最大值封顶。

✅ Verified（`src-tauri/src/agent/tools/fs.rs`、`src-tauri/src/agent/turn/history.rs`）：`read` 当前把行号文本作为字符串返回，随后原样放进下一轮的 LLM `tool` 消息。

✅ Verified（本机 `assistant-diagnostic.jsonl` 与最新 Chat session）：一次读取 759 行、58,557 字节的文档后，后续 GLM 请求在 60 秒超时。

⚠️ Proposed：将未传 `limit` 的默认值改为 50，并让成功的 `read` 结果携带分页所需的 `offset`、`limit`、`remaining_lines`，使 Agent 能自行决定是否继续读取。

## 方案

### 输入契约

⚠️ Proposed：`read` 的输入字段不变。

```json
{
  "path": "/absolute/path/to/file.md",
  "offset": 1,
  "limit": 50
}
```

⚠️ Proposed：`path` 必填且仍须通过现有 read fence；`offset` 为可选的 1-based 起始行，缺省为 1；`limit` 为可选的最大返回行数，缺省为 50。

⚠️ Proposed：保持现有归一化语义：`offset < 1` 与 `limit < 1` 均按 1 处理。

### 成功返回契约

⚠️ Proposed：成功时，`ToolResult.content` 改为序列化后的 JSON 字符串；除原有 `content` 外，只新增 `offset`、`limit`、`remaining_lines`。

```json
{
  "offset": 1,
  "limit": 50,
  "remaining_lines": 709,
  "content": "1:# 文档标题\n2:\n3:第一段内容\n..."
}
```

⚠️ Proposed：`offset` 是本次实际使用的起始行；`limit` 是本次实际使用的读取上限；`remaining_lines` 是本次最后一个已返回行之后尚未返回的行数；`content` 保持现有 `行号:内容` 格式。

⚠️ Proposed：`remaining_lines = 0` 表示已到达文件末尾。后续读取的起始行由 `content` 中最后一个行号加 1 得出，不增加 `has_more`、`next_offset`、`total_lines`、`returned_lines` 或 `path`。

⚠️ Proposed：请求的 `offset` 已在文件末尾时，仍返回上述 JSON 形状，`content` 为空字符串且 `remaining_lines = 0`；read fence 与文件不存在等失败路径继续返回现有错误文本。

## 实施计划

1. ⚠️ Proposed：在 `src-tauri/src/agent/tools/fs.rs` 将 `READ_DEFAULT_LIMIT` 从 2,000 调整为 50。
2. ⚠️ Proposed：在同一模块计算本次切片结束位置后的剩余行数，并将成功结果序列化为上述 JSON 字符串。
3. ⚠️ Proposed：更新 `src-tauri/src/agent/tools/host.rs` 中 `read` 的工具描述，使模型知道默认 50 行及返回字段。
4. ⚠️ Proposed：在 `src-tauri/src/unit-tests/agent/tools_host_tests.rs` 增加或调整单元测试，覆盖默认 limit、显式 offset/limit、`remaining_lines = 0` 和文件末尾读取。

## 验收

1. ⚠️ Proposed：未传 `limit` 时，51 行文件的首次读取只返回前 50 行，并返回 `offset: 1`、`limit: 50`、`remaining_lines: 1`。
2. ⚠️ Proposed：传入 `offset: 51, limit: 50` 时，返回第 51 行、`remaining_lines: 0`，且不再有额外元数据字段。
3. ⚠️ Proposed：成功结果仍由 Host 作为 `tool` 消息内容传递给 LLM，内容变为可解析的 JSON 字符串。
4. ⚠️ Proposed：路径围栏拒绝、缺失文件等既有错误语义不变。

## 排除

⚠️ Proposed：本轮不修改上游 LLM 的 60 秒 timeout、不引入 token 预算、不改流式输出、不改 `grep`、不改 Chat 历史截断策略。
