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

  it('uses the orange of Cursor\'s reference chip, in both themes', () => {
    expect(light).toMatch(/--fg-ref-chip:\s*#d8833b;/);
    expect(dark).toMatch(/--fg-ref-chip:\s*#d8833b;/);
    const base = rule('.home-ref-chip');
    expect(base).toMatch(/color:\s*var\(--fg-ref-chip\)/);
    expect(base).toMatch(/color-mix\([^)]*var\(--fg-ref-chip\)/);
    expect(rule('.home-ref-chip:hover')).toMatch(/color-mix\([^)]*var\(--fg-ref-chip\)/);
  });

  it('does not borrow the app accent (blue) for the chip', () => {
    const all = `${rule('.home-ref-chip')}${rule('.home-ref-chip:hover')}`;
    expect(all).not.toMatch(/--(fg|bg|border)-accent/);
  });
});
