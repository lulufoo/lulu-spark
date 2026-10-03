import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { workbenchSkillsContent } from '../../frontend/src/app-shell/state/skills-content.ts';
import { getBaselineEntries } from '../../frontend/src/home-entry-shell/entry-config.ts';
import { READ_API_INVOKE_MAP, resolveInvokeFromPath } from '../../frontend/src/host/readApiInvokeMap.ts';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

function read(rel) {
  return readFileSync(join(repoRoot, rel), 'utf8');
}

const ASSEMBLY_FILES = [
  'frontend/src/boot.ts',
  'frontend/src/shell.tsx',
  'frontend/src/shell-pages.tsx',
  'frontend/src/router/index.ts',
  'frontend/src/home/page.tsx',
  'frontend/src/home-entry-shell/entry-config.ts',
  'frontend/src/host/readApiInvokeMap.ts',
  'frontend/src/home/commands/channel-unread.ts',
  'frontend/src/home/state/store.ts',
  'frontend/src/app-shell/state/skills-content.ts',
];

describe('t2 desktop todo surface removed', () => {
  it('frontend/src/todo-task directory is gone', () => {
    expect(existsSync(join(repoRoot, 'frontend/src/todo-task'))).toBe(false);
    expect(existsSync(join(repoRoot, 'tests/todo-task'))).toBe(false);
  });

  it('assembly files no longer reference todo UI, route, or todos unread channel', () => {
    for (const rel of ASSEMBLY_FILES) {
      const src = read(rel);
      expect(src, rel).not.toMatch(/todo-task|todo-tasks|TodoTask|TodoTasks/);
      expect(src, rel).not.toMatch(/['"]todos['"]/);
    }
    expect(read('frontend/src/boot.ts')).not.toMatch(
      /createTodoTaskContentAdapter|mountTodoTasksRoute/,
    );
    expect(read('frontend/src/shell.tsx')).not.toMatch(/TodoTaskDialog/);
    expect(read('frontend/src/shell-pages.tsx')).not.toMatch(
      /TodoTasksPage|todo-tasks-view/,
    );
    expect(read('frontend/src/home/page.tsx')).not.toMatch(
      /data-home-entry="todo-tasks"/,
    );
    expect(read('frontend/src/home-entry-shell/entry-config.ts')).not.toMatch(
      /key:\s*['"]todo-task['"]/,
    );
    expect(READ_API_INVOKE_MAP['/api/todo-tasks']).toBeUndefined();
    expect(resolveInvokeFromPath('/api/todo-tasks')).toBeNull();
  });

  it('skills-content no longer points at the todo-task assistant skill', () => {
    const cmds = workbenchSkillsContent.groups.flatMap((g) =>
      g.items.map((i) => i.cmd),
    );
    expect(cmds).not.toContain('todo-task');
    expect(JSON.stringify(workbenchSkillsContent)).not.toContain('todo-task');
  });

  it('notes and knowledge entry surfaces remain', () => {
    const keys = getBaselineEntries().map((e) => e.contentKey);
    expect(keys).toContain('notes');
    expect(keys).toContain('read-later');
    expect(keys).toContain('builders');
    expect(keys).not.toContain('todo-task');
    expect(read('frontend/src/home/page.tsx')).toMatch(
      /data-home-entry="spark"/,
    );
    expect(read('frontend/src/home/page.tsx')).toMatch(
      /data-home-entry="knowledge"/,
    );
    expect(read('frontend/src/boot.ts')).not.toMatch(/createNotesContentAdapter/);
  });

  it('workbench binding lives outside the deleted todo-task module', () => {
    expect(
      existsSync(join(repoRoot, 'frontend/src/app-shell/commands/workbench-binding.ts')),
    ).toBe(true);
    expect(read('frontend/src/boot.ts')).toMatch(
      /from ['"]\.\/app-shell\/commands\/workbench-binding\.ts['"]/,
    );
  });
});
