import { execFileSync, spawnSync } from 'node:child_process';
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  statSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { afterEach, describe, expect, it } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const scriptPath = join(repoRoot, 'scripts', 'migrate-local-state');

const sandboxes = [];

function makeSandbox() {
  const base = mkdtempSync(join(tmpdir(), 'migrate-local-state-'));
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
  return { configDir, cacheDir, wbRoot };
}

function runMigrate(configDir, { captureStderr = false } = {}) {
  const env = { ...process.env, LULU_WB_CONFIG_DIR: configDir };
  if (captureStderr) {
    const { stderr } = spawnSync(scriptPath, [], {
      cwd: repoRoot,
      env,
      encoding: 'utf8',
    });
    return stderr;
  }
  return execFileSync(scriptPath, [], {
    cwd: repoRoot,
    env,
    encoding: 'utf8',
  });
}

function samplePlanTasks() {
  return JSON.stringify({
    version: 1,
    tasks: {
      abc123def45678901234567890123456: {
        title: 'Sample plan task',
        status: 'incomplete',
        sub_tasks: [],
      },
    },
  });
}

afterEach(() => {
  sandboxes.length = 0;
});

describe('migrate-local-state', () => {
  it('migrates sediment-kb and read_later from cache_dir to workbench_knowledge_root', () => {
    const { configDir, cacheDir, wbRoot } = makeSandbox();
    mkdirSync(join(cacheDir, 'sediment-kb'), { recursive: true });
    const categories = JSON.stringify({ version: 1, categories: [{ id: 'c1', name: 'Cat' }] });
    const repos = JSON.stringify({
      version: 1,
      repos: [{ full_name: 'o/r', category_id: 'c1' }],
    });
    const readLater = JSON.stringify({
      version: 1,
      entries: [
        {
          id: 'abc',
          url: 'https://example.com',
          title: 'Example',
          saved_at: '2020-01-01T00:00:00Z',
          read: false,
        },
      ],
    });
    writeFileSync(join(cacheDir, 'sediment-kb', 'categories.json'), categories);
    writeFileSync(join(cacheDir, 'sediment-kb', 'repos.json'), repos);
    const planTasks = JSON.stringify({
      version: 1,
      tasks: [{ id: 'pt1', title: 'Plan task', status: 'open' }],
    });
    writeFileSync(join(cacheDir, 'read_later.json'), readLater);
    writeFileSync(join(cacheDir, 'plan_tasks.json'), planTasks);

    runMigrate(configDir);

    expect(readFileSync(join(wbRoot, 'sediment-kb', 'categories.json'), 'utf8')).toBe(categories);
    expect(readFileSync(join(wbRoot, 'sediment-kb', 'repos.json'), 'utf8')).toBe(repos);
    expect(readFileSync(join(wbRoot, 'read_later', 'read_later.json'), 'utf8')).toBe(readLater);
    expect(readFileSync(join(wbRoot, 'plan_tasks', 'plan_tasks.json'), 'utf8')).toBe(planTasks);
  });

  it('plan_tasks: copied when source exists and target is absent (creates plan_tasks/)', () => {
    const { configDir, cacheDir, wbRoot } = makeSandbox();
    const planTasks = JSON.stringify({ version: 1, tasks: [{ id: 'copied', title: 'New' }] });
    writeFileSync(join(cacheDir, 'plan_tasks.json'), planTasks);

    runMigrate(configDir);

    expect(readFileSync(join(wbRoot, 'plan_tasks', 'plan_tasks.json'), 'utf8')).toBe(planTasks);
  });

  it('plan_tasks: skip_same when source and target have identical content', () => {
    const { configDir, cacheDir, wbRoot } = makeSandbox();
    const planTasks = JSON.stringify({ version: 1, tasks: [] });
    writeFileSync(join(cacheDir, 'plan_tasks.json'), planTasks);
    mkdirSync(join(wbRoot, 'plan_tasks'), { recursive: true });
    writeFileSync(join(wbRoot, 'plan_tasks', 'plan_tasks.json'), planTasks);
    const mtimeBefore = statSync(join(wbRoot, 'plan_tasks', 'plan_tasks.json')).mtimeMs;

    runMigrate(configDir);

    expect(statSync(join(wbRoot, 'plan_tasks', 'plan_tasks.json')).mtimeMs).toBe(mtimeBefore);
  });

  it('plan_tasks: skip_diff when target exists with different content (does not overwrite)', () => {
    const { configDir, cacheDir, wbRoot } = makeSandbox();
    writeFileSync(
      join(cacheDir, 'plan_tasks.json'),
      JSON.stringify({ version: 1, tasks: [{ id: 'cache', title: 'Cache' }] }),
    );
    mkdirSync(join(wbRoot, 'plan_tasks'), { recursive: true });
    const existing = JSON.stringify({ version: 1, tasks: [{ id: 'corpus', title: 'Corpus' }] });
    writeFileSync(join(wbRoot, 'plan_tasks', 'plan_tasks.json'), existing);

    runMigrate(configDir);

    expect(readFileSync(join(wbRoot, 'plan_tasks', 'plan_tasks.json'), 'utf8')).toBe(existing);
  });

  it('plan_tasks: missing_source when cache plan_tasks.json is absent (script continues)', () => {
    const { configDir, wbRoot } = makeSandbox();
    expect(() => runMigrate(configDir)).not.toThrow();
    expect(existsSync(join(wbRoot, 'plan_tasks', 'plan_tasks.json'))).toBe(false);
  });

  it('is idempotent: second run does not rewrite unchanged targets', () => {
    const { configDir, cacheDir, wbRoot } = makeSandbox();
    mkdirSync(join(cacheDir, 'sediment-kb'), { recursive: true });
    const categories = JSON.stringify({ version: 1, categories: [] });
    writeFileSync(join(cacheDir, 'sediment-kb', 'categories.json'), categories);

    runMigrate(configDir);
    const target = join(wbRoot, 'sediment-kb', 'categories.json');
    const mtimeAfterFirst = statSync(target).mtimeMs;

    runMigrate(configDir);
    expect(statSync(target).mtimeMs).toBe(mtimeAfterFirst);
    expect(readFileSync(target, 'utf8')).toBe(categories);
  });

  it('skips when target already exists with identical content', () => {
    const { configDir, cacheDir, wbRoot } = makeSandbox();
    mkdirSync(join(cacheDir, 'sediment-kb'), { recursive: true });
    mkdirSync(join(wbRoot, 'sediment-kb'), { recursive: true });
    const repos = JSON.stringify({ version: 1, repos: [] });
    writeFileSync(join(cacheDir, 'sediment-kb', 'repos.json'), repos);
    writeFileSync(join(wbRoot, 'sediment-kb', 'repos.json'), repos);
    const mtimeBefore = statSync(join(wbRoot, 'sediment-kb', 'repos.json')).mtimeMs;

    runMigrate(configDir);

    expect(statSync(join(wbRoot, 'sediment-kb', 'repos.json')).mtimeMs).toBe(mtimeBefore);
  });

  it('does not destructively overwrite when target content differs', () => {
    const { configDir, cacheDir, wbRoot } = makeSandbox();
    mkdirSync(join(cacheDir, 'sediment-kb'), { recursive: true });
    mkdirSync(join(wbRoot, 'sediment-kb'), { recursive: true });
    writeFileSync(
      join(cacheDir, 'sediment-kb', 'categories.json'),
      JSON.stringify({ version: 1, categories: [{ id: 'new', name: 'New' }] }),
    );
    const existing = JSON.stringify({ version: 1, categories: [{ id: 'keep', name: 'Keep' }] });
    writeFileSync(join(wbRoot, 'sediment-kb', 'categories.json'), existing);

    runMigrate(configDir);

    expect(readFileSync(join(wbRoot, 'sediment-kb', 'categories.json'), 'utf8')).toBe(existing);
  });

  it('exits gracefully when cache sources are absent', () => {
    const { configDir } = makeSandbox();
    expect(() => runMigrate(configDir)).not.toThrow();
  });

  describe('plan_tasks migration (cache plan_tasks.json → corpus plan_tasks/plan_tasks.json)', () => {
    it('copied: copies source when target is absent and creates plan_tasks/', () => {
      const { configDir, cacheDir, wbRoot } = makeSandbox();
      const planTasks = samplePlanTasks();
      writeFileSync(join(cacheDir, 'plan_tasks.json'), planTasks);
      expect(existsSync(join(wbRoot, 'plan_tasks', 'plan_tasks.json'))).toBe(false);

      runMigrate(configDir);

      expect(readFileSync(join(wbRoot, 'plan_tasks', 'plan_tasks.json'), 'utf8')).toBe(planTasks);
    });

    it('skip_same: leaves target unchanged when content matches', () => {
      const { configDir, cacheDir, wbRoot } = makeSandbox();
      const planTasks = samplePlanTasks();
      writeFileSync(join(cacheDir, 'plan_tasks.json'), planTasks);
      mkdirSync(join(wbRoot, 'plan_tasks'), { recursive: true });
      writeFileSync(join(wbRoot, 'plan_tasks', 'plan_tasks.json'), planTasks);
      const mtimeBefore = statSync(join(wbRoot, 'plan_tasks', 'plan_tasks.json')).mtimeMs;

      runMigrate(configDir);

      expect(statSync(join(wbRoot, 'plan_tasks', 'plan_tasks.json')).mtimeMs).toBe(mtimeBefore);
      expect(readFileSync(join(wbRoot, 'plan_tasks', 'plan_tasks.json'), 'utf8')).toBe(planTasks);
    });

    it('skip_diff: preserves corpus file when content differs', () => {
      const { configDir, cacheDir, wbRoot } = makeSandbox();
      writeFileSync(join(cacheDir, 'plan_tasks.json'), samplePlanTasks());
      mkdirSync(join(wbRoot, 'plan_tasks'), { recursive: true });
      const existing = JSON.stringify({ version: 1, tasks: {} });
      writeFileSync(join(wbRoot, 'plan_tasks', 'plan_tasks.json'), existing);

      const stderr = runMigrate(configDir, { captureStderr: true });

      expect(readFileSync(join(wbRoot, 'plan_tasks', 'plan_tasks.json'), 'utf8')).toBe(existing);
      expect(stderr).toMatch(/skip \(target differs\)/);
    });

    it('missing_source: continues without error when cache file is absent', () => {
      const { configDir, wbRoot } = makeSandbox();
      expect(existsSync(join(wbRoot, 'plan_tasks', 'plan_tasks.json'))).toBe(false);

      expect(() => runMigrate(configDir)).not.toThrow();
      expect(existsSync(join(wbRoot, 'plan_tasks', 'plan_tasks.json'))).toBe(false);
    });
  });

  it('I.5: is not registered as a Tauri startup hook', () => {
    const libRs = readFileSync(join(repoRoot, 'src-tauri/src/lib.rs'), 'utf8');
    expect(libRs).not.toMatch(/migrate[-_]local[-_]state/i);
    const setupBlock = libRs.match(/\.setup\(\|app\|[\s\S]*?Ok\(\(\)\)\s*\n\s*\}\)/);
    expect(setupBlock?.[0] ?? '').not.toMatch(/migrate/i);
    expect(existsSync(scriptPath)).toBe(true);
  });
});
