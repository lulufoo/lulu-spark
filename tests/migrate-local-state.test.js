import { execFileSync } from 'node:child_process';
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

function runMigrate(configDir) {
  return execFileSync(scriptPath, [], {
    cwd: repoRoot,
    env: { ...process.env, LULU_WB_CONFIG_DIR: configDir },
    encoding: 'utf8',
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
    writeFileSync(join(cacheDir, 'read_later.json'), readLater);

    runMigrate(configDir);

    expect(readFileSync(join(wbRoot, 'sediment-kb', 'categories.json'), 'utf8')).toBe(categories);
    expect(readFileSync(join(wbRoot, 'sediment-kb', 'repos.json'), 'utf8')).toBe(repos);
    expect(readFileSync(join(wbRoot, 'read_later', 'read_later.json'), 'utf8')).toBe(readLater);
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

  it('I.5: is not registered as a Tauri startup hook', () => {
    const libRs = readFileSync(join(repoRoot, 'src-tauri/src/lib.rs'), 'utf8');
    expect(libRs).not.toMatch(/migrate[-_]local[-_]state/i);
    const setupBlock = libRs.match(/\.setup\(\|app\|[\s\S]*?Ok\(\(\)\)\s*\n\s*\}\)/);
    expect(setupBlock?.[0] ?? '').not.toMatch(/migrate/i);
    expect(existsSync(scriptPath)).toBe(true);
  });
});
