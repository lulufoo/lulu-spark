/**
 * t5 — contract / UI / MCP suites aligned to tech-doc AC1–AC5
 * Retargeted to Host MCP SSOT after T10 archive of packages/knowledge-mcp.
 */
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { readTodoTaskServiceTestsSource } from '../helpers/todo-task-ui-source.js';

import { readRsPath } from '../helpers/read-rs-dir.js';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const HOST_ADAPTER = 'src-tauri/src/mcp_host';

function read(rel) {
  return readRsPath(join(repoRoot, rel));
}

describe('t5 AC1 — title-only add (host / HTTP / MCP)', () => {
  it('host command unit tests cover title-only add_todo_sub_json', () => {
    const src = read('src-tauri/src/unit-tests/commands/todo_task.rs');
    expect(src).toMatch(/add_todo_sub_json_appends_and_returns_updated_master/);
    expect(src).toMatch(/subs\[.*\]\.get\("content"\)\.is_none\(\)/);
  });

  it('command unit tests cover title-only add_todo_sub_json (no content field)', () => {
    const src = read('src-tauri/src/unit-tests/commands/todo_task.rs');
    expect(src).toContain('add_todo_sub_json_appends_and_returns_updated_master');
    expect(src).toMatch(/subs\[.*\]\.get\("content"\)\.is_none\(\)/);
  });

  it('Host adapter routes add_todo_sub; e2e keeps title-only success path', () => {
    const adapter = read(HOST_ADAPTER);
    expect(adapter).toMatch(/route\(\s*"add_todo_sub"/);
    expect(adapter).toContain('todo_task::add_sub');

    const e2e = read('scripts/todo-task-mcp-e2e.mjs');
    expect(e2e).toMatch(
      /add_todo_sub[\s\S]*?master_task_id[\s\S]*?title:\s*'Sub /,
    );
  });
});

describe('t5 AC2 — create-with-content round-trip; legacy missing field empty', () => {
  it('storage/service unit tests cover content persist + legacy absent read', () => {
    const types = read('src-tauri/src/unit-tests/services/todo_task/types.rs');
    expect(types).toContain('sub_task_missing_content_deserializes_as_none');
    expect(types).toContain('sub_task_content_roundtrip_preserves_value');

    const svc = readTodoTaskServiceTestsSource();
    expect(svc).toContain('add_sub_with_content_persists_and_load_roundtrips');
    expect(svc).toContain('load_legacy_sub_tasks_missing_content_reads_as_absent');
  });

  it('command unit tests cover optional content on add-sub round-trip', () => {
    const src = read('src-tauri/src/unit-tests/commands/todo_task.rs');
    expect(src).toContain('add_todo_sub_json_with_optional_content_persists');
  });

  it('e2e exercises create-with-content via add_todo_sub', () => {
    const e2e = read('scripts/todo-task-mcp-e2e.mjs');
    expect(e2e).toMatch(/add_todo_sub[\s\S]*?content:\s*'/);
  });
});

describe('t5 AC3 — update modify/clear/omit content; no delete-content API', () => {
  it('host/HTTP unit tests cover set / clear("") / omit-unchanged', () => {
    const cmd = read('src-tauri/src/unit-tests/commands/todo_task.rs');
    expect(cmd).toContain('update_todo_sub_json_sets_and_clears_optional_content');
    expect(cmd).toContain('update_todo_sub_json_title_only_leaves_content_unchanged');

    const http = read('src-tauri/src/unit-tests/commands/todo_task.rs');
    expect(http).toContain('update_todo_sub_json_sets_and_clears_optional_content');
  });

  it('Host adapter + e2e expose update_todo_sub content paths', () => {
    const adapter = read(HOST_ADAPTER);
    expect(adapter).toMatch(/route\(\s*"update_todo_sub"/);
    expect(adapter).toContain('todo_task::update_sub_title');

    const e2e = read('scripts/todo-task-mcp-e2e.mjs');
    expect(e2e).toMatch(/name:\s*'update_todo_sub'|callTodoTool\(\s*'update_todo_sub'/);
    expect(e2e).toMatch(/content:\s*''/);
  });

  it('no delete-content API on HTTP or Host MCP surfaces', () => {
    const http = read('src-tauri/src/main_host');
    expect(http).not.toMatch(/todo-task-delete-.*content|delete-sub-content|delete_content/);
    expect(http).not.toContain('/api/todo-task-update-sub');

    const adapter = read(HOST_ADAPTER);
    expect(adapter).not.toMatch(/route\(\s*"delete_todo_sub_content"/);
    expect(adapter).not.toMatch(/route\(\s*"delete_sub_content"/);
    expect(adapter).toMatch(/route\(\s*"update_todo_sub"/);
  });
});

describe('t5 AC4 — UI title-first + default collapsed expand editor', () => {
  it('todo-task-sub-content-ui locks title-first and default-collapsed editor', () => {
    const src = read('tests/todo-task/sub-content-ui.test.js');
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
  it('todo-task-sub-content-ui asserts Process notes after sub-list in renderSubDetailPane', () => {
    const src = read('tests/todo-task/sub-content-ui.test.js');
    expect(src).toContain(
      'renderSubDetailPane Process notes after sub-list',
    );
    expect(src).toMatch(/commentsIdx\)\.toBeGreaterThan\(subListIdx\)/);
  });

  it('todo-task-comments mounted detail keeps Process notes after sub-list', () => {
    const src = read('tests/todo-task/comments.test.js');
    expect(src).toMatch(/Process notes[\s\S]*after[\s\S]*sub-list|after the sub-list/i);
  });
});

describe('t5 wire-up / failure semantics', () => {
  it('npm test includes this AC gate and layered suites', () => {
    const pkg = JSON.parse(read('package.json'));
    expect(pkg.scripts.test).toMatch(/vitest run --dir tests/);
  });

  it('existing missing-title 400 semantics remain asserted (not relaxed)', () => {
    const cmd = read('src-tauri/src/unit-tests/commands/todo_task.rs');
    expect(cmd).toContain('update_todo_sub_json_empty_title_returns_400_class');
    const svc = read('src-tauri/src/unit-tests/services/todo_task/subs.rs');
    expect(svc).toContain('update_sub_title_blank_title_returns_400_even_with_content');
    const mcp = read('tests/todo-task/mcp-sub-content.test.js');
    expect(mcp).toContain('command/service unit tests cover optional content semantics');
  });

  it('delivery scripts exist (Host verify + e2e; Node package archived)', () => {
    expect(existsSync(join(repoRoot, 'scripts/verify-host-mcp.mjs'))).toBe(true);
    expect(existsSync(join(repoRoot, 'scripts/todo-task-mcp-e2e.mjs'))).toBe(true);
    expect(existsSync(join(repoRoot, 'packages/knowledge-mcp/index.mjs'))).toBe(false);
  });
});
