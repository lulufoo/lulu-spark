import { spawnSync } from 'node:child_process';
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { afterEach, describe, expect, it } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const scriptPath = join(repoRoot, 'scripts', 'migrate-plan-tasks-to-todo-tasks');
const GATE = '.migration_gate_passed';

const sandboxes = [];

function makeSandbox() {
  const base = mkdtempSync(join(tmpdir(), 'migrate-plan-to-todo-'));
  sandboxes.push(base);
  const configDir = join(base, 'config');
  const cacheDir = join(base, 'cache');
  const wbRoot = join(base, 'workbench');
  mkdirSync(configDir);
  mkdirSync(cacheDir, { recursive: true });
  mkdirSync(wbRoot, { recursive: true });
  writeFileSync(
    join(configDir, 'config.toml'),
    [
      `workbench_knowledge_root = "${wbRoot}"`,
      `knowledge_corpus_root = "${wbRoot}"`,
      `cache_dir = "${cacheDir}"`,
      '',
    ].join('\n'),
  );
  return { base, configDir, cacheDir, wbRoot };
}

function runMigrate(configDir) {
  return spawnSync(scriptPath, [], {
    cwd: repoRoot,
    env: { ...process.env, LULU_WB_CONFIG_DIR: configDir },
    encoding: 'utf8',
  });
}

function writeLegacyLayout(
  wbRoot,
  {
    taskId = 'task_abc123def456789012345678901234',
    body = '# Plan body\nkeep me\n',
    title = 'My plan for Q3',
    pathHint = null,
  } = {},
) {
  const hint = pathHint ?? `plan_tasks/tasks/${taskId}/plan.md`;
  const root = join(wbRoot, 'plan_tasks');
  const taskDir = join(root, 'tasks', taskId);
  mkdirSync(taskDir, { recursive: true });
  writeFileSync(join(taskDir, 'plan.md'), body);
  writeFileSync(join(taskDir, 'sub_tasks.json'), JSON.stringify({ sub_tasks: [] }, null, 2));
  writeFileSync(
    join(root, 'index.json'),
    JSON.stringify(
      {
        version: 2,
        tasks: {
          [taskId]: {
            master_task_id: taskId,
            title,
            status: 'incomplete',
            created_at: '2020-01-01T00:00:00Z',
            task_dir: `tasks/${taskId}`,
            legacy_ref: hint,
          },
        },
      },
      null,
      2,
    ),
  );
  writeFileSync(
    join(taskDir, 'notes.json'),
    JSON.stringify(
      {
        note: 'business plan wording stays',
        body_path: `plan_tasks/tasks/${taskId}/plan.md`,
      },
      null,
      2,
    ),
  );
  return { taskId, body, title };
}

function findJsonLog(combined, predicate) {
  for (const line of combined.split('\n')) {
    const trimmed = line.trim();
    if (!trimmed.startsWith('{')) continue;
    try {
      const obj = JSON.parse(trimmed);
      if (predicate(obj)) return obj;
    } catch {
      // keep scanning
    }
  }
  return null;
}

afterEach(() => {
  for (const dir of sandboxes.splice(0)) {
    rmSync(dir, { recursive: true, force: true });
  }
});

describe('migrate-plan-tasks-to-todo-tasks (t5 / SK-4)', () => {
  it('single entry migrates root, body file, path strings; exit 0 writes gate', () => {
    const { configDir, wbRoot } = makeSandbox();
    const { taskId, body, title } = writeLegacyLayout(wbRoot);

    const result = runMigrate(configDir);

    expect(result.status, result.stderr || result.stdout).toBe(0);
    expect(existsSync(join(wbRoot, 'plan_tasks'))).toBe(false);
    expect(existsSync(join(wbRoot, 'todo_tasks'))).toBe(true);
    expect(existsSync(join(wbRoot, 'todo_tasks', GATE))).toBe(true);

    const todoMd = join(wbRoot, 'todo_tasks', 'tasks', taskId, 'todo.md');
    expect(existsSync(todoMd)).toBe(true);
    expect(existsSync(join(wbRoot, 'todo_tasks', 'tasks', taskId, 'plan.md'))).toBe(false);
    expect(readFileSync(todoMd, 'utf8')).toBe(body);

    const index = JSON.parse(readFileSync(join(wbRoot, 'todo_tasks', 'index.json'), 'utf8'));
    expect(index.tasks[taskId].title).toBe(title);
    expect(index.tasks[taskId].legacy_ref).toBe(`todo_tasks/tasks/${taskId}/todo.md`);

    const notes = JSON.parse(
      readFileSync(join(wbRoot, 'todo_tasks', 'tasks', taskId, 'notes.json'), 'utf8'),
    );
    expect(notes.note).toBe('business plan wording stays');
    expect(notes.body_path).toBe(`todo_tasks/tasks/${taskId}/todo.md`);
  });

  it('is idempotent: already-migrated targets are skipped; re-run keeps data and gate', () => {
    const { configDir, wbRoot } = makeSandbox();
    const { taskId, body } = writeLegacyLayout(wbRoot);

    expect(runMigrate(configDir).status).toBe(0);
    const firstIndex = readFileSync(join(wbRoot, 'todo_tasks', 'index.json'), 'utf8');
    const firstBody = readFileSync(join(wbRoot, 'todo_tasks', 'tasks', taskId, 'todo.md'), 'utf8');

    const second = runMigrate(configDir);
    expect(second.status, second.stderr || second.stdout).toBe(0);
    expect(existsSync(join(wbRoot, 'todo_tasks', GATE))).toBe(true);
    expect(readFileSync(join(wbRoot, 'todo_tasks', 'index.json'), 'utf8')).toBe(firstIndex);
    expect(readFileSync(join(wbRoot, 'todo_tasks', 'tasks', taskId, 'todo.md'), 'utf8')).toBe(
      firstBody,
    );
    expect(firstBody).toBe(body);
    expect(existsSync(join(wbRoot, 'plan_tasks'))).toBe(false);
  });

  it('partial failure: non-zero exit and must not leave a passing gate marker', () => {
    const { configDir, wbRoot } = makeSandbox();
    const okId = 'task_ok000000000000000000000000001';
    const badId = 'task_bad00000000000000000000000001';
    writeLegacyLayout(wbRoot, { taskId: okId, body: 'ok body\n' });

    const badDir = join(wbRoot, 'plan_tasks', 'tasks', badId);
    mkdirSync(badDir, { recursive: true });
    writeFileSync(join(badDir, 'plan.md'), 'will fail\n');
    writeFileSync(join(badDir, 'todo.md'), 'conflict\n');

    const indexPath = join(wbRoot, 'plan_tasks', 'index.json');
    const index = JSON.parse(readFileSync(indexPath, 'utf8'));
    index.tasks[badId] = {
      master_task_id: badId,
      title: 'bad',
      status: 'incomplete',
      created_at: '2020-01-01T00:00:00Z',
      task_dir: `tasks/${badId}`,
    };
    writeFileSync(indexPath, JSON.stringify(index, null, 2));

    // Stale gate under a pre-created todo_tasks must be cleared on failure.
    mkdirSync(join(wbRoot, 'todo_tasks'), { recursive: true });
    writeFileSync(join(wbRoot, 'todo_tasks', GATE), 'stale\n');

    const result = runMigrate(configDir);
    expect(result.status).not.toBe(0);
    expect(existsSync(join(wbRoot, 'todo_tasks', GATE))).toBe(false);
  });

  it('failure logs are structured with task id and reason (not silent)', () => {
    const { configDir, wbRoot } = makeSandbox();
    const taskId = 'task_fail0000000000000000000000001';
    writeLegacyLayout(wbRoot, { taskId });
    writeFileSync(join(wbRoot, 'plan_tasks', 'tasks', taskId, 'todo.md'), 'already there\n');

    const result = runMigrate(configDir);
    expect(result.status).not.toBe(0);
    expect(existsSync(join(wbRoot, 'todo_tasks', GATE))).toBe(false);

    const combined = `${result.stdout || ''}\n${result.stderr || ''}`;
    const parsed = findJsonLog(
      combined,
      (obj) => obj.task_id === taskId || obj.taskId === taskId,
    );
    expect(parsed, `expected JSON log line with task id; got:\n${combined}`).toBeTruthy();
    expect(parsed.reason || parsed.error || parsed.message).toBeTruthy();
  });

  it('exit non-zero must not coexist with a valid .migration_gate_passed (falsifier)', () => {
    const { configDir, wbRoot } = makeSandbox();
    const taskId = 'task_falsify00000000000000000000001';
    writeLegacyLayout(wbRoot, { taskId });
    writeFileSync(join(wbRoot, 'plan_tasks', 'tasks', taskId, 'todo.md'), 'conflict\n');

    const result = runMigrate(configDir);
    const gatePresent = existsSync(join(wbRoot, 'todo_tasks', GATE));
    // Falsifier: non-zero exit AND gate present must never happen.
    expect(result.status !== 0 && gatePresent).toBe(false);
    expect(result.status).not.toBe(0);
    expect(gatePresent).toBe(false);
  });

  it('npm test includes migrate-plan-tasks-to-todo-tasks test', () => {
    const pkg = JSON.parse(readFileSync(join(repoRoot, 'package.json'), 'utf8'));
    expect(pkg.scripts.test).toMatch(/vitest run --dir tests/);
    expect(existsSync(scriptPath)).toBe(true);
  });
});
