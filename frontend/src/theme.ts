export const THEME_IDS = ['light', 'dark', 'system'] as const;
export type ThemeId = (typeof THEME_IDS)[number];

export const THEME_SHEET_ID = 'theme-sheet';
export const PHASE1_THEME_ID: ThemeId = 'light';

const THEME_FILE = {
  light: 'theme-light.css',
  dark: 'theme-dark.css',
} as const;

export function resolveThemeId(id: ThemeId): ThemeId {
  return id === 'dark' ? 'dark' : 'light';
}

export function themeFileFor(id: ThemeId): string {
  return resolveThemeId(id) === 'dark' ? THEME_FILE.dark : THEME_FILE.light;
}

export function readRequestedTheme(
  search = typeof location === 'undefined' ? '' : location.search,
): ThemeId | null {
  const value = new URLSearchParams(search).get('theme');
  return value === 'dark' || value === 'light' ? value : null;
}

export function applyTheme(id: ThemeId): ThemeId {
  const applied = resolveThemeId(id);
  const sheet = document.getElementById(THEME_SHEET_ID);
  if (sheet instanceof HTMLLinkElement) {
    sheet.setAttribute('href', themeFileFor(applied));
    sheet.dataset.themeId = applied;
  }
  document.documentElement.dataset.theme = applied;
  return applied;
}

export function bootTheme(): ThemeId {
  return applyTheme(readRequestedTheme() ?? PHASE1_THEME_ID);
}
