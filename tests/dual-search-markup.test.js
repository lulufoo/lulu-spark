// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect } from 'vitest';

const fixtureRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const indexHtml = readFileSync(join(fixtureRoot, 'frontend/index.html'), 'utf8');
const appCss = readFileSync(join(fixtureRoot, 'frontend/app.css'), 'utf8');

describe('dual search markup (index.html)', () => {
  it('replaces legacy #gs-wrap with #gs-wb-wrap and #gs-kb-wrap', () => {
    expect(indexHtml).not.toMatch(/id="gs-wrap"/);
    expect(indexHtml).toMatch(/id="gs-wb-wrap"/);
    expect(indexHtml).toMatch(/id="gs-kb-wrap"/);
  });

  it('each wrap has input, dropdown, and rebuild button', () => {
    for (const prefix of ['wb', 'kb']) {
      expect(indexHtml).toMatch(new RegExp(`id="gs-${prefix}-input"`));
      expect(indexHtml).toMatch(new RegExp(`id="gs-${prefix}-dropdown"`));
      expect(indexHtml).toMatch(new RegExp(`id="gs-${prefix}-rebuild-btn"`));
    }
  });

  it('removes gs-mode-pill and adds shared search classes', () => {
    expect(indexHtml).not.toMatch(/gs-mode-pill/);
    expect(indexHtml).toMatch(/class="[^"]*gs-search-wrap/);
    expect(indexHtml).toMatch(/class="[^"]*gs-search-input/);
    expect(indexHtml).toMatch(/class="[^"]*gs-search-dropdown/);
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
});
