import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

describe('OTH-212 window theme pairing', () => {
  it('opens set_theme on the default window capability', () => {
    const capability = JSON.parse(
      readFileSync(join(repoRoot, 'src-tauri/capabilities/default.json'), 'utf8'),
    );
    expect(capability.permissions).toContain('core:window:allow-set-theme');
  });

  it('applyTheme syncs the Tauri window theme', () => {
    const themeSrc = readFileSync(join(repoRoot, 'frontend/src/theme.ts'), 'utf8');
    expect(themeSrc).toMatch(/export async function syncWindowTheme/);
    expect(themeSrc).toMatch(/getCurrentWindow\(\)\.setTheme/);
    expect(themeSrc).toMatch(/void syncWindowTheme\(id\)/);
    expect(themeSrc).not.toMatch(/fetchConfig|data-panel="theme"/);
  });
});
