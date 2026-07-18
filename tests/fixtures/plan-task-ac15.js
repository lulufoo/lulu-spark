/**
 * T15 AC fixture — brand sites + static probes for plan-task attachment AC gate.
 * Consumed by plan-task-ac-gate.test.js and plan-task-f3-copy.test.js.
 */

const NO_LEGACY_BRAND = [/计划任务/];

/** @type {{ path: string, mustMatch: RegExp[], mustNotMatch?: RegExp[] }[]} */
export const PLAN_TASK_BRAND_SITES = [
  {
    path: 'frontend/js/plan-task/index.js',
    mustMatch: [/<h1 class="plan-tasks-page-title">Todos<\/h1>/],
    mustNotMatch: NO_LEGACY_BRAND,
  },
  {
    path: 'frontend/js/plan-task/index.js',
    mustMatch: [/aria-label="Todos列表"/],
    mustNotMatch: NO_LEGACY_BRAND,
  },
  {
    path: 'frontend/js/components/home-hub.js',
    mustMatch: [/<span class="home-desktop-shortcut-label">Todos<\/span>/],
    mustNotMatch: NO_LEGACY_BRAND,
  },
  {
    path: 'frontend/plan-task-assistant.html',
    mustMatch: [/<title>Todos助手<\/title>/],
    mustNotMatch: NO_LEGACY_BRAND,
  },
  {
    path: 'frontend/js/plan-task-assistant.js',
    mustMatch: [/暂无Todos/],
    mustNotMatch: NO_LEGACY_BRAND,
  },
  {
    path: 'frontend/js/plan-task-assistant.js',
    mustMatch: [/<span class="pt-assistant-popover-title">Todos助手<\/span>/],
    mustNotMatch: NO_LEGACY_BRAND,
  },
  {
    path: 'frontend/js/plan-task-assistant.js',
    mustMatch: [/aria-label="打开Todos助手"/, /title="Todos助手"/],
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
  'add_plan_attachment',
  'list_plan_attachments',
  'read_plan_attachment',
  'save_plan_attachment',
  'delete_plan_attachment',
];

export const MCP_ATTACHMENT_TOOLS = [
  'add_plan_attachment',
  'list_plan_attachments',
  'get_plan_attachment',
  'update_plan_attachment',
];

export const MCP_FORBIDDEN_DELETE_TOOLS = [
  'delete_plan_attachment',
  'remove_plan_attachment',
];

/** Integration-test file → regex probes for UI add/list/editor/save paths. */
export const UI_ATTACHMENT_TEST_PROBES = {
  'tests/plan-task-attachments.test.js': [
    /listPlanAttachments invokes list_plan_attachments/,
    /addPlanAttachment invokes add_plan_attachment/,
    /shows attachment section listing associated files/,
    /pick flow invokes add_plan_attachment then refreshes list/,
  ],
  'tests/plan-task-attachment-editor.test.js': [
    /readPlanAttachment invokes read_plan_attachment/,
    /savePlanAttachment invokes save_plan_attachment/,
    /clicking an attachment opens a modal with preview by default/,
    /can switch to edit mode and save via save_plan_attachment/,
  ],
  'tests/plan-task-attachment-delete.test.js': [
    /invokes delete_plan_attachment with masterTaskId and fileName/,
    /confirming delete invokes delete_plan_attachment then removes item from list/,
  ],
};
