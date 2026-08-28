/**
 * T15 AC fixture — brand sites + static probes for todo-task attachment AC gate.
 * Consumed by todo-task-ui-ac-gate.test.js and todo-task-f3-copy.test.js.
 */

const NO_LEGACY_BRAND = [/计划任务/];

/** @type {{ path: string, mustMatch: RegExp[], mustNotMatch?: RegExp[] }[]} */
export const TODO_TASK_BRAND_SITES = [
  {
    path: 'frontend/src/todo-task/list.tsx',
    mustMatch: [/<h1 class(?:Name)?="todo-tasks-page-title">Todos<\/h1>/],
    mustNotMatch: NO_LEGACY_BRAND,
  },
  {
    path: 'frontend/src/todo-task/page-render.tsx',
    mustMatch: [/aria-label="Todos list"/],
    mustNotMatch: NO_LEGACY_BRAND,
  },
  {
    path: 'frontend/src/home/hub.tsx',
    mustMatch: [/<span class(?:Name)?="home-desktop-shortcut-label">Todos<\/span>/],
    mustNotMatch: NO_LEGACY_BRAND,
  },
  {
    path: 'frontend/src/todo-task/assistant.tsx',
    mustMatch: [/No todos yet/],
    mustNotMatch: NO_LEGACY_BRAND,
  },
  {
    // T6: popover chrome retired; brand title lives on the content adapter / EntryConfig.
    path: 'frontend/src/todo-task/assistant.tsx',
    mustMatch: [/TODO_TASK_CONTENT_TITLE\s*=\s*['"]Todos['"]/],
    mustNotMatch: NO_LEGACY_BRAND,
  },
  {
    path: 'frontend/src/todo-task/assistant.tsx',
    mustMatch: [/TODO_TASK_CONTENT_LABEL\s*=\s*['"]Open Todos['"]/, /['"]Todos['"]/],
    mustNotMatch: NO_LEGACY_BRAND,
  },
];

/** Rust service unit-test markers locking AC1/AC3/AC6/AC8 (and T15 AC locks). */
export const RUST_SERVICE_AC_TESTS = [
  'fn add_attachment_stem_conflict_appends_numeric_suffix',
  'fn add_attachment_manifest_write_failure_rolls_back_copy',
  'fn delete_attachment_removes_manifest_entry_and_file',
  'fn delete_attachment_file_delete_failure_restores_manifest',
  'fn list_attachments_missing_manifest_or_empty_returns_empty_collection',
  'fn add_attachment_rejects_non_md_without_side_effects',
  'fn delete_master_cascades_attachments_dir_and_manifest',
  'fn ac15_stem_conflict_appends_suffix_and_keeps_originals',
  'fn ac15_add_manifest_failure_rolls_back_without_half_success',
  'fn ac15_delete_clears_manifest_entry_and_file',
  'fn ac15_empty_list_and_non_md_reject',
];

export const ATTACHMENT_COMMANDS = [
  'add_todo_attachment',
  'list_todo_attachments',
  'read_todo_attachment',
  'save_todo_attachment',
  'delete_todo_attachment',
];

export const MCP_ATTACHMENT_TOOLS = [
  'add_todo_attachment',
  'list_todo_attachments',
  'get_todo_attachment',
  'update_todo_attachment',
];

export const MCP_FORBIDDEN_DELETE_TOOLS = [
  'delete_plan_attachment',
  'remove_plan_attachment',
  'delete_todo_attachment',
  'remove_todo_attachment',
];

/** Integration-test file → regex probes for UI add/list/editor/save paths. */
export const UI_ATTACHMENT_TEST_PROBES = {
  'tests/todo-task/attachments.test.js': [
    /listPlanAttachments invokes list_todo_attachments/,
    /addPlanAttachment invokes add_todo_attachment/,
    /shows attachment section listing associated files/,
    /pick flow stages then invokes add_todo_attachment/,
  ],
  'tests/todo-task/attachment-editor.test.js': [
    /readPlanAttachment invokes read_todo_attachment/,
    /savePlanAttachment invokes save_todo_attachment with masterTaskId, fileName, sourcePath/,
    /clicking an attachment opens a modal with preview by default/,
    /can switch to edit mode and save via save_todo_attachment/,
  ],
  'tests/todo-task/attachment-delete.test.js': [
    /invokes delete_todo_attachment with masterTaskId and fileName/,
    /confirming delete invokes delete_todo_attachment then removes item from list/,
  ],
};
