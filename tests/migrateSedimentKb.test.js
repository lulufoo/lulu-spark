import { mkdtempSync, readFileSync, writeFileSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const repoRoot = fileURLToPath(new URL('..', import.meta.url));
const scriptPath = join(repoRoot, 'scripts', 'migrate_sediment_kb.py');

function runMigrate(cacheDir, extraArgs = []) {
  return spawnSync('python3', [scriptPath, '--cache-dir', cacheDir, ...extraArgs], {
    cwd: repoRoot,
    encoding: 'utf8',
  });
}

function writeRepoList(cacheDir, repos) {
  writeFileSync(
    join(cacheDir, 'repo-list.json'),
    JSON.stringify({ repos }, null, 2),
    'utf8',
  );
}

function mkCacheDir() {
  return mkdtempSync(join(tmpdir(), 'sediment-kb-migrate-'));
}

describe('migrate_sediment_kb.py', () => {
  it('reads KNOWLEDGE_CORPUS from repo-list.json and writes sediment-kb JSON', () => {
    const cacheDir = mkCacheDir();
    writeRepoList(cacheDir, [
      { full_name: 'acme/kb-a', type: 'KNOWLEDGE_CORPUS', description: 'desc a' },
      { full_name: 'acme/other', type: 'WORKBENCH_KNOWLEDGE', description: 'skip' },
    ]);

    const result = runMigrate(cacheDir);
    expect(result.status).toBe(0);

    const sedimentDir = join(cacheDir, 'sediment-kb');
    const categories = JSON.parse(
      readFileSync(join(sedimentDir, 'categories.json'), 'utf8'),
    );
    const repos = JSON.parse(readFileSync(join(sedimentDir, 'repos.json'), 'utf8'));

    expect(categories).toEqual({
      version: 1,
      categories: [{ id: 'uncategorized', name: '未分类' }],
    });
    expect(repos).toEqual({
      version: 1,
      repos: [
        {
          full_name: 'acme/kb-a',
          description: 'desc a',
          category_id: 'uncategorized',
        },
      ],
    });
  });

  it('--dry-run prints plan without writing sediment-kb', () => {
    const cacheDir = mkCacheDir();
    writeRepoList(cacheDir, [
      { full_name: 'acme/kb-a', type: 'KNOWLEDGE_CORPUS', description: 'desc a' },
    ]);

    const result = runMigrate(cacheDir, ['--dry-run']);
    expect(result.status).toBe(0);
    expect(result.stdout + result.stderr).toMatch(/acme\/kb-a/);
    expect(existsSync(join(cacheDir, 'sediment-kb'))).toBe(false);
  });

  it('skips when sediment-kb already exists (idempotent)', () => {
    const cacheDir = mkCacheDir();
    writeRepoList(cacheDir, [
      { full_name: 'acme/kb-a', type: 'KNOWLEDGE_CORPUS', description: 'desc a' },
    ]);

    const first = runMigrate(cacheDir);
    expect(first.status).toBe(0);

    writeRepoList(cacheDir, [
      { full_name: 'acme/kb-b', type: 'KNOWLEDGE_CORPUS', description: 'desc b' },
    ]);

    const second = runMigrate(cacheDir);
    expect(second.status).toBe(0);
    expect(second.stdout + second.stderr).toMatch(/skip/i);

    const repos = JSON.parse(
      readFileSync(join(cacheDir, 'sediment-kb', 'repos.json'), 'utf8'),
    );
    expect(repos.repos).toEqual([
      {
        full_name: 'acme/kb-a',
        description: 'desc a',
        category_id: 'uncategorized',
      },
    ]);
  });

  it('--force overwrites existing sediment-kb', () => {
    const cacheDir = mkCacheDir();
    writeRepoList(cacheDir, [
      { full_name: 'acme/kb-a', type: 'KNOWLEDGE_CORPUS', description: 'desc a' },
    ]);
    runMigrate(cacheDir);

    writeRepoList(cacheDir, [
      { full_name: 'acme/kb-b', type: 'KNOWLEDGE_CORPUS', description: 'desc b' },
    ]);

    const result = runMigrate(cacheDir, ['--force']);
    expect(result.status).toBe(0);

    const repos = JSON.parse(
      readFileSync(join(cacheDir, 'sediment-kb', 'repos.json'), 'utf8'),
    );
    expect(repos.repos).toEqual([
      {
        full_name: 'acme/kb-b',
        description: 'desc b',
        category_id: 'uncategorized',
      },
    ]);
  });

  it('exits with error when repo-list.json is missing', () => {
    const cacheDir = mkCacheDir();
    const result = runMigrate(cacheDir);
    expect(result.status).not.toBe(0);
    expect(result.stderr).toMatch(/repo-list\.json/i);
  });

  it('exits with error on invalid JSON', () => {
    const cacheDir = mkCacheDir();
    writeFileSync(join(cacheDir, 'repo-list.json'), '{ not json', 'utf8');
    const result = runMigrate(cacheDir);
    expect(result.status).not.toBe(0);
    expect(result.stderr).toMatch(/invalid json/i);
  });
});
