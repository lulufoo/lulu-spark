import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

function read(rel) {
  return readFileSync(join(repoRoot, rel), 'utf8');
}

const chrome = read('frontend/src/app-shell/ui/settings/chrome.tsx');
const types = read('frontend/src/app-shell/state/types.ts');
const boot = read('frontend/src/boot.ts');
const themeSrc = read('frontend/src/theme.ts');
const settingsTheme = read('frontend/src/app-shell/commands/settings/theme.ts');
const rustTypes = read('src-tauri/src/config/settings/types.rs');
const rustStore = read('src-tauri/src/config/settings/store.rs');

describe('OTH-213 Settings Theme', () => {
  it('adds a Theme panel with the three stable ids', () => {
    expect(chrome).toMatch(/data-panel="theme"[^>]*>Theme</);
    expect(chrome).toMatch(/id="settings-panel-theme"/);
    expect(chrome).toMatch(/id="settings-theme"/);
    expect(chrome).toMatch(/value="system">Follow system</);
    expect(chrome).toMatch(/value="light">Light</);
    expect(chrome).toMatch(/value="dark">Dark</);
  });

  it('persists theme through fetchConfig / setConfig, not localStorage', () => {
    expect(types).toMatch(/\btheme\?: string/);
    expect(settingsTheme).toMatch(/setConfig\(\{ theme: id \}\)/);
    expect(boot).toMatch(/parseThemeId\(cfg\.theme\)/);
    expect(boot).toMatch(/watchSystemTheme\(\)/);
    expect(themeSrc).toMatch(/prefers-color-scheme: dark/);
    expect(themeSrc).not.toMatch(/localStorage/);
    expect(rustTypes).toMatch(/pub theme: String/);
    expect(rustStore).toMatch(/"theme": settings\.theme/);
    expect(rustStore).toMatch(/payload\.get\("theme"\)/);
  });
});
