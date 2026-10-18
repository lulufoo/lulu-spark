// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect } from 'vitest';
import { readShellHtml } from '../helpers/read-frontend-js.js';

const fixtureRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const searchSrc = readFileSync(join(fixtureRoot, 'frontend/src/notes/ui/search.tsx'), 'utf8');
const markup = [readShellHtml(), searchSrc].join('\n');
const appCss = readFileSync(join(fixtureRoot, 'frontend/app.css'), 'utf8');

describe('notes search markup (button + shared modal)', () => {
  it('the inline input, dropdown and wrap are gone', () => {
    expect(markup).not.toMatch(/id="gs-wrap"/);
    expect(markup).not.toMatch(/gs-wb-wrap|gs-wb-input|gs-wb-dropdown/);
    expect(markup).not.toMatch(/gs-search-wrap|gs-search-input|gs-search-dropdown/);
    expect(markup).not.toMatch(/gs-mode-pill/);
    expect(markup).not.toMatch(/IndexRebuildStatus|IndexRebuildButton/);
    expect(searchSrc).not.toMatch(/SparkSearch|initSparkSearch|closeSparkSearch/);
  });

  it('the toolbar shows one button with the ⌘K hint', () => {
    expect(searchSrc).toMatch(/className="notes-search-btn"/);
    expect(searchSrc).toMatch(/data-role="notes-search"/);
    expect(searchSrc).toMatch(/aria-keyshortcuts="Meta\+K"/);
    expect(searchSrc).toMatch(/<kbd className="notes-search-btn-key">⌘K<\/kbd>/);
  });

  it('the history key stays gs-history-wb (shared with Home, unchanged)', () => {
    const history = readFileSync(join(fixtureRoot, 'frontend/src/shared/search-modal/history.ts'), 'utf8');
    expect(history).toMatch(/SEARCH_HISTORY_KEY = 'gs-history-wb'/);
  });
});

describe('notes search CSS (app.css)', () => {
  it('uses class selectors instead of legacy id selectors', () => {
    expect(appCss).not.toMatch(/#gs-wrap\b/);
    expect(appCss).not.toMatch(/#gs-input\b/);
    expect(appCss).not.toMatch(/#gs-dropdown\b/);
    expect(appCss).not.toMatch(/#gs-mode-pill\b/);
  });

  it('the old inline search, hit-row and history styles are deleted', () => {
    expect(appCss).not.toMatch(/\.gs-search-/);
    expect(appCss).not.toMatch(/\.gs-status\b/);
    expect(appCss).not.toMatch(/\.gs-hit/);
    expect(appCss).not.toMatch(/\.gs-hist-/);
    expect(appCss).not.toMatch(/\.gs-layer-/);
  });

  it('the button keeps the old input look and is not a window-drag region', () => {
    expect(appCss).toMatch(/\.notes-search-btn \{[\s\S]*?background: var\(--bg-surface\);/);
    expect(appCss).toMatch(/\.notes-search-btn \{[\s\S]*?min-height: 36px;/);
    expect(appCss).toMatch(/\.notes-page-toolbar \.notes-search-btn,[\s\S]*?-webkit-app-region: no-drag;/);
  });
});
