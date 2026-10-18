// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import * as api from '../../frontend/src/host/api.ts';
import { notifyState, state } from '../../frontend/src/host/state.ts';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

vi.mock('../../frontend/src/notes/viewer.ts', () => ({
  closeModal: vi.fn(),
  enterEditMode: vi.fn(),
  exitEditMode: vi.fn(),
  saveDoc: vi.fn(),
  switchLang: vi.fn(),
}));
vi.mock('../../frontend/src/notes/ui/sidebar-resize.ts', () => ({
  initSidebarResize: vi.fn(),
  attachNotesSidebarResize: vi.fn(() => () => {}),
  detachNotesSidebarResize: vi.fn(),
}));
vi.mock('../../frontend/src/notes/ui/viewer/body.tsx', () => ({ renderDocBody: vi.fn() }));
vi.mock('../../frontend/src/notes/ui/comments.tsx', () => ({
  NotesCommentFloatNav: () => null,
  NotesCommentsBar: () => null,
  NotesDeleteZone: () => null,
  openCommentDialog: vi.fn(),
}));
vi.mock('../../frontend/src/notes/ui/sidebar.tsx', () => ({ NotesSidebar: () => null }));
vi.mock('../../frontend/src/notes/ui/cards.tsx', () => ({ NotesDocList: () => null }));

import { NotesMain } from '../../frontend/src/notes/page.tsx';
import { notesSearchNames } from '../../frontend/src/notes/ui/search.tsx';

function installLocalStorageMock(initial = {}) {
  const store = { ...initial };
  globalThis.localStorage = {
    getItem: (k) => (Object.prototype.hasOwnProperty.call(store, k) ? store[k] : null),
    setItem: (k, v) => {
      store[k] = String(v);
    },
    removeItem: (k) => {
      delete store[k];
    },
  };
  return store;
}

function typeInto(input, value) {
  Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(input, value);
  input.dispatchEvent(new Event('input', { bubbles: true }));
}

function press(init, target = document.body) {
  const event = new KeyboardEvent('keydown', { bubbles: true, cancelable: true, ...init });
  act(() => {
    target.dispatchEvent(event);
  });
  return event;
}

describe('notes page search (shared modal)', () => {
  let container;
  let root;
  let searchSpy;
  let store;

  function render(props = {}) {
    act(() => {
      root.render(createElement(NotesMain, { routeParams: {}, ...props }));
      notifyState();
    });
  }

  beforeEach(() => {
    vi.useFakeTimers();
    store = installLocalStorageMock();
    container = document.createElement('div');
    document.body.appendChild(container);
    root = createRoot(container);
    state.workspaces = {
      current: 'default',
      list: [{ workspace_id: 'default', workspace_name: 'Default', is_default: true }],
    };
    state.index.topicTitles = { p1: 'Inbox', p2: 'p2' };
    state.viewer.outletMode = '';
    state.viewer.entry = null;
    state.viewer.createSession = null;
    state.viewer.editing = false;
    state.ui.loadError = '';
    state.ui.activeDate = '';
    state.index.groupedByDate = [];
    state.index.filteredGroups = [];
    searchSpy = vi.spyOn(api, 'searchSpark').mockResolvedValue({
      hits: [
        { id: 'd1', title: 'Alpha doc', workspace_id: 'default', project_id: 'p1', body: 'BODY-SNIPPET' },
        { id: 'd2', title: 'Beta doc', workspace_id: 'default', project_id: 'p2' },
      ],
    });
  });

  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    vi.restoreAllMocks();
    vi.useRealTimers();
  });

  const button = () => container.querySelector('[data-role="notes-search"]');
  const modal = () => container.querySelector('[data-role="search-modal"]');
  const input = () => container.querySelector('[data-role="search-modal-input"]');
  const hits = () => [...container.querySelectorAll('[data-role="search-hit"]')];

  async function search(q) {
    await act(async () => {
      typeInto(input(), q);
      await vi.advanceTimersByTimeAsync(300);
    });
  }

  describe('the entry', () => {
    it('replaces the inline input and dropdown with one button that shows the ⌘K hint', () => {
      render();
      expect(button()).not.toBeNull();
      expect(button().querySelector('kbd').textContent).toBe('⌘K');
      expect(button().getAttribute('aria-keyshortcuts')).toBe('Meta+K');
      expect(container.querySelector('#gs-wb-input')).toBeNull();
      expect(container.querySelector('#gs-wb-dropdown')).toBeNull();
      expect(container.querySelector('input')).toBeNull();
    });

    it('opens the modal when the button is clicked; closed until then', () => {
      render();
      expect(modal()).toBeNull();
      act(() => button().click());
      expect(modal()).not.toBeNull();
      expect(document.activeElement).toBe(input());
      expect(input().getAttribute('placeholder')).toBe('Search this workspace…');
    });
  });

  describe('⌘K', () => {
    it('opens the modal and prevents the default action, and ⌘K again closes it', () => {
      render();
      const first = press({ key: 'k', metaKey: true });
      expect(first.defaultPrevented).toBe(true);
      expect(modal()).not.toBeNull();
      press({ key: 'k', metaKey: true });
      expect(modal()).toBeNull();
    });

    it('also works while a document is open (the toolbar is hidden then)', () => {
      state.viewer.outletMode = 'create';
      render();
      expect(container.querySelector('.notes-page-toolbar').hidden).toBe(true);
      press({ key: 'k', metaKey: true });
      expect(modal()).not.toBeNull();
    });

    it('ignores Ctrl+K, plain k, auto-repeat and IME composition', () => {
      render();
      press({ key: 'k', ctrlKey: true });
      press({ key: 'k' });
      press({ key: 'k', metaKey: true, repeat: true });
      press({ key: 'k', metaKey: true, isComposing: true });
      expect(modal()).toBeNull();
    });

    it('does nothing when the notes route is not the one showing (page stays mounted)', () => {
      render({ active: false });
      const event = press({ key: 'k', metaKey: true });
      expect(modal()).toBeNull();
      expect(event.defaultPrevented).toBe(false);
    });

    it('does not take ⌘O (that is Home\'s New chat)', () => {
      render();
      const event = press({ key: 'o', metaKey: true });
      expect(modal()).toBeNull();
      expect(event.defaultPrevented).toBe(false);
    });

    it('stops listening once the page unmounts', () => {
      render();
      act(() => root.unmount());
      root = createRoot(container);
      const event = press({ key: 'k', metaKey: true });
      expect(event.defaultPrevented).toBe(false);
    });
  });

  describe('leaving the notes route', () => {
    it('closes an open modal, and it stays closed on coming back', () => {
      render();
      press({ key: 'k', metaKey: true });
      expect(modal()).not.toBeNull();
      render({ active: false });
      expect(modal()).toBeNull();
      render({ active: true });
      expect(modal()).toBeNull();
    });

    it('releases the shortcut while away and takes it again on return', () => {
      render();
      render({ active: false });
      press({ key: 'k', metaKey: true });
      expect(modal()).toBeNull();
      render({ active: true });
      press({ key: 'k', metaKey: true });
      expect(modal()).not.toBeNull();
    });
  });

  describe('scope and results', () => {
    it('searches only the current workspace', async () => {
      render();
      press({ key: 'k', metaKey: true });
      await search('alpha');
      expect(searchSpy).toHaveBeenCalledTimes(1);
      expect(searchSpy).toHaveBeenCalledWith('alpha', 8, 'default', { allWorkspaces: false });
    });

    it('follows the current workspace', async () => {
      state.workspaces = { ...state.workspaces, current: 'mem' };
      render();
      press({ key: 'k', metaKey: true });
      await search('alpha');
      expect(searchSpy).toHaveBeenCalledWith('alpha', 8, 'mem', { allWorkspaces: false });
    });

    it('strips a leading # before searching', async () => {
      render();
      press({ key: 'k', metaKey: true });
      await search('#topic-name');
      expect(searchSpy).toHaveBeenCalledWith('topic-name', 8, 'default', { allWorkspaces: false });
    });

    it('shows the title and just the project under it, with no body snippet', async () => {
      render();
      press({ key: 'k', metaKey: true });
      await search('doc');
      expect(hits()).toHaveLength(2);
      expect(hits()[0].querySelector('.search-modal-row-title').textContent).toBe('Alpha doc');
      expect(hits()[0].querySelector('.search-modal-row-sub').textContent).toBe('Inbox');
      expect(container.textContent).not.toContain('BODY-SNIPPET');
      expect(container.textContent).not.toContain('Default /');
    });

    it('leaves the line out for a project that only has its id as a title', async () => {
      render();
      press({ key: 'k', metaKey: true });
      await search('doc');
      expect(hits()[1].querySelector('.search-modal-row-sub')).toBeNull();
    });

    it('reports an empty result, an unbuilt index, and an error', async () => {
      render();
      press({ key: 'k', metaKey: true });
      searchSpy.mockResolvedValueOnce({ hits: [] });
      await search('none');
      expect(container.textContent).toContain('No related results');
      searchSpy.mockResolvedValueOnce({ error: 'not_indexed' });
      await search('idx');
      expect(container.textContent).toContain('Index not built yet.');
      searchSpy.mockResolvedValueOnce({ error: 'unavailable' });
      await search('err');
      expect(container.textContent).toContain('Search error');
    });
  });

  describe('recent searches', () => {
    it('an empty query lists the shared history (gs-history-wb), with a delete ×', () => {
      store['gs-history-wb'] = JSON.stringify(['agentic design', 'read later']);
      render();
      press({ key: 'k', metaKey: true });
      const items = [...container.querySelectorAll('[data-role="search-history-item"]')];
      expect(items.map((el) => el.textContent.replace('×', '').trim())).toEqual(['agentic design', 'read later']);
      expect(container.querySelectorAll('[data-role="search-history-remove"]')).toHaveLength(2);
    });

    it('opening a hit stores the query without its # prefix under gs-history-wb', async () => {
      render();
      press({ key: 'k', metaKey: true });
      await search('#stored-query');
      act(() => hits()[0].click());
      const stored = JSON.parse(store['gs-history-wb'] || '[]');
      expect(stored).toContain('stored-query');
      expect(stored.some((q) => q.startsWith('#'))).toBe(false);
    });
  });

  describe('opening a document and closing', () => {
    it('clicking a row dispatches cta:open-entry with the id only, and closes', async () => {
      render();
      press({ key: 'k', metaKey: true });
      await search('alpha');
      const handler = vi.fn();
      document.addEventListener('cta:open-entry', handler);
      act(() => hits()[0].click());
      document.removeEventListener('cta:open-entry', handler);
      expect(handler).toHaveBeenCalledTimes(1);
      expect(handler.mock.calls[0][0].detail).toEqual({ id: 'd1' });
      expect(modal()).toBeNull();
    });

    it('Enter opens the highlighted row and closes', async () => {
      render();
      press({ key: 'k', metaKey: true });
      await search('alpha');
      const handler = vi.fn();
      document.addEventListener('cta:open-entry', handler);
      press({ key: 'ArrowDown' }, input());
      press({ key: 'Enter' }, input());
      document.removeEventListener('cta:open-entry', handler);
      expect(handler.mock.calls[0][0].detail).toEqual({ id: 'd2' });
      expect(modal()).toBeNull();
    });

    it('esc and a click outside close it', () => {
      render();
      press({ key: 'k', metaKey: true });
      press({ key: 'Escape' }, input());
      expect(modal()).toBeNull();
      act(() => button().click());
      act(() => {
        modal().dispatchEvent(new MouseEvent('mousedown', { bubbles: true }));
      });
      expect(modal()).toBeNull();
    });
  });

  describe('notesSearchNames', () => {
    it('gives the project title, and nothing for an unknown or id-only project', () => {
      const names = notesSearchNames({ p1: 'Inbox', p2: 'p2' });
      expect(names.project('default', 'p1')).toBe('Inbox');
      expect(names.project('default', 'p2')).toBe('');
      expect(names.project('default', 'zzz')).toBe('');
      expect(names.workspace('default')).toBe('');
    });
  });

  it('search.tsx does not import attachDigestTooltip', () => {
    const src = readFileSync(join(repoRoot, 'frontend/src/notes/ui/search.tsx'), 'utf8');
    expect(src).not.toMatch(/attachDigestTooltip/);
    expect(src).not.toMatch(/digest-tooltip/);
  });
});
