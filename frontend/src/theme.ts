export const THEME_IDS = ['light', 'dark', 'system'] as const;
export type ThemeId = (typeof THEME_IDS)[number];

export const THEME_SHEET_ID = 'theme-sheet';
export const PHASE1_THEME_ID: ThemeId = 'light';

const THEME_FILE = {
  light: 'theme-light.css',
} as const;

export function resolveThemeId(_id: ThemeId): ThemeId {
  return PHASE1_THEME_ID;
}

export function themeFileFor(id: ThemeId): string {
  return THEME_FILE[resolveThemeId(id)];
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
  return applyTheme(PHASE1_THEME_ID);
}
