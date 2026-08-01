/**
 * t5 — contract / UI / MCP suites aligned to tech-doc AC1–AC5
 * (optional SubTask content + Process notes order).
 *
 * Source-lock gate: host/HTTP/MCP/UI layers must expose the AC behaviors.
 * Does not rewrite T10's 13-tool EQUIVALENCE set; requires update_todo_sub
 * as an additive sub-content surface in verify/e2e.
 */
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..');

function read(rel) {
  return readFileSync(join(repoRoot, rel), 'utf8');
}

function extractQuotedToolList(src, constName) {
  const re = new RegExp(`const ${constName}\\s*=\\s*\\[([\\s\\S]*?)\\];`);
  const m = src.match(re);
  if (!m) return null;
  return [...m[1].matchAll(/['"]([^'"]+)['"]/g)].map((x) => x[1]);
}

describe('t5 AC1 — title-only add (host / HTTP / MCP)', () => {
  it('host command unit tests cover title-only add_todo_sub_json', () => {
    const src = read('src-tauri/src/unit-tests/commands/todo_task.rs');
    expect(src).toMatch(/add_todo_sub_json_appends_and_returns_updated_master/);
    expect(src).toMatch(/subs\[.*\]\.get\("content"\)\.is_none\(\)/);
  });

  it('HTTP unit tests cover title-only POST /api/todo-task-add-sub (no content field → 201)', () => {
    const src = read('src-tauri/src/unit-tests/services/local_http.rs');
    expect(src).toContain('post_todo_task_add_sub_title_only_without_content_field_returns_201');
    expect(src).toContain('/api/todo-task-add-sub');
  });

  it('MCP schema + verify/e2e keep title-only add_todo_sub success path', () => {
    const index = read('packages/knowledge-mcp/index.mjs');
    expect(index).toMatch(/registerTool\(\s*'add_todo_sub'/);
    expect(index).toMatch(/if\s*\(\s*content\s*!=\s*null\s*\)/);

    const verify = read('packages/knowledge-mcp/scripts/verify.mjs');
    expect(verify).toMatch(/name:\s*'add_todo_sub'/);
    // title-only call must not require content in mock path
    expect(verify).toMatch(
      /todo-task-add-sub[\s\S]*?respondJson\(\s*res,\s*201/,
    );

    const e2e = read('packages/knowledge-mcp/scripts/todo-task-mcp-e2e.mjs');
    expect(e2e).toMatch(
      /add_todo_sub[\s\S]*?master_task_id[\s\S]*?title:\s*'Sub /,
    );
  });
});

describe('t5 AC2 — create-with-content round-trip; legacy missing field empty', () => {
  it('storage/service unit tests cover content persist + legacy absent read', () => {
    const types = read('src-tauri/src/unit-tests/services/todo_task_types.rs');
    expect(types).toContain('sub_task_missing_content_deserializes_as_none');
    expect(types).toContain('sub_task_content_roundtrip_preserves_value');

    const svc = read('src-tauri/src/unit-tests/services/todo_task.rs');
    expect(svc).toContain('add_sub_with_content_persists_and_load_roundtrips');
    expect(svc).toContain('load_legacy_sub_tasks_missing_content_reads_as_absent');
  });

  it('HTTP unit tests cover optional content on add-sub round-trip', () => {
    const src = read('src-tauri/src/unit-tests/services/local_http.rs');
    expect(src).toContain('post_todo_task_add_sub_with_optional_content_persists_via_get');
  });

  it('verify mock persists optional content on add-sub; e2e/verify exercise create-with-content', () => {
    const verify = read('packages/knowledge-mcp/scripts/verify.mjs');
    // mock must accept optional content (not drop it)
    expect(verify).toMatch(
      /todo-task-add-sub[\s\S]*?payload\.content[\s\S]*?sub_tasks\.push/,
    );
    expect(verify).toMatch(/name:\s*'add_todo_sub'[\s\S]*?content:\s*'/);

    const e2e = read('packages/knowledge-mcp/scripts/todo-task-mcp-e2e.mjs');
    expect(e2e).toMatch(/add_todo_sub[\s\S]*?content:\s*'/);
  });
});

describe('t5 AC3 — update modify/clear/omit content; no delete-content API', () => {
  it('host/HTTP unit tests cover set / clear("") / omit-unchanged', () => {
    const cmd = read('src-tauri/src/unit-tests/commands/todo_task.rs');
    expect(cmd).toContain('update_todo_sub_json_sets_and_clears_optional_content');
    expect(cmd).toContain('update_todo_sub_json_title_only_leaves_content_unchanged');

    const http = read('src-tauri/src/unit-tests/services/local_http.rs');
    expect(http).toContain('post_todo_task_update_sub_writes_clears_and_omits_content');
  });

  it('verify mock exposes update-sub; verify/e2e exercise update_todo_sub content paths', () => {
    const verify = read('packages/knowledge-mcp/scripts/verify.mjs');
    expect(verify).toContain('/api/todo-task-update-sub');
    const subContentTools =
      extractQuotedToolList(verify, 'SUB_CONTENT_TODO_TOOLS') ||
      extractQuotedToolList(verify, 'SUB_CONTENT_MCP_TOOLS');
    expect(
      subContentTools,
      'verify.mjs must declare SUB_CONTENT_TODO_TOOLS incl. update_todo_sub',
    ).toBeTruthy();
    expect(subContentTools).toContain('update_todo_sub');
    expect(verify).toMatch(/name:\s*'update_todo_sub'/);
    expect(verify).toMatch(/content:\s*''/);

    const e2e = read('packages/knowledge-mcp/scripts/todo-task-mcp-e2e.mjs');
    expect(e2e).toMatch(/name:\s*'update_todo_sub'|callTodoTool\(\s*'update_todo_sub'/);
    expect(e2e).toMatch(/content:\s*''/);
  });

  it('no delete-content API on HTTP or MCP surfaces', () => {
    const http = read('src-tauri/src/services/local_http/mod.rs');
    expect(http).not.toMatch(/todo-task-delete-.*content|delete-sub-content|delete_content/);
    expect(http).toContain('/api/todo-task-update-sub');

    const index = read('packages/knowledge-mcp/index.mjs');
    expect(index).not.toMatch(
      /registerTool\(\s*'delete_todo_sub_content'|registerTool\(\s*'delete_sub_content'/,
    );
    expect(index).toMatch(/registerTool\(\s*'update_todo_sub'/);
  });
});

describe('t5 AC4 — UI title-first + default collapsed expand editor', () => {
  it('plan-task-sub-content-ui locks title-first and default-collapsed editor', () => {
    const src = read('tests/plan-task-sub-content-ui.test.js');
    expect(src).toContain(
      'renderSubRow title-first + default-collapsed content editor',
    );
    expect(src).toContain(
      'keeps title input as primary row and collapses content editor by default',
    );
    expect(src).toContain(
      'shows editable content editor only when session expand flag is set',
    );
  });
});

describe('t5 AC5 — Process notes after sub-list', () => {
  it('plan-task-sub-content-ui asserts Process notes after sub-list in renderSubDetailPane', () => {
    const src = read('tests/plan-task-sub-content-ui.test.js');
    expect(src).toContain(
      'renderSubDetailPane Process notes after sub-list',
    );
    expect(src).toMatch(/commentsIdx\)\.toBeGreaterThan\(subListIdx\)/);
  });

  it('plan-task-comments mounted detail keeps Process notes after sub-list', () => {
    const src = read('tests/plan-task-comments.test.js');
    expect(src).toMatch(/Process notes[\s\S]*after[\s\S]*sub-list|after the sub-list/i);
  });
});

describe('t5 wire-up / failure semantics', () => {
  it('npm test includes this AC gate and layered suites', () => {
    const pkg = JSON.parse(read('package.json'));
    expect(pkg.scripts.test).toContain('tests/todo-task-sub-content-ac-gate.test.js');
    expect(pkg.scripts.test).toContain('tests/todo-task-mcp-sub-content.test.js');
    expect(pkg.scripts.test).toContain('tests/plan-task-sub-content-ui.test.js');
  });

  it('existing missing-title 400 semantics remain asserted (not relaxed)', () => {
    const http = read('src-tauri/src/unit-tests/services/local_http.rs');
    expect(http).toContain('post_todo_task_add_sub_missing_or_blank_title_returns_400');
    expect(http).toContain('post_todo_task_update_sub_missing_or_blank_title_returns_400');
    const mcp = read('tests/todo-task-mcp-sub-content.test.js');
    expect(mcp).toContain(
      'rejects when both title and content are omitted (no destructive empty-title POST)',
    );
  });

  it('delivery scripts exist', () => {
    expect(existsSync(join(repoRoot, 'packages/knowledge-mcp/scripts/verify.mjs'))).toBe(
      true,
    );
    expect(
      existsSync(join(repoRoot, 'packages/knowledge-mcp/scripts/todo-task-mcp-e2e.mjs')),
    ).toBe(true);
  });
});
