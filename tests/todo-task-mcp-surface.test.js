import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..');

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
  return [...src.matchAll(/registerTool\(\s*['"]([^'"]+)['"]/g)].map((m) => m[1]);
}

function toolBlock(src, toolName) {
  const start = src.indexOf(`registerTool(\n    '${toolName}'`);
  expect(start, `${toolName} registration missing`).toBeGreaterThanOrEqual(0);
  const next = src.indexOf('registerTool(', start + 1);
  return src.slice(start, next === -1 ? undefined : next);
}

describe('MCP tool surface hard-cut to todo_* (tech-doc T7 / SK-2)', () => {
  it('index.mjs registers exactly the todo_* tools incl. update_todo_sub; old plan_* names absent', () => {
    const indexPath = join(repoRoot, 'packages/knowledge-mcp/index.mjs');
    expect(existsSync(indexPath), 'missing packages/knowledge-mcp/index.mjs').toBe(true);
    const src = readFileSync(indexPath, 'utf8');
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

  it('create_todo_task schema surface uses todo_md (not plan_md)', () => {
    const src = readFileSync(join(repoRoot, 'packages/knowledge-mcp/index.mjs'), 'utf8');
    const createIdx = src.indexOf("registerTool(\n    'create_todo_task'");
    expect(createIdx, 'create_todo_task registration missing').toBeGreaterThanOrEqual(0);
    const nextRegister = src.indexOf('registerTool(', createIdx + 1);
    const createBlock = src.slice(createIdx, nextRegister === -1 ? undefined : nextRegister);
    expect(createBlock).toMatch(/todo_md\s*:/);
    expect(createBlock).not.toMatch(/plan_md\s*:/);
  });

  it('delivery e2e entry is todo-task-mcp-e2e.mjs; old plan-task-mcp-e2e.mjs removed', () => {
    const todoE2e = join(repoRoot, 'packages/knowledge-mcp/scripts/todo-task-mcp-e2e.mjs');
    const planE2e = join(repoRoot, 'packages/knowledge-mcp/scripts/plan-task-mcp-e2e.mjs');
    expect(existsSync(todoE2e), 'missing todo-task-mcp-e2e.mjs').toBe(true);
    expect(existsSync(planE2e), 'old plan-task-mcp-e2e.mjs must not remain as delivery entry').toBe(
      false,
    );
  });

  it('npm test includes todo-task MCP surface test', () => {
    const pkg = JSON.parse(readFileSync(join(repoRoot, 'package.json'), 'utf8'));
    expect(pkg.scripts.test).toContain('tests/todo-task-mcp-surface.test.js');
  });
});

describe('MCP category surface (tech-doc T-3 / AC1–AC4 / L09#4)', () => {
  it('registers list_todo_categories as thin proxy GET to Sidecar list-categories', () => {
    const src = readFileSync(join(repoRoot, 'packages/knowledge-mcp/index.mjs'), 'utf8');
    const block = toolBlock(src, 'list_todo_categories');
    expect(block).toMatch(/proxyGet\s*\(\s*[`'"]\/api\/todo-task-list-categories/);
    // Protocol Adapter must not call Host category APIs directly
    expect(block).not.toMatch(/invoke|tauri|list_todo_categories\s*\(/i);
  });

  it('forbids MCP category directory create/delete tools (L09#4)', () => {
    const src = readFileSync(join(repoRoot, 'packages/knowledge-mcp/index.mjs'), 'utf8');
    const names = registeredToolNames(src);
    for (const tool of FORBIDDEN_CATEGORY_CRUD_TOOLS) {
      expect(names, `forbidden category CRUD tool ${tool}`).not.toContain(tool);
    }
  });

  it('create_todo_task schema exposes optional category_id and forwards when present', () => {
    const src = readFileSync(join(repoRoot, 'packages/knowledge-mcp/index.mjs'), 'utf8');
    const block = toolBlock(src, 'create_todo_task');
    expect(block).toMatch(/category_id\s*:\s*z\.string\(\)[\s\S]*?\.optional\(\)/);
    expect(block).toMatch(/if\s*\(\s*category_id\s*!=\s*null\s*\)/);
    expect(block).toMatch(/body\.category_id\s*=\s*category_id/);
    expect(block).toContain("proxyPost('/api/todo-task-create'");
  });

  it('list_todo_tasks accepts optional category_id filter via Sidecar query', () => {
    const src = readFileSync(join(repoRoot, 'packages/knowledge-mcp/index.mjs'), 'utf8');
    const block = toolBlock(src, 'list_todo_tasks');
    expect(block).toMatch(/category_id\s*:\s*z\.string\(\)[\s\S]*?\.optional\(\)/);
    // Omit → unfiltered GET /api/todo-tasks; with id → query param
    expect(block).toMatch(/\/api\/todo-tasks/);
    expect(block).toMatch(/category_id/);
    expect(block).toMatch(/URLSearchParams|category_id=/);
  });

  it('update_todo_task optional category_id can set category alone (no dedicated set tool)', () => {
    const src = readFileSync(join(repoRoot, 'packages/knowledge-mcp/index.mjs'), 'utf8');
    const names = registeredToolNames(src);
    expect(names).not.toContain('set_todo_category');
    expect(names).not.toContain('set_todo_task_category');

    const block = toolBlock(src, 'update_todo_task');
    expect(block).toMatch(/category_id\s*:\s*z\.string\(\)[\s\S]*?\.optional\(\)/);
    expect(block).toMatch(/body\.category_id\s*=\s*category_id/);
    // Allow update with only category_id (title/todo_md may both be omitted)
    expect(block).toMatch(/category_id\s*==\s*null/);
    expect(block).toContain("proxyPost('/api/todo-task-update'");
  });

  it('Sidecar local_http exposes list-categories route and category_id on create/list/update', () => {
    const httpSrc = readFileSync(
      join(repoRoot, 'src-tauri/src/services/local_http/mod.rs'),
      'utf8',
    );
    expect(httpSrc).toMatch(/\/api\/todo-task-list-categories/);
    // create/update handlers must read category_id from JSON body
    expect(httpSrc).toMatch(/category_id/);
  });
});
