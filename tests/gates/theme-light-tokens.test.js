import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const indexHtml = readFileSync(join(repoRoot, 'frontend/index.html'), 'utf8');
const themeCss = readFileSync(join(repoRoot, 'frontend/theme-light.css'), 'utf8');
const appCss = readFileSync(join(repoRoot, 'frontend/app.css'), 'utf8');

const COLOR_PROP =
  /(?:^|[;{])\s*(?:-?-[a-z]+-)?(?:color|fill|stroke|background(?:-color|-image)?|border(?:-(?:top|right|bottom|left))?(?:-color)?|outline(?:-color)?|box-shadow|text-shadow|caret-color|scrollbar-color)\s*:\s*([^;{}]+)/gi;
const THEME_VAR = /var\(--((?:bg|fg|border|shadow|color)-[a-z0-9-]+)\)/g;
const RAW_HEX = /#[0-9a-fA-F]{3,8}\b/;
const RAW_RGBA = /rgba?\(/i;
const THEME_PREFIX = /^(?:bg|fg|border|shadow|color)-/;

function stripCssNoise(css) {
  return css
    .replace(/\/\*[\s\S]*?\*\//g, '')
    .replace(/url\(\s*(?:'[^']*'|"[^"]*"|[^)]+)\s*\)/gi, 'url()');
}

function listedThemeTokens(css) {
  const names = new Set();
  for (const match of css.matchAll(/--((?:bg|fg|border|shadow|color)-[a-z0-9-]+)\s*:/g)) {
    names.add(match[1]);
  }
  return names;
}

function appearanceValues(css) {
  const values = [];
  const cleaned = stripCssNoise(css);
  for (const match of cleaned.matchAll(COLOR_PROP)) {
    values.push(match[1]);
  }
  return values;
}

describe('OTH-209 Light theme tokens', () => {
  it('loads theme-light.css before app.css', () => {
    const themeAt = indexHtml.indexOf('href="theme-light.css"');
    const appAt = indexHtml.indexOf('href="app.css"');
    expect(themeAt).toBeGreaterThan(-1);
    expect(appAt).toBeGreaterThan(themeAt);
  });

  it('keeps Light values on the canvas and default ink tokens', () => {
    expect(themeCss).toMatch(/:root\s*\{/);
    expect(themeCss).toMatch(/--bg-canvas:\s*#f6f8fa;/);
    expect(themeCss).toMatch(/--fg-default:\s*#24292f;/);
    expect(themeCss).toMatch(/--bg-surface:\s*#fff;/);
    expect(themeCss).toMatch(/--border-default:\s*#d0d7de;/);
    const leftover = stripCssNoise(themeCss).replace(/:root\s*\{[\s\S]*\}/, '').trim();
    expect(leftover).toBe('');
  });

  it('points body appearance at those tokens', () => {
    expect(appCss).toMatch(/body\s*\{[\s\S]*?background:\s*var\(--bg-canvas\);/);
    expect(appCss).toMatch(/body\s*\{[\s\S]*?color:\s*var\(--fg-default\);/);
  });

  it('does not leave extracted appearance as hardcoded hex or rgba', () => {
    const raw = appearanceValues(appCss).filter(
      (value) => RAW_HEX.test(value) || RAW_RGBA.test(value),
    );
    expect(raw).toEqual([]);
  });

  it('defines every theme token that app.css asks for', () => {
    const defined = listedThemeTokens(themeCss);
    const asked = new Set();
    for (const match of appCss.matchAll(THEME_VAR)) {
      asked.add(match[1]);
    }
    const missing = [...asked].filter((name) => THEME_PREFIX.test(name) && !defined.has(name));
    expect(missing).toEqual([]);
  });
});
