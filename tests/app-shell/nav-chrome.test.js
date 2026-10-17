// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect, beforeEach, vi } from 'vitest';

vi.mock('../../frontend/src/notes/ui/search.tsx', () => ({
  closeSparkSearch: vi.fn(),
}));

import { closeSparkSearch } from '../../frontend/src/notes/ui/search.tsx';
import { applySearchNavChrome } from '../../frontend/src/app-shell/ui/nav-chrome.ts';
import { readMainSource } from '../helpers/read-frontend-js.js';

const fixtureRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const mainJs = readMainSource();

function navChromeDom() {
  document.body.innerHTML = `
    <button id="btn-nav-home-title" hidden></button>
    <button id="btn-nav-home"></button>
    <div id="gs-wb-wrap" class="gs-search-wrap" hidden>
      <input id="gs-wb-input" class="gs-search-input" type="text" />
    </div>
  `;
}

describe('applySearchNavChrome search wrap', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    navChromeDom();
  });

  it('home: wrap hidden and input disabled', () => {
    applySearchNavChrome('home');

    expect(document.getElementById('gs-wb-wrap').hidden).toBe(true);
    expect(document.getElementById('gs-wb-input').disabled).toBe(true);
  });

  it('spark: wrap visible and input enabled', () => {
    applySearchNavChrome('spark');

    expect(document.getElementById('gs-wb-wrap').hidden).toBe(false);
    expect(document.getElementById('gs-wb-input').disabled).toBe(false);
  });

  it('calls closeSparkSearch on every route switch', () => {
    applySearchNavChrome('home');
    applySearchNavChrome('spark');
    applySearchNavChrome('read-later');

    expect(closeSparkSearch).toHaveBeenCalledTimes(3);
  });

  it('rapid home <-> spark leaves only the active state', () => {
    applySearchNavChrome('home');
    applySearchNavChrome('spark');
    applySearchNavChrome('home');
    applySearchNavChrome('spark');

    expect(document.getElementById('gs-wb-wrap').hidden).toBe(false);
  });

  it('home route: the search input cannot receive focus (hidden + disabled)', () => {
    applySearchNavChrome('home');

    const wbInput = document.getElementById('gs-wb-input');
    wbInput.focus();

    expect(document.activeElement).not.toBe(wbInput);
  });

  it('main.js updateNavChrome only applies search chrome', () => {
    expect(mainJs).not.toMatch(/btn-nav-home-title/);
    expect(mainJs).not.toMatch(/classList\.toggle\('is-back', !onHome\)/);
    expect(mainJs).toMatch(/applySearchNavChrome\(routeName\)/);
  });
});

describe('main.js nav chrome integration', () => {
  it('imports applySearchNavChrome from nav-chrome.ts via wrapRouteMount', () => {
    expect(mainJs).toMatch(/from '[^']*nav-chrome\.ts'/);
    expect(mainJs).toMatch(/updateNavChrome\(routeName\)/);
  });

  it('removes initGlobalSearch import and init call', () => {
    expect(mainJs).not.toMatch(/initGlobalSearch/);
  });

  it('keeps wrapRouteMount and home hub default landing', () => {
    expect(mainJs).toMatch(/wrapRouteMount/);
    expect(mainJs).toMatch(/HomePage/);
    expect(mainJs).toMatch(/['"]#\/home['"]/);
  });
});
