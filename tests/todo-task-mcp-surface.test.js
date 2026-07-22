import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..');

/** tech-doc 公开契约映射 — 12 todo_* MCP tools; no complete_plan_sub. */
const EXPECTED_TODO_TOOLS = [
  'create_todo_task',
  'list_todo_tasks',
  'get_todo_task',
  'delete_todo_task',
  'add_todo_sub',
  'delete_todo_sub',
  'complete_todo',
  'link_todo_archive',
  'add_todo_attachment',
  'list_todo_attachments',
  'get_todo_attachment',
  'update_todo_attachment',
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

function registeredToolNames(src) {
  return [...src.matchAll(/registerTool\(\s*['"]([^'"]+)['"]/g)].map((m) => m[1]);
}

describe('MCP tool surface hard-cut to todo_* (tech-doc T7 / SK-2)', () => {
  it('index.mjs registers exactly the 12 todo_* tools; old plan_* names absent', () => {
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
