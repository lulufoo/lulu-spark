import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

function read(rel) {
  return readFileSync(join(repoRoot, rel), 'utf8');
}

function tokenNames(css) {
  return [...css.matchAll(/--((?:bg|fg|border|shadow|color)-[a-z0-9-]+)\s*:/g)].map(
    (match) => match[1],
  );
}

const lightCss = read('frontend/theme-light.css');
const darkCss = read('frontend/theme-dark.css');
const themeSrc = read('frontend/src/theme.ts');
const appCss = read('frontend/app.css');

describe('OTH-211 Dark theme tokens', () => {
  it('defines the same token names as Light on a dark canvas', () => {
    expect(darkCss).toMatch(/:root\s*\{/);
    expect(darkCss).toMatch(/color-scheme:\s*dark;/);
    expect(darkCss).toMatch(/--bg-canvas:\s*#0d1117;/);
    expect(darkCss).toMatch(/--fg-default:\s*#e6edf3;/);
    expect(darkCss).toMatch(/--bg-surface:\s*#161b22;/);
    expect(tokenNames(darkCss)).toEqual(tokenNames(lightCss));
  });

  it('lets the loader swap to theme-dark.css', () => {
    expect(themeSrc).toMatch(/dark:\s*'theme-dark\.css'/);
    expect(themeSrc).toMatch(/id === 'dark' \? 'dark'/);
    expect(themeSrc).toMatch(/readRequestedTheme/);
  });

  it('does not leave the Light muted chevron hex in app.css', () => {
    expect(appCss).not.toMatch(/stroke='%2357606a'/);
  });
});
