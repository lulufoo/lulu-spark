// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { flushSync } from 'react-dom';
import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { openTodoTaskDialog, TodoTaskDialog } from '../../frontend/src/todo-task/ui/dialog.tsx';
import { TODO_TASK_BRAND_SITES } from '../fixtures/todo-task-ac15.js';
import { readRsPath } from '../helpers/read-rs-dir.js';
import { readFrontendJs } from '../helpers/read-frontend-js.js';

const fixtureRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const dialogSource = [
  readFrontendJs('frontend/src/todo-task/ui/dialog.tsx'),
  readFrontendJs('frontend/src/todo-task/commands/dialog.ts'),
  readFrontendJs('frontend/src/todo-task/state/dialog.ts'),
].join('\n');
const mcpSource = readRsPath(join(fixtureRoot, 'src-tauri/src/services/mcp_host'));

function extractCreateTodoTaskBlock(source) {
  // Host SSOT: ToolRoute entries are `route("…", …)` (T10).
  const start = source.search(/route\(\s*"create_todo_task"/);
  if (start === -1) return '';
  const rest = source.slice(start + 1);
  const endRel = rest.search(/route\(\s*"/);
  return endRel === -1 ? source.slice(start) : source.slice(start, start + 1 + endRel);
}

function mountTodoTaskDialog() {
  document.body.innerHTML = '<div id="todo-task-dialog-host"></div>';
  const root = createRoot(document.getElementById('todo-task-dialog-host'));
  flushSync(() => root.render(createElement(TodoTaskDialog)));
  return root;
}

describe('F3 copy sync — dialog.js', () => {
  let dialogRoot;

  beforeEach(() => {
    dialogRoot = mountTodoTaskDialog();
  });

  afterEach(() => {
    flushSync(() => dialogRoot?.unmount());
    document.body.innerHTML = '';
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
      const remapped = site.path
        .replace('frontend/src/todo-task/list.tsx', 'frontend/src/todo-task/ui/list.tsx')
        .replace('frontend/src/todo-task/page-render.tsx', 'frontend/src/todo-task/ui/page-render.tsx')
        .replace('frontend/src/todo-task/assistant.tsx', 'frontend/src/todo-task/ui/assistant.tsx');
      const src = remapped.startsWith('frontend/src/')
        ? readFrontendJs(remapped)
        : readFileSync(join(fixtureRoot, remapped), 'utf8');
      for (const re of site.mustMatch) {
        expect(src, `${site.path} mustMatch ${re}`).toMatch(re);
      }
      for (const re of site.mustNotMatch ?? []) {
        expect(src, `${site.path} mustNotMatch ${re}`).not.toMatch(re);
      }
    }
  });

  it('keeps technical todo-task identifiers (route/api) unchanged', () => {
    const index = readFileSync(join(fixtureRoot, 'frontend/src/todo-task/state/format.ts'), 'utf8');
    expect(index).toMatch(/todo-tasks/);
    const hub = readFrontendJs('frontend/src/home/hub.tsx');
    expect(hub).toMatch(/data-home-entry="todo-tasks"/);
  });
});
