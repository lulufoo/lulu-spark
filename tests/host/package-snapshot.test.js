import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

function readRel(rel) {
  return readFileSync(join(repoRoot, rel), 'utf8');
}

describe('package snapshot host api', () => {
  it('exposes getPackageSnapshot through host/api and invoke', async () => {
    const api = await import('../../frontend/src/host/api.ts');
    expect(typeof api.getPackageSnapshot).toBe('function');
    const src = readRel('frontend/src/host/api/package-snapshot.ts');
    expect(src).toMatch(/invoke\('get_package_snapshot'\)/);
    expect(src).not.toMatch(/@tauri-apps\//);
    expect(src).not.toMatch(/ApiInvokeMap/);
  });

  it('pages do not import the snapshot invoke map', () => {
    const page = readRel('frontend/src/home/page.tsx');
    expect(page).not.toMatch(/ApiInvokeMap/);
  });
});
