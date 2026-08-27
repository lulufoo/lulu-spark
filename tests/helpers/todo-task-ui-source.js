import { readdirSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

/** Concatenate every JS file under frontend/js/todo-task/ for source-scan contracts. */
export function readTodoTaskUiSource() {
  const dir = join(repoRoot, 'frontend/js/todo-task');
  return readdirSync(dir)
    .filter((name) => name.endsWith('.js'))
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
  'frontend/js/todo-task/index.js',
  'frontend/js/todo-task/page.js',
  'frontend/js/todo-task/host.js',
  'frontend/js/todo-task/format.js',
  'frontend/js/todo-task/list.js',
  'frontend/js/todo-task/detail.js',
  'frontend/js/todo-task/plan-md.js',
  'frontend/js/todo-task/attachments.js',
  'frontend/js/todo-task/comments.js',
  'frontend/js/todo-task/dialog.js',
  'frontend/js/todo-task/todos-lifecycle.js',
  'frontend/js/todo-task/todos-binding.js',
];
