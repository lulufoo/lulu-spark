import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

function read(rel) {
  return readFileSync(join(repoRoot, rel), 'utf8');
}

const themeSrc = read('frontend/src/theme.ts');
const mainSrc = read('frontend/src/main.tsx');
const indexHtml = read('frontend/index.html');

describe('OTH-210 theme loader gates', () => {
  it('keeps theme-light.css on #theme-sheet ahead of app.css', () => {
    expect(indexHtml).toMatch(/<html[^>]*\bdata-theme="light"/);
    expect(indexHtml).toMatch(
      /<link rel="stylesheet" id="theme-sheet" href="theme-light.css"/,
    );
    const themeAt = indexHtml.indexOf('id="theme-sheet"');
    const appAt = indexHtml.indexOf('href="app.css"');
    expect(themeAt).toBeGreaterThan(-1);
    expect(appAt).toBeGreaterThan(themeAt);
  });

  it('boots through applyTheme and keeps theme ids stable', () => {
    expect(mainSrc).toMatch(/bootTheme\(\)/);
    expect(themeSrc).toMatch(/export function applyTheme/);
    expect(themeSrc).toMatch(/'light',\s*'dark',\s*'system'/);
    expect(themeSrc).not.toMatch(/localStorage/);
  });
});
