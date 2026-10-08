import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

function read(rel) {
  return readFileSync(join(repoRoot, rel), 'utf8');
}

function tokenNames(css) {
  return [...css.matchAll(/--((?:bg|fg|border|shadow|color)-[a-z0-9-]+)\s*:/g)].map(
    (match) => match[1],
  );
}

const lightCss = read('frontend/theme-light.css');
const darkCss = read('frontend/theme-dark.css');
const themeSrc = read('frontend/src/theme.ts');
const appCss = read('frontend/app.css');
const notesLinks = read('frontend/src/notes/ui/links-bar.tsx');
const knowledgeLinks = read('frontend/src/knowledge/ui/links-bar.tsx');

describe('OTH-211 Dark theme tokens', () => {
  it('defines the same token names as Light on a dark canvas', () => {
    expect(darkCss).toMatch(/:root\s*\{/);
    expect(darkCss).toMatch(/color-scheme:\s*dark;/);
    expect(darkCss).toMatch(/--bg-canvas:\s*#1c1d1f;/);
    expect(darkCss).toMatch(/--fg-default:\s*#b7b4bb;/);
    expect(darkCss).toMatch(/--fg-accent:\s*#c4c1c7;/);
    expect(darkCss).toMatch(/--fg-accent-emphasis:\s*#d2d0d6;/);
    expect(darkCss).toMatch(/--bg-surface:\s*#212023;/);
    expect(darkCss).toMatch(/--bg-hex-e7ebef:\s*#2a2b2e;/);
    expect(tokenNames(darkCss)).toEqual(tokenNames(lightCss));
  });

  it('lets the loader swap to theme-dark.css', () => {
    expect(themeSrc).toMatch(/dark:\s*'theme-dark\.css'/);
    expect(themeSrc).toMatch(/id === 'dark' \? 'dark'/);
    expect(themeSrc).toMatch(/readRequestedTheme/);
  });

  it('does not leave the Light muted chevron hex in app.css', () => {
    expect(appCss).not.toMatch(/stroke='%2357606a'/);
  });

  it('lifts Dark Send above the composer dock without remapping --bg-emphasis-2', () => {
    expect(appCss).toMatch(
      /\[data-theme="dark"\]\s+\.home-chat-send\s*\{[^}]*background:\s*var\(--bg-dark\)/,
    );
    expect(appCss).toMatch(
      /\[data-theme="dark"\]\s+\.home-chat-send:hover\s*\{[^}]*background:\s*var\(--border-emphasis-2\)/,
    );
    expect(appCss).toMatch(
      /\[data-theme="dark"\]\s+\.home-chat-send:disabled\s*\{[^}]*background:\s*var\(--bg-muted\)/,
    );
    expect(darkCss).toMatch(/--bg-dark:\s*#4c4851;/);
    expect(darkCss).toMatch(/--bg-emphasis-2:\s*#19181b;/);
  });

  it('lifts Dark account placeholder avatar off the sidebar surface', () => {
    expect(appCss).toMatch(
      /\[data-theme="dark"\]\s+\.home-account-bar-avatar--placeholder\s*\{[^}]*background:\s*var\(--bg-dark\)/,
    );
    expect(darkCss).toMatch(/--bg-hex-1d1d1f:\s*#212023;/);
  });

  it('does not hardcode Light link blue on notes or knowledge link bars', () => {
    expect(notesLinks).not.toMatch(/#0969da/);
    expect(knowledgeLinks).not.toMatch(/#0969da/);
    expect(notesLinks).toMatch(/color:\s*'var\(--fg-accent\)'/);
    expect(knowledgeLinks).toMatch(/color:\s*'var\(--fg-accent\)'/);
  });

  it('lifts Dark settings save buttons off --bg-accent', () => {
    expect(appCss).toMatch(
      /\[data-theme="dark"\]\s+\.btn-settings-save\s*\{[^}]*background:\s*var\(--bg-dark\)/,
    );
    expect(appCss).toMatch(
      /\[data-theme="dark"\]\s+\.btn-settings-save:hover:not\(:disabled\)\s*\{[^}]*background:\s*var\(--border-emphasis-2\)/,
    );
    expect(darkCss).toMatch(/--bg-accent:\s*#3b73ff;/);
  });

  it('paints form focus from --border-focus instead of --border-accent', () => {
    expect(lightCss).toMatch(/--border-focus:\s*#afb8c1;/);
    expect(darkCss).toMatch(/--border-focus:\s*#98959d;/);
    expect(darkCss).toMatch(/--border-accent:\s*#3b73ff;/);
    expect(lightCss).toMatch(/--border-accent:\s*#0969da;/);
    expect(appCss).toMatch(/\.gs-search-input:focus\s*\{[^}]*border-color:\s*var\(--border-focus\)/);
    expect(appCss).toMatch(/\.settings-field input:focus[\s\S]*?border-color:\s*var\(--border-focus\)/);
    expect(appCss).toMatch(
      /\.home-chat-composer-dock:focus-within\s*\{[^}]*border-color:\s*var\(--border-focus\)/,
    );
    expect(appCss).not.toMatch(
      /\.home-chat-composer-dock:focus-within\s*\{[^}]*shadow-rgba-0969da/,
    );
    expect(appCss).toMatch(
      /\[data-theme="light"\]\s+\.home-chat-composer-dock:focus-within\s*\{[^}]*shadow-rgba-1f2328-06/,
    );
    expect(appCss).not.toMatch(/\.settings-field input:focus \{ border-color: var\(--border-accent\)/);
    expect(appCss).not.toMatch(/\.gs-search-input:focus\s*\{[^}]*--border-accent/);
  });

  it('lifts the Dark context ring off the composer surface', () => {
    expect(appCss).toMatch(
      /\[data-theme="dark"\]\s+\.home-chat-context-ring-track\s*\{[^}]*stroke:\s*var\(--fg-subtle\)/,
    );
    expect(appCss).toMatch(
      /\[data-theme="dark"\]\s+\.home-chat-context-ring-progress\s*\{[^}]*stroke:\s*var\(--fg-muted\)/,
    );
    expect(darkCss).toMatch(/--fg-rgba-1f2328-08:\s*rgba\(31, 35, 40, 0\.14\);/);
  });
});
