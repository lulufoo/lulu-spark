// @vitest-environment jsdom
import { beforeEach, describe, expect, it } from 'vitest';
import {
  applyTheme,
  bootTheme,
  PHASE1_THEME_ID,
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

  it('resolves every reserved id to Light in phase 1', () => {
    expect(resolveThemeId('light')).toBe('light');
    expect(resolveThemeId('dark')).toBe('light');
    expect(resolveThemeId('system')).toBe('light');
    expect(themeFileFor('dark')).toBe('theme-light.css');
    expect(themeFileFor('system')).toBe('theme-light.css');
  });

  it('applyTheme loads the Light file and marks the applied id', () => {
    expect(applyTheme('dark')).toBe('light');
    const sheet = document.getElementById(THEME_SHEET_ID);
    expect(sheet?.getAttribute('href')).toBe('theme-light.css');
    expect(sheet?.dataset.themeId).toBe('light');
    expect(document.documentElement.dataset.theme).toBe('light');
  });

  it('bootTheme applies the phase-1 Light id', () => {
    expect(bootTheme()).toBe(PHASE1_THEME_ID);
    expect(document.documentElement.dataset.theme).toBe('light');
  });
});
