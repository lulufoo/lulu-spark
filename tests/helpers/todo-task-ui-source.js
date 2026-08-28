import { readdirSync, readFileSync, statSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

const SOURCE_EXT = new Set(['.js', '.ts', '.tsx']);

/** Recursively collect JS/TS/TSX files under `dir`, sorted. */
function walkSourceFiles(dir) {
  const out = [];
  for (const name of readdirSync(dir).sort()) {
    const abs = join(dir, name);
    if (statSync(abs).isDirectory()) {
      out.push(...walkSourceFiles(abs));
      continue;
    }
    const ext = name.slice(name.lastIndexOf('.'));
    if (SOURCE_EXT.has(ext)) out.push(abs);
  }
  return out;
}

/** Concatenate every JS/TS/TSX file under frontend/src/todo-task/ (root + ui/commands/state). */
export function readTodoTaskUiSource() {
  const dir = join(repoRoot, 'frontend/src/todo-task');
  return walkSourceFiles(dir)
    .map((abs) => readFileSync(abs, 'utf8'))
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
  'frontend/src/todo-task/ui/page-render.tsx',
  'frontend/src/todo-task/commands/page-dialogs.ts',
  'frontend/src/todo-task/commands/page-events.ts',
  'frontend/src/todo-task/state/host.ts',
  'frontend/src/todo-task/state/format.ts',
  'frontend/src/todo-task/ui/list.tsx',
  'frontend/src/todo-task/commands/list.ts',
  'frontend/src/todo-task/commands/detail.ts',
  'frontend/src/todo-task/ui/detail.tsx',
  'frontend/src/todo-task/ui/plan-md.tsx',
  'frontend/src/todo-task/commands/plan-md.ts',
  'frontend/src/todo-task/commands/attachments.ts',
  'frontend/src/todo-task/ui/attachments.tsx',
  'frontend/src/todo-task/ui/comments.tsx',
  'frontend/src/todo-task/commands/comments.ts',
  'frontend/src/todo-task/ui/dialog.tsx',
  'frontend/src/todo-task/commands/dialog.ts',
  'frontend/src/todo-task/state/dialog.ts',
  'frontend/src/todo-task/commands/lifecycle.ts',
  'frontend/src/todo-task/commands/binding.ts',
];
