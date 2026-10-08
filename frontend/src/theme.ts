export const THEME_IDS = ['light', 'dark', 'system'] as const;
export type ThemeId = (typeof THEME_IDS)[number];

export const THEME_SHEET_ID = 'theme-sheet';
export const PHASE1_THEME_ID: ThemeId = 'light';

const THEME_FILE = {
  light: 'theme-light.css',
  dark: 'theme-dark.css',
} as const;

let preferredTheme: ThemeId = PHASE1_THEME_ID;

export function preferredThemeId(): ThemeId {
  return preferredTheme;
}

export function prefersDarkScheme(): boolean {
  return typeof matchMedia === 'function'
    && matchMedia('(prefers-color-scheme: dark)').matches;
}

export function resolveThemeId(id: ThemeId): ThemeId {
  if (id === 'dark') return 'dark';
  if (id === 'system') return prefersDarkScheme() ? 'dark' : 'light';
  return 'light';
}

export function parseThemeId(value: unknown): ThemeId | null {
  return THEME_IDS.includes(value as ThemeId) ? (value as ThemeId) : null;
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

export type WindowTheme = 'light' | 'dark' | null;

export function windowThemeFor(id: ThemeId): WindowTheme {
  if (id === 'system') return null;
  return id === 'dark' ? 'dark' : 'light';
}

export async function syncWindowTheme(id: ThemeId): Promise<void> {
  const theme = windowThemeFor(id);
  try {
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    await getCurrentWindow().setTheme(theme);
  } catch {
    // Browser / tests have no Tauri window.
  }
}

export function applyTheme(id: ThemeId): ThemeId {
  preferredTheme = id;
  const applied = resolveThemeId(id);
  const sheet = document.getElementById(THEME_SHEET_ID);
  if (sheet instanceof HTMLLinkElement) {
    sheet.setAttribute('href', themeFileFor(applied));
    sheet.dataset.themeId = applied;
  }
  document.documentElement.dataset.theme = applied;
  void syncWindowTheme(id);
  return applied;
}

export function watchSystemTheme(): () => void {
  if (typeof matchMedia !== 'function') return () => {};
  const media = matchMedia('(prefers-color-scheme: dark)');
  const onChange = () => {
    if (preferredTheme === 'system') applyTheme('system');
  };
  media.addEventListener('change', onChange);
  return () => media.removeEventListener('change', onChange);
}

export function bootTheme(preferred?: ThemeId): ThemeId {
  return applyTheme(readRequestedTheme() ?? preferred ?? PHASE1_THEME_ID);
}
