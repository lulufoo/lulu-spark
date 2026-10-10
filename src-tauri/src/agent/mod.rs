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

Stage (external files on this chat, not a directory):
When the user gives an absolute path to a file outside Spark's own data, call `stage` with that path, then read and edit it. Files on Stage are readable and writable as exact paths. Unstaging removes both read and write.

Notes and knowledge documents:
Spark's own data is not directly readable. To read or edit a note, call get_note_file with its id and dest_dir set to SESSION_WORKSPACE_DIR, then read the returned copy. After editing the copy, call update_note with source_path set to it. To read a knowledge document, call get_knowledge_file the same way.
When you point the user at a note, write it as [标题](note:<id>) using the note's id. When you point the user at a knowledge document, write it as [标题](knowledge:<id>) using the id from search_document."#;

pub const SESSION_WORKSPACE_DIR_PLACEHOLDER: &str = "{session_workspace_dir}";

/// Fill the Host file-directory placeholder for one turn. Scratch stays a
/// placeholder when this turn has no session scratch.
pub fn fill_host_file_dirs(prompt: &str, scratch: Option<&Path>) -> String {
    match scratch {
        Some(scratch) => prompt.replace(
            SESSION_WORKSPACE_DIR_PLACEHOLDER,
            scratch.to_string_lossy().as_ref(),
        ),
        None => prompt.to_string(),
    }
}

#[cfg(test)]
#[path = "../unit-tests/agent/mod.rs"]
mod tests;
