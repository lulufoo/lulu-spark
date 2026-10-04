// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect, beforeEach, vi } from 'vitest';

vi.mock('../../frontend/src/notes/ui/search.tsx', () => ({
  closeSparkSearch: vi.fn(),
}));
vi.mock('../../frontend/src/knowledge/ui/search.tsx', () => ({
  closeKnowledgeSearch: vi.fn(),
}));

import { closeSparkSearch } from '../../frontend/src/notes/ui/search.tsx';
import { closeKnowledgeSearch } from '../../frontend/src/knowledge/ui/search.tsx';
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
    <div id="gs-kb-wrap" class="gs-search-wrap" hidden>
      <input id="gs-kb-input" class="gs-search-input" type="text" />
    </div>
  `;
}

describe('applySearchNavChrome dual search wraps', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    navChromeDom();
  });

  it('home: both wraps hidden and inputs disabled', () => {
    applySearchNavChrome('home');

    const wbWrap = document.getElementById('gs-wb-wrap');
    const kbWrap = document.getElementById('gs-kb-wrap');
    const wbInput = document.getElementById('gs-wb-input');
    const kbInput = document.getElementById('gs-kb-input');

    expect(wbWrap.hidden).toBe(true);
    expect(kbWrap.hidden).toBe(true);
    expect(wbInput.disabled).toBe(true);
    expect(kbInput.disabled).toBe(true);
  });

  it('spark: wb visible+enabled, kb hidden', () => {
    applySearchNavChrome('spark');

    expect(document.getElementById('gs-wb-wrap').hidden).toBe(false);
    expect(document.getElementById('gs-kb-wrap').hidden).toBe(true);
    expect(document.getElementById('gs-wb-input').disabled).toBe(false);
    expect(document.getElementById('gs-kb-input').disabled).toBe(true);
  });

  it('knowledge-doc: kb visible+enabled, wb hidden', () => {
    applySearchNavChrome('knowledge-doc');

    expect(document.getElementById('gs-wb-wrap').hidden).toBe(true);
    expect(document.getElementById('gs-kb-wrap').hidden).toBe(false);
    expect(document.getElementById('gs-wb-input').disabled).toBe(true);
    expect(document.getElementById('gs-kb-input').disabled).toBe(false);
  });

  it('calls close*Search on every route switch', () => {
    applySearchNavChrome('home');
    applySearchNavChrome('spark');
    applySearchNavChrome('knowledge-doc');

    expect(closeSparkSearch).toHaveBeenCalledTimes(3);
    expect(closeKnowledgeSearch).toHaveBeenCalledTimes(3);
  });

  it('rapid home ↔ spark ↔ knowledge leaves only the active wrap visible', () => {
    applySearchNavChrome('home');
    applySearchNavChrome('spark');
    applySearchNavChrome('knowledge-doc');
    applySearchNavChrome('home');
    applySearchNavChrome('spark');

    expect(document.getElementById('gs-wb-wrap').hidden).toBe(false);
    expect(document.getElementById('gs-kb-wrap').hidden).toBe(true);
  });

  it('home route: search inputs cannot receive focus (hidden + disabled)', () => {
    applySearchNavChrome('home');

    const wbInput = document.getElementById('gs-wb-input');
    const kbInput = document.getElementById('gs-kb-input');
    wbInput.focus();
    kbInput.focus();

    expect(document.activeElement).not.toBe(wbInput);
    expect(document.activeElement).not.toBe(kbInput);
  });

  it('main.js updateNavChrome keeps home header title/back swap', () => {
    expect(mainJs).toMatch(/btn-nav-home-title/);
    expect(mainJs).toMatch(/homeTitle\) homeTitle\.hidden = !onHome/);
    expect(mainJs).toMatch(/homeNav\) homeNav\.hidden = onHome/);
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
