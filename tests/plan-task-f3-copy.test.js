// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect, beforeEach } from 'vitest';
import { openPlanTaskDialog } from '../frontend/js/plan-task/dialog.js';

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
    expect(hint).toMatch(/不含.*子任务|无子任务/);
  });
});

describe('F3 copy sync — create_plan_task MCP description', () => {
  const createBlock = extractCreatePlanTaskBlock(mcpSource);

  it('does not describe omit/empty as implicit default sub', () => {
    expect(createBlock).not.toMatch(/implicit sub/i);
    expect(createBlock).not.toMatch(/one implicit sub/i);
  });

  it('describes title and plan_md without sub_titles', () => {
    expect(createBlock).toMatch(/plan_md/);
    expect(createBlock).toMatch(/max 20/i);
    expect(createBlock).not.toMatch(/sub_titles/);
    expect(createBlock).toMatch(/empty sub_tasks/i);
  });
});
