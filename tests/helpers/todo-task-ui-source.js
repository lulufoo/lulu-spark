import { readdirSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

/** Concatenate every JS file under frontend/src/todo-task/ for source-scan contracts. */
export function readTodoTaskUiSource() {
  const dir = join(repoRoot, 'frontend/src/todo-task');
  return readdirSync(dir)
    .filter((name) => name.endsWith('.js') || name.endsWith('.ts') || name.endsWith('.tsx'))
    .sort()
    .map((name) => readFileSync(join(dir, name), 'utf8'))
    .join('\n');
}

/** Concatenate L4 todo_task service unit tests for source-scan contracts. */
export function readTodoTaskServiceTestsSource() {
  const dir = join(repoRoot, 'src-tauri/src/unit-tests/services/todo_task');
  return readdirSync(dir)
    .filter((name) => name.endsWith('.rs'))
    .sort()
    .map((name) => readFileSync(join(dir, name), 'utf8'))
    .join('\n');
}

export const TODO_TASK_UI_FILES = [
  'frontend/src/todo-task/index.ts',
  'frontend/src/todo-task/page.tsx',
  'frontend/src/todo-task/page-render.tsx',
  'frontend/src/todo-task/page-dialogs.ts',
  'frontend/src/todo-task/page-events.ts',
  'frontend/src/todo-task/host.ts',
  'frontend/src/todo-task/format.ts',
  'frontend/src/todo-task/list.tsx',
  'frontend/src/todo-task/detail.ts',
  'frontend/src/todo-task/detail-render.tsx',
  'frontend/src/todo-task/plan-md.tsx',
  'frontend/src/todo-task/attachments.tsx',
  'frontend/src/todo-task/attachments-render.tsx',
  'frontend/src/todo-task/comments.tsx',
  'frontend/src/todo-task/dialog.tsx',
  'frontend/src/todo-task/lifecycle.ts',
  'frontend/src/todo-task/binding.ts',
];
