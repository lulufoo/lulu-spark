import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

import { readRsPath } from '../helpers/read-rs-dir.js';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const HOST_ADAPTER = 'src-tauri/src/services/mcp_host';

/** tech-doc 公开契约映射 — todo_* MCP tools (+ update_todo_sub / list_todo_categories); no complete_plan_sub. */
const EXPECTED_TODO_TOOLS = [
  'create_todo_task',
  'update_todo_task',
  'list_todo_tasks',
  'get_todo_task',
  'delete_todo_task',
  'add_todo_sub',
  'update_todo_sub',
  'delete_todo_sub',
  'complete_todo',
  'link_todo_archive',
  'add_todo_attachment',
  'list_todo_attachments',
  'get_todo_attachment',
  'update_todo_attachment',
  'list_todo_categories',
];

const FORBIDDEN_PLAN_TOOLS = [
  'create_plan_task',
  'list_plan_tasks',
  'get_plan_task',
  'delete_plan_task',
  'add_plan_sub',
  'delete_plan_sub',
  'complete_plan',
  'complete_plan_sub',
  'link_plan_archive',
  'add_plan_attachment',
  'list_plan_attachments',
  'get_plan_attachment',
  'update_plan_attachment',
];

/** L09#4 — MCP must not expose category directory create/delete. */
const FORBIDDEN_CATEGORY_CRUD_TOOLS = [
  'create_todo_category',
  'delete_todo_category',
  'add_todo_category',
  'remove_todo_category',
];

function registeredToolNames(src) {
  return [...src.matchAll(/route\(\s*"([^"]+)"/g)].map((m) => m[1]);
}

function toolRouteBlock(src, toolName) {
  const start = src.search(new RegExp(`route\\(\\s*"${toolName}"`));
  expect(start, `${toolName} route missing`).toBeGreaterThanOrEqual(0);
  const rest = src.slice(start + 1);
  const nextRel = rest.search(/route\(\s*"/);
  return nextRel === -1 ? src.slice(start) : src.slice(start, start + 1 + nextRel);
}

describe('MCP tool surface hard-cut to todo_* (Host SSOT / T10)', () => {
  it('Host adapter registers exactly the todo_* tools incl. update_todo_sub; old plan_* names absent', () => {
    const indexPath = join(repoRoot, HOST_ADAPTER);
    expect(existsSync(indexPath), `missing ${HOST_ADAPTER}`).toBe(true);
    const src = readRsPath(indexPath);
    const names = registeredToolNames(src);

    for (const tool of EXPECTED_TODO_TOOLS) {
      expect(names, `missing ${tool}`).toContain(tool);
    }
    for (const tool of FORBIDDEN_PLAN_TOOLS) {
      expect(names, `forbidden ${tool} still registered`).not.toContain(tool);
    }
    expect(names).not.toContain('complete_plan_sub');

    const taskSurface = names.filter(
      (n) =>
        n.includes('todo_') ||
        n.includes('plan_') ||
        n === 'complete_todo' ||
        n === 'complete_plan',
    );
    expect(taskSurface.sort()).toEqual([...EXPECTED_TODO_TOOLS].sort());
  });

  it('create_todo_task routes to Sidecar create API (todo_md contract lives on Host HTTP)', () => {
    const src = readRsPath(join(repoRoot, HOST_ADAPTER));
    const block = toolRouteBlock(src, 'create_todo_task');
    expect(block).toContain('/api/todo-task-create');
    expect(block).toMatch(/&\["title", "todo_md"\]/);
    expect(block).not.toMatch(/Optional task body/);
    expect(src).not.toMatch(/name:\s*"create_plan_task"/);
  });

  it('delivery e2e entry is scripts/todo-task-mcp-e2e.mjs; old plan-task-mcp-e2e.mjs removed', () => {
    const todoE2e = join(repoRoot, 'scripts/todo-task-mcp-e2e.mjs');
    const planE2e = join(repoRoot, 'scripts/plan-task-mcp-e2e.mjs');
    expect(existsSync(todoE2e), 'missing todo-task-mcp-e2e.mjs').toBe(true);
    expect(existsSync(planE2e), 'old plan-task-mcp-e2e.mjs must not remain as delivery entry').toBe(
      false,
    );
  });

  it('npm test includes todo-task MCP surface test', () => {
    const pkg = JSON.parse(readFileSync(join(repoRoot, 'package.json'), 'utf8'));
    expect(pkg.scripts.test).toMatch(/vitest run --dir tests/);
  });
});

describe('MCP category surface (tech-doc T-3 / AC1–AC4 / L09#4)', () => {
  it('registers list_todo_categories as thin proxy GET to Sidecar list-categories', () => {
    const src = readRsPath(join(repoRoot, HOST_ADAPTER));
    const block = toolRouteBlock(src, 'list_todo_categories');
    expect(block).toMatch(/HttpMethod::Get/);
    expect(block).toContain('/api/todo-task-list-categories');
  });

  it('forbids MCP category directory create/delete tools (L09#4)', () => {
    const src = readRsPath(join(repoRoot, HOST_ADAPTER));
    const names = registeredToolNames(src);
    for (const tool of FORBIDDEN_CATEGORY_CRUD_TOOLS) {
      expect(names, `forbidden category CRUD tool ${tool}`).not.toContain(tool);
    }
  });

  it('create_todo_task / list / update route to Sidecar paths that accept category_id', () => {
    const src = readRsPath(join(repoRoot, HOST_ADAPTER));
    expect(toolRouteBlock(src, 'create_todo_task')).toContain('/api/todo-task-create');
    expect(toolRouteBlock(src, 'list_todo_tasks')).toContain('/api/todo-tasks');
    expect(toolRouteBlock(src, 'update_todo_task')).toContain('/api/todo-task-update');
    const names = registeredToolNames(src);
    expect(names).not.toContain('set_todo_category');
    expect(names).not.toContain('set_todo_task_category');
  });

  it('Sidecar local_http exposes list-categories route and category_id on create/list/update', () => {
    const httpSrc = readRsPath(join(repoRoot, 'src-tauri/src/services/local_http'));
    expect(httpSrc).toMatch(/\/api\/todo-task-list-categories/);
    expect(httpSrc).toMatch(/category_id/);
  });
});
