/**
 * t3 — MCP add_todo_sub optional content + new update_todo_sub
 * (tech-doc T3 / L13-T; AC1–AC3; hard constraints #4/#9).
 */
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const INDEX = 'packages/knowledge-mcp/index.mjs';

function read(rel) {
  return readFileSync(join(repoRoot, rel), 'utf8');
}

function toolBlock(src, toolName) {
  const start = src.indexOf(`registerTool(\n    '${toolName}'`);
  expect(start, `${toolName} registration missing`).toBeGreaterThanOrEqual(0);
  const next = src.indexOf('registerTool(', start + 1);
  return src.slice(start, next === -1 ? undefined : next);
}

describe('t3 MCP add_todo_sub optional content', () => {
  it('add_todo_sub schema keeps required title and adds optional content', () => {
    const block = toolBlock(read(INDEX), 'add_todo_sub');
    expect(block).toMatch(/title\s*:\s*z\.string\(\)[\s\S]*?\.min\(1\)/);
    expect(block).toMatch(/content\s*:\s*z\.string\(\)[\s\S]*?\.optional\(\)/);
    expect(block).toContain("proxyPost('/api/todo-task-add-sub'");
  });

  it('add_todo_sub title-only path still posts master_task_id + title without requiring content', () => {
    const block = toolBlock(read(INDEX), 'add_todo_sub');
    // content only attached when provided (parallel to create_todo_task todo_md)
    expect(block).toMatch(/if\s*\(\s*content\s*!=\s*null\s*\)/);
    expect(block).toMatch(/body\.content\s*=\s*content/);
  });
});

describe('t3 MCP update_todo_sub (NEW)', () => {
  it('registers update_todo_sub proxying POST /api/todo-task-update-sub', () => {
    const src = read(INDEX);
    expect(src).toMatch(/registerTool\(\s*'update_todo_sub'/);
    const block = toolBlock(src, 'update_todo_sub');
    expect(block).toContain("proxyPost('/api/todo-task-update-sub'");
  });

  it('schema: master_task_id + sub_task_id + optional title + optional content', () => {
    const block = toolBlock(read(INDEX), 'update_todo_sub');
    expect(block).toMatch(/master_task_id\s*:\s*z\.string\(\)[\s\S]*?\.min\(1\)/);
    expect(block).toMatch(/sub_task_id\s*:\s*z\.string\(\)[\s\S]*?\.min\(1\)/);
    expect(block).toMatch(/title\s*:\s*z\.string\(\)[\s\S]*?\.optional\(\)/);
    expect(block).toMatch(/content\s*:\s*z\.string\(\)[\s\S]*?\.optional\(\)/);
  });

  it('rejects when both title and content are omitted (no destructive empty-title POST)', () => {
    const block = toolBlock(read(INDEX), 'update_todo_sub');
    expect(block).toMatch(/title\s*==\s*null\s*&&\s*content\s*==\s*null/);
    expect(block).toMatch(/toolError\(\s*400/);
  });

  it('when title omitted but content provided, resolves current title before HTTP proxy', () => {
    const block = toolBlock(read(INDEX), 'update_todo_sub');
    // Must fetch master (get) to resolve current sub title — never POST empty title
    expect(block).toMatch(/\/api\/todo-task\?/);
    expect(block).toMatch(/title\s*==\s*null/);
    expect(block).toMatch(/body\.title/);
  });

  it('non-2xx from HTTP still returns toolError (isError path)', () => {
    const block = toolBlock(read(INDEX), 'update_todo_sub');
    expect(block).toMatch(/if\s*\(\s*!result\.ok\s*\)/);
    expect(block).toMatch(/toolError\(\s*result\.status,\s*result\.text\s*\)/);
  });
});

describe('t3 wire-up', () => {
  it('npm test includes todo-task-mcp-sub-content.test.js', () => {
    const pkg = JSON.parse(read('package.json'));
    expect(pkg.scripts.test).toContain('tests/todo-task-mcp-sub-content.test.js');
  });

  it('index.mjs exists for MCP surface', () => {
    expect(existsSync(join(repoRoot, INDEX))).toBe(true);
  });
});
