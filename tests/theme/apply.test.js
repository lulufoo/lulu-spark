// @vitest-environment jsdom
import { beforeEach, describe, expect, it } from 'vitest';
import {
  applyTheme,
  bootTheme,
  PHASE1_THEME_ID,
  readRequestedTheme,
  resolveThemeId,
  THEME_IDS,
  THEME_SHEET_ID,
  themeFileFor,
} from '../../frontend/src/theme.ts';

function mountSheet() {
  document.documentElement.removeAttribute('data-theme');
  document.getElementById(THEME_SHEET_ID)?.remove();
  const sheet = document.createElement('link');
  sheet.id = THEME_SHEET_ID;
  sheet.rel = 'stylesheet';
  sheet.setAttribute('href', 'theme-light.css');
  sheet.dataset.themeId = 'light';
  document.head.appendChild(sheet);
  return sheet;
}

describe('theme loader', () => {
  beforeEach(() => {
    mountSheet();
  });

  it('reserves light, dark, and system ids', () => {
    expect(THEME_IDS).toEqual(['light', 'dark', 'system']);
  });

  it('resolves dark to the Dark file and leaves system on Light', () => {
    expect(resolveThemeId('light')).toBe('light');
    expect(resolveThemeId('dark')).toBe('dark');
    expect(resolveThemeId('system')).toBe('light');
    expect(themeFileFor('dark')).toBe('theme-dark.css');
    expect(themeFileFor('system')).toBe('theme-light.css');
  });

  it('applyTheme loads Dark without changing structure sheets', () => {
    expect(applyTheme('dark')).toBe('dark');
    const sheet = document.getElementById(THEME_SHEET_ID);
    expect(sheet?.getAttribute('href')).toBe('theme-dark.css');
    expect(sheet?.dataset.themeId).toBe('dark');
    expect(document.documentElement.dataset.theme).toBe('dark');
  });

  it('bootTheme applies Light when no theme query is present', () => {
    expect(bootTheme()).toBe(PHASE1_THEME_ID);
    expect(document.documentElement.dataset.theme).toBe('light');
  });

  it('reads a Dark query as a dev entry, not as Settings', () => {
    expect(readRequestedTheme('')).toBeNull();
    expect(readRequestedTheme('?theme=dark')).toBe('dark');
    expect(readRequestedTheme('?theme=system')).toBeNull();
  });
});
