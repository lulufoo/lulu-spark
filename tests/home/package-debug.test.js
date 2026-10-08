import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

const getPackageSnapshotMock = vi.hoisted(() => vi.fn());

vi.mock('../../frontend/src/host/api.ts', () => ({
  getPackageSnapshot: (...args) => getPackageSnapshotMock(...args),
}));

function readRel(rel) {
  return readFileSync(join(repoRoot, rel), 'utf8');
}

describe('package debug · source', () => {
  it('Home page loads is_debug through commands and paints AccountBar', () => {
    const page = readRel('frontend/src/home/page.tsx');
    expect(page).toMatch(/loadPackageDebug/);
    expect(page).toMatch(/isDebug=\{isDebug\}/);
    expect(page).not.toMatch(/getPackageSnapshot/);
    expect(page).not.toMatch(/ApiInvokeMap/);
  });

  it('command reads host/api and does not paint', () => {
    const src = readRel('frontend/src/home/commands/package-debug.ts');
    expect(src).toMatch(/getPackageSnapshot/);
    expect(src).not.toMatch(/className|background/);
  });
});

describe('loadPackageDebug', () => {
  beforeEach(async () => {
    getPackageSnapshotMock.mockReset();
    const { packageDebugStore } = await import(
      '../../frontend/src/home/state/package-debug.ts'
    );
    packageDebugStore.set(false);
  });

  it('stores is_debug from the snapshot', async () => {
    getPackageSnapshotMock.mockResolvedValue({
      is_debug: true,
      version: 'Beta v0.1',
      product_name: 'Lulu Spark',
    });
    const { loadPackageDebug } = await import(
      '../../frontend/src/home/commands/package-debug.ts'
    );
    const { packageDebugStore } = await import(
      '../../frontend/src/home/state/package-debug.ts'
    );
    await loadPackageDebug();
    expect(packageDebugStore.getSnapshot()).toBe(true);
  });

  it('stays false when invoke fails', async () => {
    getPackageSnapshotMock.mockRejectedValue(new Error('no host'));
    const { loadPackageDebug } = await import(
      '../../frontend/src/home/commands/package-debug.ts'
    );
    const { packageDebugStore } = await import(
      '../../frontend/src/home/state/package-debug.ts'
    );
    packageDebugStore.set(true);
    await loadPackageDebug();
    expect(packageDebugStore.getSnapshot()).toBe(false);
  });
});
