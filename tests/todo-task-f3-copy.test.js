// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect, beforeEach } from 'vitest';
import { openTodoTaskDialog } from '../frontend/js/todo-task/dialog.js';
import { TODO_TASK_BRAND_SITES } from './fixtures/todo-task-ac15.js';

const fixtureRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const dialogSource = readFileSync(join(fixtureRoot, 'frontend/js/todo-task/dialog.js'), 'utf8');
const mcpSource = readFileSync(join(fixtureRoot, 'src-tauri/src/services/mcp_protocol_adapter.rs'), 'utf8');

function extractCreateTodoTaskBlock(source) {
  // Host SSOT: ToolRoute entries are `route("…", …)` (T10).
  const start = source.search(/route\(\s*"create_todo_task"/);
  if (start === -1) return '';
  const rest = source.slice(start + 1);
  const endRel = rest.search(/route\(\s*"/);
  return endRel === -1 ? source.slice(start) : source.slice(start, start + 1 + endRel);
}

function seedTodoTaskDialogDom() {
  document.body.innerHTML = `
    <div id="todo-task-dialog">
      <h3 id="todo-task-dialog-title"></h3>
      <div id="todo-task-dialog-body"></div>
      <p id="todo-task-dialog-error" hidden></p>
      <button type="button" id="todo-task-dialog-cancel"></button>
      <button type="button" id="todo-task-dialog-primary"></button>
    </div>
  `;
}

describe('F3 copy sync — dialog.js', () => {
  beforeEach(() => {
    seedTodoTaskDialogDom();
  });

  it('does not mention implicit default sub task hint in source', () => {
    expect(dialogSource).not.toMatch(/留空则创建默认子任务/);
    expect(dialogSource).not.toMatch(/默认子任务/);
  });

  it('create-master dialog hint states empty sub list semantics', () => {
    openTodoTaskDialog({ type: 'create-master', onSubmit: async () => {} });
    const body = document.getElementById('todo-task-dialog-body');
    const hint = body?.querySelector('.todo-task-dialog-hint')?.textContent?.trim() ?? '';
    expect(hint).not.toMatch(/默认子任务/);
    expect(hint).toMatch(/skip sub-tasks|no sub-tasks|will have none/i);
  });

  it('create-category dialog uses English copy and name field', () => {
    openTodoTaskDialog({ type: 'create-category', onSubmit: async () => {} });
    expect(document.getElementById('todo-task-dialog-title')?.textContent).toBe('New category');
    expect(document.getElementById('todo-task-dialog-primary')?.textContent).toBe(
      'Create category',
    );
    const body = document.getElementById('todo-task-dialog-body');
    expect(body?.textContent).toMatch(/Organize todos under a shared category/i);
    expect(body?.querySelector('[data-field="name"]')).not.toBeNull();
    expect(body?.textContent).not.toMatch(/[\u4e00-\u9fff]/);
  });
});

describe('F3 copy sync — create_todo_task MCP description', () => {
  const createBlock = extractCreateTodoTaskBlock(mcpSource);

  it('does not describe omit/empty as implicit default sub', () => {
    expect(createBlock).not.toMatch(/implicit sub/i);
    expect(createBlock).not.toMatch(/one implicit sub/i);
  });

  it('Host create_todo_task routes to Sidecar without plan_* residue', () => {
    expect(createBlock).toContain('/api/todo-task-create');
    expect(createBlock).not.toMatch(/implicit sub|one implicit sub/i);
    expect(mcpSource).not.toMatch(/route\(\s*"create_plan_task"/);
    // Delivery e2e still documents todo_md + empty sub_tasks contract for Host URL.
    const e2e = readFileSync(join(fixtureRoot, 'scripts/todo-task-mcp-e2e.mjs'), 'utf8');
    expect(e2e).toMatch(/todo_md/);
    expect(e2e).toMatch(/empty sub_tasks|sub_tasks/);
  });
});

describe('AC7 brand copy — user-visible Todos sites (tech-doc T14/T15)', () => {
  it('locks remaining brand sites to Todos without residual 计划任务 user copy', () => {
    expect(TODO_TASK_BRAND_SITES).toHaveLength(6);
    for (const site of TODO_TASK_BRAND_SITES) {
      const src = readFileSync(join(fixtureRoot, site.path), 'utf8');
      for (const re of site.mustMatch) {
        expect(src, `${site.path} mustMatch ${re}`).toMatch(re);
      }
      for (const re of site.mustNotMatch ?? []) {
        expect(src, `${site.path} mustNotMatch ${re}`).not.toMatch(re);
      }
    }
  });

  it('keeps technical todo-task identifiers (route/api) unchanged', () => {
    const index = readFileSync(join(fixtureRoot, 'frontend/js/todo-task/index.js'), 'utf8');
    expect(index).toMatch(/todo-tasks/);
    const hub = readFileSync(join(fixtureRoot, 'frontend/js/components/home-hub.js'), 'utf8');
    expect(hub).toMatch(/data-home-entry="todo-tasks"/);
  });
});
