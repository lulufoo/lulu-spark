// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { notifyState, state } from '../../frontend/src/host/state.ts';

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
vi.mock('../../frontend/src/notes/ui/cards.tsx', () => ({ NotesDocList: () => null }));
vi.mock('../../frontend/src/notes/ui/search.tsx', () => ({
  NotesSearchButton: () => createElement('button', { className: 'notes-search-btn' }),
  NotesSearchModal: () => null,
  useNotesSearch: () => ({ open: false, openSearch: () => {}, closeSearch: () => {} }),
}));

import * as api from '../../frontend/src/host/api.ts';
import { mountHomeHub } from '../../frontend/src/home/hub.tsx';
import { NotesMain } from '../../frontend/src/notes/page.tsx';
import { NotesFrame } from '../../frontend/src/notes/ui/notes-frame.tsx';
import { SketchPreview } from '../../frontend/src/board/ui/sketch-preview.tsx';
import { sidebarCollapsedStore } from '../../frontend/src/app-shell/state/sidebar-collapsed.ts';
import { dirTreeStore } from '../../frontend/src/notes/state/dir-tree.ts';
import { notesSidebarWidthStore } from '../../frontend/src/notes/state/sidebar-width.ts';
import { homeSidebarWidthStore } from '../../frontend/src/home/state/sidebar-width.ts';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const NOTES_WIDTH = 'cta_sidebar_width';
const HOME_WIDTH = 'home_chat_sidebar_width';

let container;
let root;
let originalStorage;
let storage;

function installStorage(seed) {
  storage = { ...seed };
  globalThis.localStorage = {
    getItem: (key) => (Object.prototype.hasOwnProperty.call(storage, key) ? storage[key] : null),
    setItem: (key, value) => {
      storage[key] = String(value);
    },
    removeItem: (key) => {
      delete storage[key];
    },
  };
}

function seedNotes() {
  state.workspaces = {
    current: 'default',
    list: [{ workspace_id: 'default', workspace_name: 'Default', view_mode: 'timeline', is_default: true }],
  };
  state.index.groupedByDate = [];
  state.index.filteredGroups = [{ date: '20250101', entries: [] }];
  state.index.topicTitles = {};
  state.index.topicOrder = [];
  state.ui.activeDate = '20250101';
  state.ui.activeTopic = '';
  state.ui.loadError = '';
  state.viewer.outletMode = 'open';
  state.viewer.entry = { id: 'n1', created_at: '20250101120000', project_id: 'p', translations: {} };
  state.viewer.rawText = 'hello sketch';
  state.viewer.bodyPaintKey = 1;
  state.viewer.createSession = null;
  state.viewer.editing = true;
  state.viewer.loading = false;
  state.viewer.loadError = '';
  state.viewer.layer = 'raw';
  state.viewer.lang = 'en';
}

function mountNotes(routeParams) {
  act(() => {
    notifyState();
    root.render(
      createElement(NotesFrame, { activeDate: '20250101' }, createElement(NotesMain, { routeParams, active: true })),
    );
  });
}

function pressToggle() {
  act(() => {
    container.querySelector('[data-sidebar-toggle]').click();
  });
}

beforeEach(() => {
  globalThis.IS_REACT_ACT_ENVIRONMENT = true;
  originalStorage = globalThis.localStorage;
  installStorage({ [NOTES_WIDTH]: '180', [HOME_WIDTH]: '180' });
  sidebarCollapsedStore.set(false);
  notesSidebarWidthStore.set(221);
  homeSidebarWidthStore.set(248);
  dirTreeStore.set({ expanded: ['proj/a'] });
  container = document.createElement('div');
  document.body.appendChild(container);
  root = createRoot(container);
});

afterEach(() => {
  act(() => root?.unmount());
  container?.remove();
  sidebarCollapsedStore.set(false);
  notesSidebarWidthStore.set(221);
  homeSidebarWidthStore.set(248);
  dirTreeStore.set({ expanded: [] });
  globalThis.localStorage = originalStorage;
});

describe('sidebar collapse regression', () => {
  it('keeps the article editor, tree expansion, and english toggle contract', () => {
    seedNotes();
    mountNotes({ note: 'n1' });
    const editor = container.querySelector('#md-edit-area');
    const body = container.querySelector('#md-body');
    editor.value = 'unsaved sentence';
    const toggle = container.querySelector('[data-sidebar-toggle]');
    expect(toggle.tagName).toBe('BUTTON');
    expect(toggle.getAttribute('type')).toBe('button');
    expect(toggle.getAttribute('data-tip')).toBe('Hide sidebar');
    expect(toggle.getAttribute('aria-expanded')).toBe('true');
    expect(toggle.getAttribute('aria-controls')).toBe('notes-sidebar-nav');

    pressToggle();

    const nav = container.querySelector('#notes-sidebar-nav');
    expect(nav.hidden).toBe(true);
    expect(nav.hasAttribute('inert')).toBe(true);
    expect(container.querySelector('#sidebar-resizer').hidden).toBe(true);
    expect(container.querySelector('#sidebar-resizer').tabIndex).toBe(-1);
    expect(container.querySelector('[data-sidebar-toggle]').getAttribute('data-tip')).toBe('Show sidebar');
    expect(container.querySelector('[data-sidebar-toggle]').getAttribute('aria-expanded')).toBe('false');
    expect(container.querySelector('#md-edit-area')).toBe(editor);
    expect(editor.value).toBe('unsaved sentence');
    expect(container.querySelector('#md-body')).toBe(body);
    expect(dirTreeStore.getSnapshot().expanded).toEqual(['proj/a']);
    expect(localStorage.getItem(NOTES_WIDTH)).toBe('180');
    expect(container.querySelector('a.page-back-home').closest('[hidden]')).toBeNull();
  });

  it('keeps list scroll and restores the shared preference on the home page', async () => {
    seedNotes();
    state.viewer.outletMode = '';
    state.viewer.entry = null;
    state.viewer.rawText = '';
    state.viewer.editing = false;
    mountNotes({});
    const list = container.querySelector('#doc-list');
    list.scrollTop = 48;
    pressToggle();
    expect(container.querySelector('#doc-list')).toBe(list);
    expect(list.scrollTop).toBe(48);
    act(() => root.unmount());
    root = null;

    window.__TAURI__ = { event: { listen: vi.fn(async () => vi.fn()) } };
    vi.spyOn(api, 'createChannel').mockImplementation(async (onmessage) => ({ onmessage }));
    vi.spyOn(api, 'invoke').mockImplementation(async (cmd) => {
      if (cmd === 'query_binding') return { state: 'bound' };
      if (cmd === 'list_chat_sessions') {
        return {
          sessions: [{ session_id: 's1', title: 'Kept' }],
          current_session_id: 's1',
        };
      }
      return {};
    });
    const home = document.createElement('div');
    document.body.appendChild(home);
    const stop = mountHomeHub(home, { navigate: vi.fn(), openReadLater: vi.fn() });
    await vi.waitFor(() => {
      expect(home.querySelector('#home-sidebar-nav').hidden).toBe(true);
    });
    expect(localStorage.getItem(HOME_WIDTH)).toBe('180');
    expect(localStorage.getItem(NOTES_WIDTH)).toBe('180');
    expect(home.querySelector('[data-sidebar-toggle]').getAttribute('data-tip')).toBe('Show sidebar');
    expect(home.querySelector('.home-chat-sidebar-resizer').hidden).toBe(true);
    const input = home.querySelector('[data-role="input"]');
    input.textContent = 'home draft';
    act(() => {
      home.querySelector('[data-sidebar-toggle]').click();
    });
    expect(home.querySelector('#home-sidebar-nav').hidden).toBe(false);
    expect(home.querySelector('[data-role="input"]')).toBe(input);
    expect(input.textContent).toBe('home draft');
    stop();
    home.remove();
    vi.restoreAllMocks();
    delete window.__TAURI__;
  });

  it('does not rebuild a sketch frame when its parent width changes', () => {
    const host = document.createElement('div');
    document.body.appendChild(host);
    const sketchRoot = createRoot(host);
    const props = {
      html: '<p>Hello</p>',
      pageUrl: 'https://luluboard.app/view#b:b_0123abcd',
      appearance: 'light',
    };
    const paint = (width) => {
      sketchRoot.render(createElement('div', { style: { width } }, createElement(SketchPreview, props)));
    };
    act(() => paint('400px'));
    const frame = host.querySelector('iframe');
    const src = frame.getAttribute('src');
    const posts = vi.fn();
    frame.contentWindow.postMessage = posts;
    act(() => paint('800px'));
    expect(host.querySelector('iframe')).toBe(frame);
    expect(frame.getAttribute('src')).toBe(src);
    expect(posts).not.toHaveBeenCalled();
    act(() => sketchRoot.unmount());
    host.remove();
  });

  it('keeps the 56px bar, token colors, and one toggle hover', () => {
    const app = readFileSync(join(repoRoot, 'frontend/app.css'), 'utf8');
    const home = readFileSync(join(repoRoot, 'frontend/home.css'), 'utf8');
    const light = readFileSync(join(repoRoot, 'frontend/theme-light.css'), 'utf8');
    const dark = readFileSync(join(repoRoot, 'frontend/theme-dark.css'), 'utf8');
    expect(app).toMatch(/--app-header-height:\s*56px/);
    expect(app).toMatch(/grid-template-rows:\s*var\(--app-header-height\)\s+minmax\(0,\s*1fr\)/);
    expect(app).toMatch(/\.sidebar-layout\[data-collapsed="true"\] \.sidebar-layout-content \{[\s\S]*?grid-column:\s*1 \/ -1;/);
    expect(app).toMatch(/\.notes-sidebar-tool \{[^}]*background:\s*var\(--bg-surface\);/);
    expect(home).toMatch(/\.home-sidebar-tool \{[^}]*background:\s*var\(--bg-canvas\);/);
    expect(app.match(/\.sidebar-toggle:hover/g)).toHaveLength(1);
    expect(home).not.toMatch(/\.sidebar-toggle:hover/);
    expect(app).toMatch(/\.notes-page-toolbar #date-heading \{[\s\S]*?text-overflow:\s*ellipsis;/);
    expect(home).toMatch(/\.home-chat-title \{[\s\S]*?text-overflow:\s*ellipsis;/);
    for (const theme of [light, dark]) {
      expect(theme).toMatch(/--bg-canvas:/);
      expect(theme).toMatch(/--bg-surface:/);
      expect(theme).toMatch(/--border-default:/);
      expect(theme).toMatch(/--border-muted:/);
    }
  });
});
