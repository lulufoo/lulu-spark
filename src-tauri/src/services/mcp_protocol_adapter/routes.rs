//! Slot → tool table (workbench / mobile). Domain IO stays on Sidecar HTTP.

use serde_json::{json, Value};

use super::types::{HttpMethod, SceneSlotApi, SlotToolTable, ToolDescriptor, ToolRoute};

pub(crate) fn scene_slot_api(slot: &str) -> Option<SceneSlotApi> {
    match slot {
        "workbench" => Some(SceneSlotApi {
            include_notes: true,
            include_todo: true,
        }),
        "cursor_ide" => Some(SceneSlotApi {
            include_notes: true,
            include_todo: true,
        }),
        _ => None,
    }
}

pub(super) fn object_schema(properties: Value, required: &[&str]) -> Value {
    let mut schema = json!({
        "type": "object",
        "properties": properties,
        "additionalProperties": false,
    });
    if !required.is_empty() {
        schema["required"] = json!(required);
    }
    schema
}

pub(super) fn route(
    name: &str,
    description: &str,
    method: HttpMethod,
    api_path: &str,
    input_schema: Value,
    read_only: bool,
    destructive: bool,
) -> ToolRoute {
    ToolRoute {
        name: name.into(),
        description: description.into(),
        method,
        api_path: api_path.into(),
        read_only,
        destructive,
        input_schema,
    }
}

pub(super) fn source_type_schema() -> Value {
    json!({
        "type": "string",
        "enum": ["summary", "article", "theme-line", "dialogue", "transcript", "jot"],
        "description": "Optional source type; defaults to summary."
    })
}

pub(super) fn create_note_properties(include_source_path: bool) -> Value {
    let mut props = json!({
        "title": {
            "type": "string",
            "description": "Required display title. May be Chinese."
        },
        "project": {
            "type": "string",
            "description": "Optional first path segment. Defaults to inbox."
        },
        "theme": {
            "type": "string",
            "description": "Optional second path segment. Defaults to notes."
        },
        "created_at": {
            "type": "string",
            "description": "Optional YYYYMMDDHHMM (UTC+8). Defaults to now."
        },
        "source_type": source_type_schema(),
        "translations": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "lang": { "type": "string" },
                    "content": { "type": "string" },
                    "source_path": { "type": "string" }
                },
                "required": ["lang"],
                "additionalProperties": false
            },
            "description": "Optional translations. Prefer source_path. Host rejects stub or short zh."
        }
    });
    if include_source_path {
        props["source_path"] = json!({
            "type": "string",
            "description": "Absolute path to an allow-listed Markdown file."
        });
    } else {
        props["content"] = json!({
            "type": "string",
            "description": "Markdown note body. Host writes this text; do not send source_path."
        });
        if let Some(items) = props
            .get_mut("translations")
            .and_then(|t| t.get_mut("items"))
            .and_then(|i| i.get_mut("properties"))
            .and_then(|p| p.as_object_mut())
        {
            items.remove("source_path");
        }
    }
    props
}

pub(super) fn notes_tool_routes() -> Vec<ToolRoute> {
    vec![
        route(
            "get_notes_catalog",
            "List the latest archive entry for each top-level topic.",
            HttpMethod::Get,
            "/api/notes-catalog",
            object_schema(
                json!({
                    "mode": {
                        "type": "string",
                        "const": "latest_per_topic",
                        "description": "The only supported catalog mode."
                    }
                }),
                &["mode"],
            ),
            true,
            false,
        ),
        route(
            "get_notes_files",
            "Read archived digest bodies by entry id.",
            HttpMethod::Post,
            "/api/notes-files",
            object_schema(
                json!({
                    "ids": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Archive entry ids to read."
                    }
                }),
                &["ids"],
            ),
            true,
            false,
        ),
        route(
            "create_note",
            "Create a note from an allow-listed Markdown file by absolute source_path. The Host reads the file; never send document text. Path is {project}/{theme}/{created_at}-{6-char}-{source filename}.",
            HttpMethod::Post,
            "/api/create-note",
            object_schema(create_note_properties(true), &["source_path", "title"]),
            false,
            false,
        ),
        route(
            "create_note_digest",
            "Write digest Markdown for an existing note.",
            HttpMethod::Post,
            "/api/create-note-digest",
            object_schema(
                json!({
                    "id": { "type": "string", "description": "Archive entry id." },
                    "digest": { "type": "string", "description": "Digest Markdown." },
                    "force": {
                        "type": "boolean",
                        "description": "Overwrite an existing digest when true."
                    }
                }),
                &["id", "digest"],
            ),
            false,
            true,
        ),
    ]
}

pub(super) fn todo_tool_routes() -> Vec<ToolRoute> {
    vec![
        route(
            "create_todo_task",
            "Create a todo task with a title and optional Markdown body, category, and initial subtask titles.",
            HttpMethod::Post,
            "/api/todo-task-create",
            object_schema(
                json!({
                    "title": { "type": "string", "description": "Todo task title." },
                    "todo_md": { "type": "string", "description": "Optional task body Markdown." },
                    "category_id": { "type": "string", "description": "Optional todo category id." },
                    "sub_titles": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Optional initial subtask titles."
                    }
                }),
                &["title"],
            ),
            false,
            false,
        ),
        route(
            "update_todo_task",
            "Update a todo task's title, Markdown body, and/or category. Provide at least one field to change.",
            HttpMethod::Post,
            "/api/todo-task-update",
            object_schema(
                json!({
                    "master_task_id": { "type": "string", "description": "Todo task id." },
                    "title": { "type": "string", "description": "Replacement task title." },
                    "todo_md": { "type": "string", "description": "Replacement task body; empty clears it." },
                    "category_id": { "type": "string", "description": "Replacement category id." }
                }),
                &["master_task_id"],
            ),
            false,
            false,
        ),
        route(
            "list_todo_tasks",
            "List todo tasks, optionally filtered by category id.",
            HttpMethod::Get,
            "/api/todo-tasks",
            object_schema(
                json!({
                    "category_id": { "type": "string", "description": "Optional category id filter." }
                }),
                &[],
            ),
            true,
            false,
        ),
        route(
            "list_todo_categories",
            "List all todo categories.",
            HttpMethod::Get,
            "/api/todo-task-list-categories",
            object_schema(json!({}), &[]),
            true,
            false,
        ),
        route(
            "get_todo_task",
            "Get one todo task by id.",
            HttpMethod::Get,
            "/api/todo-task",
            object_schema(
                json!({ "id": { "type": "string", "description": "Todo task id." } }),
                &["id"],
            ),
            true,
            false,
        ),
        route(
            "delete_todo_task",
            "Delete a todo task and its contents.",
            HttpMethod::Post,
            "/api/todo-task-delete",
            object_schema(
                json!({ "master_task_id": { "type": "string", "description": "Todo task id." } }),
                &["master_task_id"],
            ),
            false,
            true,
        ),
        route(
            "add_todo_sub",
            "Add a subtask to a todo task.",
            HttpMethod::Post,
            "/api/todo-task-add-sub",
            object_schema(
                json!({
                    "master_task_id": { "type": "string", "description": "Todo task id." },
                    "title": { "type": "string", "description": "Subtask title." },
                    "content": { "type": "string", "description": "Optional subtask content." }
                }),
                &["master_task_id", "title"],
            ),
            false,
            false,
        ),
        route(
            "update_todo_sub",
            "Update a subtask title and optional content. A title is always required.",
            HttpMethod::Post,
            "/api/todo-task-update-sub",
            object_schema(
                json!({
                    "master_task_id": { "type": "string", "description": "Todo task id." },
                    "sub_task_id": { "type": "string", "description": "Subtask id." },
                    "title": { "type": "string", "description": "Replacement subtask title." },
                    "content": { "type": "string", "description": "Optional replacement content." }
                }),
                &["master_task_id", "sub_task_id", "title"],
            ),
            false,
            false,
        ),
        route(
            "delete_todo_sub",
            "Delete a subtask.",
            HttpMethod::Post,
            "/api/todo-task-delete-sub",
            object_schema(
                json!({
                    "master_task_id": { "type": "string", "description": "Todo task id." },
                    "sub_task_id": { "type": "string", "description": "Subtask id." }
                }),
                &["master_task_id", "sub_task_id"],
            ),
            false,
            true,
        ),
        route(
            "complete_todo",
            "Complete a todo task or one of its subtasks.",
            HttpMethod::Post,
            "/api/todo-task-complete",
            object_schema(
                json!({
                    "master_task_id": { "type": "string", "description": "Todo task id." },
                    "sub_task_id": { "type": "string", "description": "Optional subtask id." }
                }),
                &["master_task_id"],
            ),
            false,
            false,
        ),
        route(
            "link_todo_archive",
            "Link an archive entry to a todo subtask.",
            HttpMethod::Post,
            "/api/todo-task-link-archive",
            object_schema(
                json!({
                    "master_task_id": { "type": "string", "description": "Todo task id." },
                    "sub_task_id": { "type": "string", "description": "Subtask id." },
                    "archive_id": { "type": "string", "description": "Archive entry id." }
                }),
                &["master_task_id", "sub_task_id", "archive_id"],
            ),
            false,
            false,
        ),
        route(
            "add_todo_attachment",
            "Copy a local Markdown file into a todo task as an attachment. Use source_path, never file content.",
            HttpMethod::Post,
            "/api/todo-task-add-attachment",
            object_schema(
                json!({
                    "master_task_id": { "type": "string", "description": "Todo task id." },
                    "source_path": { "type": "string", "description": "Absolute source Markdown path." }
                }),
                &["master_task_id", "source_path"],
            ),
            false,
            false,
        ),
        route(
            "list_todo_attachments",
            "List attachments for a todo task.",
            HttpMethod::Post,
            "/api/todo-task-list-attachments",
            object_schema(
                json!({ "master_task_id": { "type": "string", "description": "Todo task id." } }),
                &["master_task_id"],
            ),
            true,
            false,
        ),
        route(
            "get_todo_attachment",
            "Read one todo attachment by file name.",
            HttpMethod::Post,
            "/api/todo-task-get-attachment",
            object_schema(
                json!({
                    "master_task_id": { "type": "string", "description": "Todo task id." },
                    "file_name": { "type": "string", "description": "Attachment file name." }
                }),
                &["master_task_id", "file_name"],
            ),
            true,
            false,
        ),
        route(
            "update_todo_attachment",
            "Replace an existing todo attachment by copying a local Markdown file. Use source_path, never file content.",
            HttpMethod::Post,
            "/api/todo-task-update-attachment",
            object_schema(
                json!({
                    "master_task_id": { "type": "string", "description": "Todo task id." },
                    "file_name": { "type": "string", "description": "Existing attachment file name." },
                    "source_path": { "type": "string", "description": "Absolute replacement Markdown path." }
                }),
                &["master_task_id", "file_name", "source_path"],
            ),
            false,
            true,
        ),
    ]
}

/// Build the Host-authoritative slot→tool routing table.
/// Unregistered slots return `None` (caller must HTTP hard-reject).
pub fn build_slot_tool_table(slot: &str) -> Option<SlotToolTable> {
    let api = scene_slot_api(slot)?;
    let mut tools = Vec::new();
    if api.include_notes {
        tools.extend(notes_tool_routes());
    }
    if api.include_todo {
        tools.extend(todo_tool_routes());
    }
    Some(SlotToolTable {
        scene_slot: slot.to_string(),
        include_notes: api.include_notes,
        include_todo: api.include_todo,
        tools,
    })
}

/// Allowlisted tool names for a slot (empty when unregistered).
pub fn tools_list_for_slot(slot: &str) -> Vec<ToolDescriptor> {
    build_slot_tool_table(slot)
        .map(|table| {
            table
                .tools
                .into_iter()
                .map(|t| ToolDescriptor { name: t.name })
                .collect()
        })
        .unwrap_or_default()
}

/// Grouped catalog for Settings checkboxes. Not a slot table.
pub fn catalog_groups() -> Vec<(&'static str, Vec<ToolRoute>)> {
    vec![
        ("notes", notes_tool_routes()),
        ("todo", todo_tool_routes()),
    ]
}

