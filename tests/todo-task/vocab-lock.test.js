import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

const EXPECTED_TOOLS = [
  'create_todo_task',
  'update_todo_task',
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

function read(rel) {
  return readFileSync(join(repoRoot, rel), 'utf8');
}

describe('plan→todo vocab lock (tech-doc SK-0 / T1)', () => {
  it('Host MCP todo routes keep locked public names and todo_md', () => {
    const routes = read('src-tauri/src/services/mcp_protocol_adapter/routes.rs');
    for (const name of EXPECTED_TOOLS) {
      expect(routes, `missing tool ${name}`).toContain(`"${name}"`);
    }
    expect(routes).toContain('todo_md');
    expect(routes).not.toContain('complete_plan_sub');
  });

  it('HTTP prefix is /api/todo- and disk body is todo.md', () => {
    const dispatch = read('src-tauri/src/services/local_http/dispatch.rs');
    const paths = read('src-tauri/src/config/paths.rs');
    expect(dispatch).toMatch(/\/api\/todo-tasks/);
    expect(dispatch).toMatch(/\/api\/todo-task-/);
    expect(paths).toContain('.join("todo.md")');
    expect(paths).not.toContain('.join("plan.md")');
  });

  it('npm test includes todo-task vocab lock test', () => {
    const pkg = JSON.parse(read('package.json'));
    expect(pkg.scripts.test).toMatch(/vitest run --dir tests/);
  });
});
