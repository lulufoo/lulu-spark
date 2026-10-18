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

const darkCss = readFileSync(join(repoRoot, 'frontend/theme-dark.css'), 'utf8');

function tokenValue(css, name) {
  const match = css.match(new RegExp(`--${name}:\\s*([^;]+);`));
  if (!match) throw new Error(`token --${name} not found`);
  return match[1].trim();
}

function channel(value) {
  const v = value / 255;
  return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
}

function luminance(hex) {
  const n = parseInt(hex.slice(1), 16);
  return 0.2126 * channel((n >> 16) & 255) + 0.7152 * channel((n >> 8) & 255) + 0.0722 * channel(n & 255);
}

function contrast(a, b) {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}

describe('Periwinkle accent tokens', () => {
  it('uses one accent #576ae0 in Light and Dark', () => {
    for (const css of [themeCss, darkCss]) {
      expect(tokenValue(css, 'bg-accent')).toBe('#576ae0');
      expect(tokenValue(css, 'border-accent')).toBe('#576ae0');
      expect(tokenValue(css, 'bg-accent-emphasis')).toBe('#4a5cd4');
    }
  });

  it('locks the Light link, selected row and focus ring values', () => {
    expect(tokenValue(themeCss, 'fg-accent')).toBe('#4f62d8');
    expect(tokenValue(themeCss, 'fg-accent-emphasis')).toBe('#4a5cd4');
    expect(tokenValue(themeCss, 'bg-hex-ddf4ff')).toBe('#ebedfb');
    expect(tokenValue(themeCss, 'shadow-rgba-accent-35')).toBe('rgba(87, 106, 224, 0.35)');
  });

  it('locks the Dark link, selected row and focus ring values', () => {
    expect(tokenValue(darkCss, 'fg-accent')).toBe('#8b98ee');
    expect(tokenValue(darkCss, 'fg-accent-emphasis')).toBe('#a4aef2');
    expect(tokenValue(darkCss, 'bg-hex-ddf4ff')).toBe('#262a45');
    expect(tokenValue(darkCss, 'shadow-rgba-accent-35')).toBe('rgba(110, 127, 229, 0.45)');
  });

  it('keeps accent text and button labels at WCAG AA (4.5)', () => {
    expect(contrast('#ffffff', tokenValue(themeCss, 'bg-accent'))).toBeGreaterThanOrEqual(4.5);
    expect(contrast('#ffffff', tokenValue(themeCss, 'bg-accent-emphasis'))).toBeGreaterThanOrEqual(4.5);
    expect(contrast(tokenValue(themeCss, 'fg-accent'), '#ffffff')).toBeGreaterThanOrEqual(4.5);
    expect(
      contrast(tokenValue(themeCss, 'fg-accent-emphasis'), tokenValue(themeCss, 'bg-hex-ddf4ff')),
    ).toBeGreaterThanOrEqual(4.5);
    expect(
      contrast(tokenValue(darkCss, 'fg-accent'), tokenValue(darkCss, 'bg-surface')),
    ).toBeGreaterThanOrEqual(4.5);
    expect(
      contrast(tokenValue(darkCss, 'fg-accent-emphasis'), tokenValue(darkCss, 'bg-hex-ddf4ff')),
    ).toBeGreaterThanOrEqual(4.5);
  });

  it('paints the in-app icons and the sign-in landing with the periwinkle icon colours', () => {
    const iconSources = [
      'frontend/icons/app-icon.svg',
      'frontend/src/home/ui/welcome.tsx',
      'frontend/src/shared/home-mark.tsx',
      'src-tauri/src/services/auth_login_landing.html',
    ].map((rel) => readFileSync(join(repoRoot, rel), 'utf8'));
    for (const source of iconSources) {
      expect(source).toMatch(/576AE0/i);
      expect(source).toMatch(/FF5A4E/i);
      expect(source).not.toMatch(/0071e3|113, 227/i);
    }
  });

  it('leaves no legacy blue in the theme files or app.css', () => {
    for (const css of [themeCss, darkCss, appCss]) {
      expect(css).not.toMatch(/0969da|#3b73ff|#2a5fe0|#4187da|#0550ae/i);
    }
  });
});
