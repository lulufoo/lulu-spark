// @vitest-environment jsdom
/**
 * T5: retire #btn-feed / showFeedView as a user-reachable Builders path.
 * New builders-entry FAB path (t3) remains the sole entry.
 */
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

const renderFeedMock = vi.fn();

vi.mock('../frontend/js/builders/feed.js', () => ({
  renderFeed: (...args) => renderFeedMock(...args),
}));

import { createBuildersContentAdapter } from '../frontend/js/builders/assistant.js';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..');

function readMain() {
  return readFileSync(join(repoRoot, 'frontend/js/main.js'), 'utf8');
}

function readIndexHtml() {
  return readFileSync(join(repoRoot, 'frontend/index.html'), 'utf8');
}

function extractFunctionSource(source, name) {
  const start = source.indexOf(`function ${name}`);
  if (start === -1) return '';
  const braceStart = source.indexOf('{', start);
  let depth = 0;
  for (let i = braceStart; i < source.length; i += 1) {
    if (source[i] === '{') depth += 1;
    else if (source[i] === '}') {
      depth -= 1;
      if (depth === 0) return source.slice(start, i + 1);
    }
  }
  return '';
}

/** Bare `#btn-feed` classList sync — not optional-chained / null-guarded. */
const BARE_BTN_FEED_CLASSLIST =
  /document\.getElementById\(\s*['"]btn-feed['"]\s*\)\.classList\.(?:add|remove)\s*\(/;

const BTN_FEED_CLICK_WIRING =
  /document\.getElementById\(\s*['"]btn-feed['"]\s*\)\.addEventListener\(\s*['"]click['"]/;

function stubMountEnv() {
  return {
    clearHeaderSyncCorpusContext: () => {},
    unmountCorpusDocList: null,
    corpusDocListRepo: '',
    hideCorpusDocView: () => {},
    hideReadLaterView: () => {},
    hideTodoTasksView: () => {},
    hideHomeView: () => {},
    feedView: document.getElementById('feed-view') || { style: { display: '' } },
    unmountTodoTaskSplit: null,
    unmountHomeHub: null,
    mountHomeHub: () => () => {},
    mountTodoTaskSplit: () => ({ unmount: () => {} }),
    navigate: () => {},
    openReadLaterDialog: () => {},
    state: { ui: { activeDate: null } },
  };
}

function compileMountFn(fnSource, env) {
  const locals = [
    'clearHeaderSyncCorpusContext',
    'unmountCorpusDocList',
    'corpusDocListRepo',
    'hideCorpusDocView',
    'hideReadLaterView',
    'hideTodoTasksView',
    'hideHomeView',
    'feedView',
    'unmountTodoTaskSplit',
    'unmountHomeHub',
    'mountHomeHub',
    'mountTodoTaskSplit',
    'navigate',
    'openReadLaterDialog',
    'state',
  ];
  const brace = fnSource.indexOf('{');
  const body = fnSource.slice(brace + 1, fnSource.lastIndexOf('}'));
  const prelude = locals.map((k) => `var ${k} = __env.${k};`).join('\n');
  // eslint-disable-next-line no-new-func
  return new Function(
    '__env',
    'document',
    `${prelude}\nreturn function (route) {\n${body}\n};`,
  )(env, document);
}

describe('T5 retire #btn-feed / showFeedView user entry', () => {
  describe('index.html — old Builders entry unreachable', () => {
    it('removes or disables #btn-feed so users cannot click the old Builders entry', () => {
      const html = readIndexHtml();
      const hasId = /id\s*=\s*["']btn-feed["']/.test(html);
      if (!hasId) {
        expect(hasId).toBe(false);
        return;
      }
      const buttonMatch =
        html.match(/<button[^>]*id\s*=\s*["']btn-feed["'][^>]*>/i) ||
        html.match(/<[^>]*id\s*=\s*["']btn-feed["'][^>]*>/i);
      expect(buttonMatch, '#btn-feed markup should be parseable').toBeTruthy();
      const tag = buttonMatch[0];
      const disabled =
        /\bdisabled\b/i.test(tag) ||
        /\bhidden\b/i.test(tag) ||
        /aria-hidden\s*=\s*["']true["']/i.test(tag) ||
        /style\s*=\s*["'][^"']*display\s*:\s*none/i.test(tag);
      expect(disabled, '#btn-feed must be removed or disabled').toBe(true);
    });
  });

  describe('main.js — cut click→showFeedView wiring and #btn-feed classList touchpoints', () => {
    it('does not wire #btn-feed click to showFeedView / showArchiveView toggle', () => {
      expect(readMain()).not.toMatch(BTN_FEED_CLICK_WIRING);
    });

    it('showFeedView (if present) does not bare-add active on #btn-feed', () => {
      const show = extractFunctionSource(readMain(), 'showFeedView');
      if (!show) {
        expect(show).toBe('');
        return;
      }
      expect(show).not.toMatch(BARE_BTN_FEED_CLASSLIST);
    });

    it.each(['mountHomeRoute', 'mountTodoTasksRoute', 'mountWorkbench'])(
      '%s does not bare-remove active on #btn-feed',
      (name) => {
        const src = extractFunctionSource(readMain(), name);
        expect(src, `${name} missing`).not.toBe('');
        expect(src).not.toMatch(BARE_BTN_FEED_CLASSLIST);
      },
    );

    it('has no remaining bare #btn-feed classList.add/remove anywhere in main.js', () => {
      expect(readMain()).not.toMatch(BARE_BTN_FEED_CLASSLIST);
    });
  });

  describe('route mounts — tolerate missing #btn-feed node', () => {
    beforeEach(() => {
      document.body.innerHTML = `
        <div id="home-view" style="display:none"></div>
        <div id="corpus-doc-view" style="display:none"></div>
        <div id="read-later-view" style="display:none"></div>
        <div id="todo-tasks-view" style="display:none"></div>
        <div class="layout">
          <main>
            <div id="status"></div>
            <div id="date-heading" style="display:none"></div>
            <div id="doc-list"></div>
            <div id="feed-view" style="display:none"></div>
          </main>
        </div>
      `;
      expect(document.getElementById('btn-feed')).toBeNull();
    });

    afterEach(() => {
      document.body.innerHTML = '';
    });

    it.each(['mountHomeRoute', 'mountTodoTasksRoute', 'mountWorkbench'])(
      '%s does not throw when #btn-feed is absent',
      (name) => {
        const fnSource = extractFunctionSource(readMain(), name);
        expect(fnSource, `${name} missing`).not.toBe('');
        const fn = compileMountFn(fnSource, stubMountEnv());
        expect(() => fn({ params: {} })).not.toThrow();
      },
    );
  });

  describe('no dual entry — Builders content adapter remains', () => {
    /** @type {HTMLElement} */
    let slot;

    beforeEach(() => {
      slot = document.createElement('div');
      document.body.appendChild(slot);
      renderFeedMock.mockReset();
      renderFeedMock.mockImplementation((container) => {
        container.innerHTML = '<div class="feed-mock">feed</div>';
      });
    });

    afterEach(() => {
      slot.remove();
    });

    it('main.js orchestrates Builders via home-entry shell (legacy body mount retired)', () => {
      const main = readMain();
      expect(main).toMatch(/mountHomeEntryShell\s*\(\s*document\.body\b/);
      expect(main).not.toMatch(/mountBuildersAssistantWidget\s*\(\s*document\.body\b/);
      expect(main).toMatch(/createBuildersContentAdapter/);
    });

    it('Builders content adapter mounts feed into the slot (regression)', () => {
      const handle = createBuildersContentAdapter().mount(slot, { host: {} });
      expect(renderFeedMock).toHaveBeenCalledWith(slot);
      expect(slot.querySelector('.feed-mock')).not.toBeNull();
      handle.unmount();
    });

    it('user-reachable Builders path is not via #btn-feed click wiring', () => {
      expect(readMain()).not.toMatch(BTN_FEED_CLICK_WIRING);
      expect(readMain()).toMatch(/createBuildersContentAdapter/);
    });
  });
});
