/**
 * t3 — MCP add_todo_sub optional content + update_todo_sub
 * Retargeted to Host Protocol Adapter SSOT (T10).
 */
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const HOST_ADAPTER = 'src-tauri/src/services/mcp_protocol_adapter.rs';

function read(rel) {
  return readFileSync(join(repoRoot, rel), 'utf8');
}

function toolRouteBlock(src, toolName) {
  const start = src.search(new RegExp(`route\\(\\s*"${toolName}"`));
  expect(start, `${toolName} route missing`).toBeGreaterThanOrEqual(0);
  const rest = src.slice(start + 1);
  const nextRel = rest.search(/route\(\s*"/);
  return nextRel === -1 ? src.slice(start) : src.slice(start, start + 1 + nextRel);
}

describe('t3 MCP add_todo_sub optional content (Host SSOT)', () => {
  it('add_todo_sub routes to Sidecar POST /api/todo-task-add-sub', () => {
    const block = toolRouteBlock(read(HOST_ADAPTER), 'add_todo_sub');
    expect(block).toMatch(/HttpMethod::Post/);
    expect(block).toContain('/api/todo-task-add-sub');
  });
});

describe('t3 MCP update_todo_sub (Host SSOT)', () => {
  it('registers update_todo_sub proxying POST /api/todo-task-update-sub', () => {
    const src = read(HOST_ADAPTER);
    expect(src).toMatch(/route\(\s*"update_todo_sub"/);
    const block = toolRouteBlock(src, 'update_todo_sub');
    expect(block).toContain('/api/todo-task-update-sub');
  });

  it('Sidecar HTTP handlers cover optional content semantics (title-only / set / clear)', () => {
    const http = read('src-tauri/src/unit-tests/services/local_http.rs');
    expect(http).toContain('post_todo_task_add_sub_title_only_without_content_field_returns_201');
    expect(http).toContain('post_todo_task_add_sub_with_optional_content_persists_via_get');
    expect(http).toContain('post_todo_task_update_sub_writes_clears_and_omits_content');
    expect(http).toContain('post_todo_task_add_sub_missing_or_blank_title_returns_400');
    expect(http).toContain('post_todo_task_update_sub_missing_or_blank_title_returns_400');
  });
});

describe('t3 wire-up', () => {
  it('npm test includes todo-task-mcp-sub-content.test.js', () => {
    const pkg = JSON.parse(read('package.json'));
    expect(pkg.scripts.test).toContain('tests/todo-task-mcp-sub-content.test.js');
  });
});
