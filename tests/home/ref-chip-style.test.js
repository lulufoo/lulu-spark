import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const css = readFileSync('frontend/app.css', 'utf8');
const light = readFileSync('frontend/theme-light.css', 'utf8');
const dark = readFileSync('frontend/theme-dark.css', 'utf8');

function rule(selector) {
  const escaped = selector.replace(/[.:[\]]/g, '\\$&');
  const match = new RegExp(`(?:^|\\n)${escaped}\\s*\\{([^}]*)\\}`).exec(css);
  return match ? match[1] : null;
}

describe('reference chip style', () => {
  it('has no border: it is a tinted pill', () => {
    const base = rule('.home-ref-chip');
    expect(base).not.toBeNull();
    expect(base).not.toMatch(/(^|[\s;])border(-color|-width)?\s*:\s*(?!none|0\b)/);
    expect(base).toMatch(/background:/);
  });

  it('only the background changes on hover', () => {
    const hover = rule('.home-ref-chip:hover');
    expect(hover).not.toBeNull();
    expect(hover).toMatch(/background:/);
    expect(hover).not.toMatch(/(^|[\s;])border[\w-]*\s*:/);
  });

  it('takes its colors from theme tokens, not literals', () => {
    const all = `${rule('.home-ref-chip')}${rule('.home-ref-chip:hover')}`;
    expect(all).not.toMatch(/#[0-9a-fA-F]{3,8}\b|rgba?\(/);
    expect(all).toMatch(/var\(--/);
  });

  it('is periwinkle: chip text from --fg-ref-chip, chip fill mixed from the app accent', () => {
    expect(light).toMatch(/--fg-ref-chip:\s*#3a4bbd;/);
    expect(dark).toMatch(/--fg-ref-chip:\s*#a4aef2;/);
    const base = rule('.home-ref-chip');
    expect(base).toMatch(/color:\s*var\(--fg-ref-chip\)/);
    expect(base).toMatch(/color-mix\([^)]*var\(--bg-accent\) 16%/);
    expect(rule('.home-ref-chip:hover')).toMatch(/color-mix\([^)]*var\(--bg-accent\) 26%/);
    expect(`${light}${dark}${css}`).not.toMatch(/d8833b/i);
  });

  it('keeps chip text at WCAG AA on its fill, hover included, over every surface', () => {
    const accent = { light: '#576ae0', dark: '#576ae0' };
    const surfaces = {
      light: ['#f0f2f4', '#f0f2fc', '#ffffff', '#f6f8fa'],
      dark: ['#19181b', '#212023', '#1c1d1f', '#26272b'],
    };
    const text = { light: '#3a4bbd', dark: '#a4aef2' };
    const rgb = (hex) => [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16));
    const lin = (v) => ((v / 255) <= 0.03928 ? v / 255 / 12.92 : ((v / 255 + 0.055) / 1.055) ** 2.4);
    const lum = ([r, g, b]) => 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
    const ratio = (a, b) => {
      const [hi, lo] = [lum(a), lum(b)].sort((x, y) => y - x);
      return (hi + 0.05) / (lo + 0.05);
    };
    for (const theme of ['light', 'dark']) {
      for (const surface of surfaces[theme]) {
        for (const alpha of [0.16, 0.26]) {
          const fill = rgb(accent[theme]).map((c, i) => c * alpha + rgb(surface)[i] * (1 - alpha));
          expect(ratio(rgb(text[theme]), fill)).toBeGreaterThanOrEqual(4.5);
        }
      }
    }
  });

  it('sits on the text baseline instead of being pinned to the line box', () => {
    const base = rule('.home-ref-chip');
    expect(base).toMatch(/display:\s*inline-flex/);
    expect(base).not.toMatch(/vertical-align:\s*(bottom|top|middle|text-bottom|text-top)/);
    expect(base).not.toMatch(/overflow:/);
  });

  it('truncates a long title on the label, not on the chip', () => {
    const label = rule('.home-ref-chip-label');
    expect(label).not.toBeNull();
    expect(label).toMatch(/overflow:\s*hidden/);
    expect(label).toMatch(/text-overflow:\s*ellipsis/);
    expect(label).toMatch(/white-space:\s*nowrap/);
  });
});
