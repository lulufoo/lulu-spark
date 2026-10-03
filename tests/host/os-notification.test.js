import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

function readRel(rel) {
  return readFileSync(join(repoRoot, rel), 'utf8');
}

describe('os-notification host surface', () => {
  it('exposes showOsNotification through host/api and writePost', () => {
    const apiBarrel = readRel('frontend/src/host/api.ts');
    const apiMod = readRel('frontend/src/host/api/os-notification.ts');
    expect(apiBarrel).toMatch(/showOsNotification/);
    expect(apiBarrel).not.toMatch(/writeApiInvokeMap/);
    expect(apiMod).toMatch(/writePost\('\/api\/show-os-notification'/);
    expect(apiMod).not.toMatch(/writeApiInvokeMap/);
  });
});
