import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const read = (rel) => readFileSync(join(repoRoot, rel), 'utf8');
const lightCss = read('frontend/theme-light.css');
const darkCss = read('frontend/theme-dark.css');
const appCss = read('frontend/app.css');
const homeCss = read('frontend/home.css');

const HOVER_PART = /:hover|:focus-visible|\[data-highlighted\]|\[data-active="true"\]|\[aria-expanded="true"\]/;
const SELECTED_PART = /\.active\b|\.selected\b|\.is-active\b|\.is-selected\b|--active\b|--selected\b/;
const NEUTRAL_FILL =
  /var\(--(?:bg-(?:canvas|muted|subtle|surface|inset)|bg-hex-(?:e7ebef|ececef|e8ecf0|e3e9f0|f3f4f6|eef6ff)|bg-rgba-eaeef2-70)\)/;

// Rules that may keep a neutral fill while hovered or selected, with the reason.
const NEUTRAL_ALLOWED = new Map([
  ['.knowledge-doc-tree-node.is-dir.is-expanded > .knowledge-doc-tree-label:not(.selected)', 'expanded folder, not a hover or selected state'],
  ['.todo-task-sub--selected .todo-task-sub-title-input', 'text input inside the selected card needs a white field'],
  ['.list-select-trigger:focus-visible', 'focus ring on a form field keeps the field fill'],
  ['.comment-tab-btn.active', 'segmented control: white pill on its grey track'],
  ['.convert-tab-btn.active', 'segmented control: white pill on its grey track'],
]);

function tokenValue(css, name) {
  const match = css.match(new RegExp(`--${name}:\\s*([^;]+);`));
  if (!match) throw new Error(`token --${name} not found`);
  return match[1].trim();
}

function rules() {
  const css = `${appCss}\n${homeCss}`.replace(/\/\*[\s\S]*?\*\//g, '');
  return [...css.matchAll(/([^{}]+)\{([^{}]*)\}/g)].map(([, selector, body]) => ({
    selector: selector.trim().replace(/\s+/g, ' '),
    parts: selector.split(',').map((part) => part.trim()),
    fills: [...body.matchAll(/background(?:-color)?\s*:\s*([^;]+)/g)].map((m) => m[1].trim()),
  }));
}

const usesToken = (rule, name) => rule.fills.some((fill) => fill.includes(`var(--${name})`));
const isHover = (rule) => rule.parts.every((part) => HOVER_PART.test(part));
const isSelected = (rule) => rule.parts.every((part) => SELECTED_PART.test(part) && !HOVER_PART.test(part));

describe('hover and selected colours are defined once', () => {
  it('defines --bg-hover and --bg-selected in both themes with the locked values', () => {
    expect(tokenValue(lightCss, 'bg-hover')).toBe('rgba(87, 106, 224, 0.06)');
    expect(tokenValue(darkCss, 'bg-hover')).toBe('rgba(110, 127, 229, 0.12)');
    expect(tokenValue(lightCss, 'bg-selected')).toBe('#ebedfb');
    expect(tokenValue(darkCss, 'bg-selected')).toBe('#262a45');
  });

  it('drops the old per-purpose tokens so nothing can bypass the semantic ones', () => {
    for (const source of [lightCss, darkCss, appCss]) {
      expect(source).not.toMatch(/--bg-accent-hover\b/);
      expect(source).not.toMatch(/--bg-hex-ddf4ff\b/);
    }
  });

  it('has no second hover colour in home.css', () => {
    for (const source of [lightCss, darkCss, appCss, homeCss]) {
      expect(source).not.toMatch(/--home-hover\b/);
    }
  });

  it('only hover rules use --bg-hover', () => {
    const users = rules().filter((rule) => usesToken(rule, 'bg-hover'));
    expect(users.length).toBeGreaterThan(50);
    expect(users.filter((rule) => !isHover(rule)).map((rule) => rule.selector)).toEqual([]);
  });

  it('only selected rules use --bg-selected', () => {
    const users = rules().filter((rule) => usesToken(rule, 'bg-selected'));
    expect(users.length).toBeGreaterThanOrEqual(9);
    expect(users.filter((rule) => !isSelected(rule)).map((rule) => rule.selector)).toEqual([]);
  });

  it('keeps coloured chips off the hover and selected tokens', () => {
    const chips = rules().filter((rule) => usesToken(rule, 'bg-accent-subtle'));
    expect(chips.length).toBeGreaterThan(0);
    expect(chips.filter((rule) => isHover(rule) || isSelected(rule)).map((rule) => rule.selector)).toEqual([]);
  });

  it('does not paint hover or selected rules with a neutral fill unless allowed', () => {
    const offenders = rules()
      .filter((rule) => rule.parts.some((part) => HOVER_PART.test(part) || SELECTED_PART.test(part)))
      .filter((rule) => rule.fills.some((fill) => NEUTRAL_FILL.test(fill)))
      .map((rule) => rule.selector)
      .filter((selector) => !NEUTRAL_ALLOWED.has(selector));
    expect(offenders).toEqual([]);
  });

  it('keeps every allow-list entry alive', () => {
    const selectors = new Set(rules().map((rule) => rule.selector));
    expect([...NEUTRAL_ALLOWED.keys()].filter((selector) => !selectors.has(selector))).toEqual([]);
  });
});
