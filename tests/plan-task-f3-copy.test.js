// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect, beforeEach } from 'vitest';
import { openPlanTaskDialog } from '../frontend/js/plan-task/dialog.js';
import { PLAN_TASK_BRAND_SITES } from './fixtures/plan-task-ac15.js';

const fixtureRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const dialogSource = readFileSync(join(fixtureRoot, 'frontend/js/plan-task/dialog.js'), 'utf8');
const mcpSource = readFileSync(join(fixtureRoot, 'packages/knowledge-mcp/index.mjs'), 'utf8');

function extractCreatePlanTaskBlock(source) {
  const marker = "server.registerTool(\n    'create_plan_task'";
  const start = source.indexOf(marker);
  if (start === -1) return '';
  const end = source.indexOf("server.registerTool(\n    'list_plan_tasks'", start);
  return end === -1 ? source.slice(start) : source.slice(start, end);
}

function seedPlanTaskDialogDom() {
  document.body.innerHTML = `
    <div id="plan-task-dialog">
      <h3 id="plan-task-dialog-title"></h3>
      <div id="plan-task-dialog-body"></div>
      <p id="plan-task-dialog-error" hidden></p>
      <button type="button" id="plan-task-dialog-cancel"></button>
      <button type="button" id="plan-task-dialog-primary"></button>
    </div>
  `;
}

describe('F3 copy sync — dialog.js', () => {
  beforeEach(() => {
    seedPlanTaskDialogDom();
  });

  it('does not mention implicit default sub task hint in source', () => {
    expect(dialogSource).not.toMatch(/留空则创建默认子任务/);
    expect(dialogSource).not.toMatch(/默认子任务/);
  });

  it('create-master dialog hint states empty sub list semantics', () => {
    openPlanTaskDialog({ type: 'create-master', onSubmit: async () => {} });
    const body = document.getElementById('plan-task-dialog-body');
    const hint = body?.querySelector('.plan-task-dialog-hint')?.textContent?.trim() ?? '';
    expect(hint).not.toMatch(/默认子任务/);
    expect(hint).toMatch(/skip sub-tasks|no sub-tasks|will have none/i);
  });
});

describe('F3 copy sync — create_plan_task MCP description', () => {
  const createBlock = extractCreatePlanTaskBlock(mcpSource);

  it('does not describe omit/empty as implicit default sub', () => {
    expect(createBlock).not.toMatch(/implicit sub/i);
    expect(createBlock).not.toMatch(/one implicit sub/i);
  });

  it('describes title and todo_md without sub_titles', () => {
    expect(createBlock).toMatch(/todo_md/);
    expect(createBlock).toMatch(/max 20/i);
    expect(createBlock).not.toMatch(/sub_titles/);
    expect(createBlock).toMatch(/empty sub_tasks/i);
  });
});

describe('AC7 brand copy — 7 user-visible Todos sites (tech-doc T14/T15)', () => {
  it('locks all 7 brand sites to Todos without residual 计划任务 user copy', () => {
    expect(PLAN_TASK_BRAND_SITES).toHaveLength(7);
    for (const site of PLAN_TASK_BRAND_SITES) {
      const src = readFileSync(join(fixtureRoot, site.path), 'utf8');
      for (const re of site.mustMatch) {
        expect(src, `${site.path} mustMatch ${re}`).toMatch(re);
      }
      for (const re of site.mustNotMatch ?? []) {
        expect(src, `${site.path} mustNotMatch ${re}`).not.toMatch(re);
      }
    }
  });

  it('keeps technical plan-task identifiers (route/api) unchanged', () => {
    const index = readFileSync(join(fixtureRoot, 'frontend/js/plan-task/index.js'), 'utf8');
    expect(index).toMatch(/plan-tasks/);
    const hub = readFileSync(join(fixtureRoot, 'frontend/js/components/home-hub.js'), 'utf8');
    expect(hub).toMatch(/data-home-entry="plan-tasks"/);
  });
});
