/**
 * t3 — MCP add_todo_sub required content + update_todo_sub blank reject
 * Retargeted to Host Protocol Adapter SSOT (T10).
 */
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

import { readRsPath } from '../helpers/read-rs-dir.js';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const HOST_ADAPTER = 'src-tauri/src/mcp_host';

function read(rel) {
  return readRsPath(join(repoRoot, rel));
}

function toolRouteBlock(src, toolName) {
  const header = src.search(new RegExp(`//! MCP API: \`${toolName}\``));
  const routeStart = src.search(new RegExp(`route\\(\\s*"${toolName}"`));
  expect(routeStart, `${toolName} route missing`).toBeGreaterThanOrEqual(0);
  const start = header >= 0 ? header : routeStart;
  const rest = src.slice(start + 1);
  const nextRel = rest.search(/\/\/! MCP API:/);
  return nextRel === -1 ? src.slice(start) : src.slice(start, start + 1 + nextRel);
}

describe('t3 MCP add_todo_sub required content (Host SSOT)', () => {
  it('add_todo_sub invokes Services add_sub', () => {
    const block = toolRouteBlock(read(HOST_ADAPTER), 'add_todo_sub');
    expect(block).toContain('todo_task::add_sub');
    expect(block).toMatch(/&\["master_task_id", "title", "content"\]/);
    expect(block).toContain('require_nonempty_str');
    expect(block).not.toContain('Optional subtask content');
  });
});

describe('t3 MCP update_todo_sub (Host SSOT)', () => {
  it('registers update_todo_sub invoking Services update_sub_title', () => {
    const src = read(HOST_ADAPTER);
    expect(src).toMatch(/route\(\s*"update_todo_sub"/);
    const block = toolRouteBlock(src, 'update_todo_sub');
    expect(block).toContain('todo_task::update_sub_title');
    expect(block).toContain('optional_nonempty_str');
    expect(block).toMatch(/&\["master_task_id", "sub_task_id", "title"\]/);
  });

  it('command/service unit tests cover optional content semantics (title-only / set / clear)', () => {
    const cmd = read('src-tauri/src/unit-tests/commands/todo_task.rs');
    expect(cmd).toContain('add_todo_sub_json_appends_and_returns_updated_master');
    expect(cmd).toContain('add_todo_sub_json_with_optional_content_persists');
    expect(cmd).toContain('update_todo_sub_json_sets_and_clears_optional_content');
    expect(cmd).toContain('update_todo_sub_json_empty_title_returns_400_class');
    const svc = read('src-tauri/src/unit-tests/services/todo_task/subs.rs');
    expect(svc).toContain('add_sub_without_content_matches_title_only_behavior');
    expect(svc).toContain('update_sub_title_blank_title_returns_400_even_with_content');
  });
});

describe('t3 wire-up', () => {
  it('npm test includes todo-task-mcp-sub-content.test.js', () => {
    const pkg = JSON.parse(read('package.json'));
    expect(pkg.scripts.test).toMatch(/vitest run --dir tests/);
  });
});
