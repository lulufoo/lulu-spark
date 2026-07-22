import { readFileSync, existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const LOCK_PATH = 'docs/features/plan-task-todos-contract/vocab-lock-confirmation.json';

const EXPECTED_TOOLS = [
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

function loadLock() {
  const abs = join(repoRoot, LOCK_PATH);
  expect(existsSync(abs), `missing ${LOCK_PATH}`).toBe(true);
  return JSON.parse(readFileSync(abs, 'utf8'));
}

describe('plan→todo vocab lock (tech-doc SK-0 / T1)', () => {
  it('records locked public contract mapping with hard-cut rules', () => {
    const lock = loadLock();
    expect(lock.version).toBe(1);
    expect(lock.feature_id).toBe('feature-20260721160725-475641f1');
    expect(lock.task_id).toBe('t1');
    expect(lock.status).toBe('locked');
    expect(lock.hard_cut).toEqual({
      dual_names: false,
      aliases: false,
      complete_plan_sub_restored: false,
    });

    expect(lock.slash).toBe('todo-task');
    expect(lock.host_module).toBe('todo_task');
    expect(lock.wire_field).toBe('todo_md');
    expect(lock.http_prefix).toBe('/api/todo-');
    expect(lock.disk).toEqual({
      root_dir: 'todo_tasks/',
      body_file: 'todo.md',
    });

    expect(lock.mcp_tools).toEqual(EXPECTED_TOOLS);
    expect(lock.mcp_tools).toContain('complete_todo');
    expect(lock.mcp_tools).not.toContain('complete_plan_sub');
    expect(lock.excluded_tools).toEqual(['complete_plan_sub']);

    expect(lock.source).toEqual({
      tech_doc_section: '公开契约映射',
      tech_doc_ref:
        'lulu-plan/revision1/tech-doc.md#公开契约映射',
    });
  });

  it('npm test includes todo-task vocab lock test', () => {
    const pkg = JSON.parse(readFileSync(join(repoRoot, 'package.json'), 'utf8'));
    expect(pkg.scripts.test).toContain('tests/todo-task-vocab-lock.test.js');
  });
});
