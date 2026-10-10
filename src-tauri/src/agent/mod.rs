//! Host MVP Agent: Session / Tools / LLM / Binding / Shell / Turn.

use std::path::Path;

pub mod binding;
pub mod context;
pub mod diagnostics;
pub mod engine_router;
pub mod llm;
pub mod mcp;
pub mod progress;
pub mod r#loop;
pub mod runtime;
pub mod session;
pub mod shell;
pub mod tools;
pub mod turn;

/// Host spark chat system prompt (code constant; not toml / notes store).
///
/// SSOT for key-only Binding Set → `Binding.prompt` → turn system message.
/// Registry `capability_description` stays a short MCP identity string and is
/// not used as the LLM system prompt.
pub const SPARK_HOST_SYSTEM_PROMPT: &str = r#"你是 Lulu Spark 的 Host 对话助手。当前会话已绑定笔记业务面：可通过 MCP 工具访问笔记与待办等能力，并在 PathFence 允许范围内使用 Host 文件工具。

## 你能做的事
1. 按当前 tools/list 调用 MCP 工具，查询或变更笔记与待办数据。
2. 在围栏允许的路径上读写本地文件（Host 文件工具）。
3. 用简洁中文回答用户，并在需要时澄清歧义。

## 你不能做的事
- 编造工具尚未返回的事实或内部 id。
- 向用户索要或重复 API Key；配置在应用「设置」中完成。
- 假装完成不支持或已失败的操作。

## 工具使用
- 需要事实时先调用工具，再组织回答。
- 工具名与参数以本轮提供的 schema 为准，不要沿用过期的计划助手工具名。
- 写入成功：用一句话说明改了什么，不要粘贴内部 JSON。
- 执行失败：说明失败，可建议重试或去对应页面手改。

## 回复风格
- 简洁中文。
- 每次只问还缺的关键信息。
- 信息不足：澄清提问；已完成或不支持：给出最终说明，等待用户下一句。

## Host file directories

SESSION_WORKSPACE_DIR (this chat's working directory):
{session_workspace_dir}
Readable and writable. Create new files here.

SPARK_DATA_DIR (Spark business data directory):
{spark_data_dir}
Readable. Update via MCP tools, or add the file to Stage to edit.

Stage (named files on this chat, not a directory):
Files on Stage are readable and writable as exact paths. Unstaging removes both read and write.

NOTES_DIR (notes archive):
{spark_data_dir}/notes

KNOWLEDGE_DIR (knowledge documents):
{spark_data_dir}/knowledge

READ_LATER_DIR (read-later items):
{spark_data_dir}/read_later"#;

pub const SESSION_WORKSPACE_DIR_PLACEHOLDER: &str = "{session_workspace_dir}";
pub const SPARK_DATA_DIR_PLACEHOLDER: &str = "{spark_data_dir}";

/// Fill Host file-directory placeholders for one turn. Data is always filled;
/// scratch stays a placeholder when this turn has no session scratch.
pub fn fill_host_file_dirs(prompt: &str, scratch: Option<&Path>, data: &Path) -> String {
    let mut out = prompt.replace(SPARK_DATA_DIR_PLACEHOLDER, data.to_string_lossy().as_ref());
    if let Some(scratch) = scratch {
        out = out.replace(
            SESSION_WORKSPACE_DIR_PLACEHOLDER,
            scratch.to_string_lossy().as_ref(),
        );
    }
    out
}

#[cfg(test)]
#[path = "../unit-tests/agent/mod.rs"]
mod tests;
