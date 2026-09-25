// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect } from 'vitest';
import { readShellHtml } from '../helpers/read-frontend-js.js';

const fixtureRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const indexHtml = readShellHtml();
const appCss = readFileSync(join(fixtureRoot, 'frontend/app.css'), 'utf8');

describe('dual search markup (index.html)', () => {
  it('replaces legacy #gs-wrap with #gs-wb-wrap and #gs-kb-wrap', () => {
    expect(indexHtml).not.toMatch(/id="gs-wrap"/);
    expect(indexHtml).toMatch(/id="gs-wb-wrap"/);
    expect(indexHtml).toMatch(/id="gs-kb-wrap"/);
  });

  it('each wrap has input and dropdown; no per-field rebuild button', () => {
    for (const prefix of ['wb', 'kb']) {
      expect(indexHtml).toMatch(new RegExp(`id="gs-${prefix}-input"`));
      expect(indexHtml).toMatch(new RegExp(`id="gs-${prefix}-dropdown"`));
      expect(indexHtml).not.toMatch(new RegExp(`id="gs-${prefix}-rebuild-btn"`));
    }
    expect(indexHtml).not.toMatch(/IndexRebuildStatus|IndexRebuildButton/);
  });

  it('removes gs-mode-pill and adds shared search classes', () => {
    expect(indexHtml).not.toMatch(/gs-mode-pill/);
    expect(indexHtml).toMatch(/className="[^"]*gs-search-wrap/);
    expect(indexHtml).toMatch(/className="[^"]*gs-search-input/);
    expect(indexHtml).toMatch(/className="[^"]*gs-search-dropdown/);
  });

  it('both wraps default to hidden', () => {
    expect(indexHtml).toMatch(/id="gs-wb-wrap"[^>]*\bhidden\b/);
    expect(indexHtml).toMatch(/id="gs-kb-wrap"[^>]*\bhidden\b/);
  });
});

describe('dual search CSS (app.css)', () => {
  it('uses class selectors instead of legacy id selectors', () => {
    expect(appCss).not.toMatch(/#gs-wrap\b/);
    expect(appCss).not.toMatch(/#gs-input\b/);
    expect(appCss).not.toMatch(/#gs-dropdown\b/);
    expect(appCss).not.toMatch(/#gs-mode-pill\b/);
    expect(appCss).toMatch(/\.gs-search-wrap\b/);
    expect(appCss).toMatch(/\.gs-search-input\b/);
    expect(appCss).toMatch(/\.gs-search-dropdown\b/);
  });

  it('retains shared hit and history classes', () => {
    expect(appCss).toMatch(/\.gs-rebuild-btn\b/);
    expect(appCss).toMatch(/\.gs-hit\b/);
    expect(appCss).toMatch(/\.gs-hist-/);
  });

  it('hidden attribute wins over flex display on search wraps', () => {
    expect(appCss).toMatch(/\.gs-search-wrap\[hidden\][\s\S]*display:\s*none\s*!important/);
  });
});
