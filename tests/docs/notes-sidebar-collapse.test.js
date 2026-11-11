// @vitest-environment jsdom
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

import { renderDocBody } from '../../frontend/src/notes/ui/viewer/body.tsx';
import { NotesMain } from '../../frontend/src/notes/page.tsx';
import { NotesFrame } from '../../frontend/src/notes/ui/notes-frame.tsx';
import { sidebarCollapsedStore } from '../../frontend/src/app-shell/state/sidebar-collapsed.ts';
import { notesSidebarWidthStore } from '../../frontend/src/notes/state/sidebar-width.ts';

const WIDTH_KEY = 'cta_sidebar_width';

let container;
let root;

function seedHost() {
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
  state.viewer.outletMode = '';
  state.viewer.entry = null;
  state.viewer.rawText = '';
  state.viewer.bodyPaintKey = 1;
  state.viewer.createSession = null;
  state.viewer.editing = false;
  state.viewer.loading = false;
  state.viewer.loadError = '';
  state.viewer.layer = 'raw';
  state.viewer.lang = 'en';
}

function mount(routeParams) {
  act(() => {
    notifyState();
    root.render(
      createElement(NotesFrame, { activeDate: '20250101' }, createElement(NotesMain, { routeParams, active: true })),
    );
  });
}

function toggle() {
  act(() => {
    container.querySelector('[data-sidebar-toggle]').click();
  });
}

let originalStorage;

function installStorage() {
  const store = { [WIDTH_KEY]: '180' };
  globalThis.localStorage = {
    getItem: (key) => (Object.prototype.hasOwnProperty.call(store, key) ? store[key] : null),
    setItem: (key, value) => {
      store[key] = String(value);
    },
    removeItem: (key) => {
      delete store[key];
    },
    clear: () => {
      for (const key of Object.keys(store)) delete store[key];
    },
  };
}

beforeEach(() => {
  globalThis.IS_REACT_ACT_ENVIRONMENT = true;
  originalStorage = globalThis.localStorage;
  installStorage();
  sidebarCollapsedStore.set(false);
  notesSidebarWidthStore.set(221);
  seedHost();
  container = document.createElement('div');
  document.body.appendChild(container);
  root = createRoot(container);
});

afterEach(() => {
  act(() => root?.unmount());
  container?.remove();
  sidebarCollapsedStore.set(false);
  notesSidebarWidthStore.set(221);
  globalThis.localStorage = originalStorage;
});

describe('list and article sidebar collapse', () => {
  it('keeps the open article body and the saved width when the nav hides', () => {
    state.viewer.outletMode = 'open';
    state.viewer.entry = { id: 'n1', created_at: '20250101120000', project_id: 'p', translations: {} };
    state.viewer.rawText = 'hello sketch';
    mount({ note: 'n1' });

    const body = container.querySelector('#md-body');
    const navContent = container.querySelector('[data-sidebar-content]');
    const frame = document.createElement('iframe');
    body.appendChild(frame);
    const paints = renderDocBody.mock.calls.length;

    expect(container.querySelector('#notes-sidebar-nav').hidden).toBe(false);
    expect(container.querySelector('#md-header').parentElement.classList.contains('notes-header-slot')).toBe(true);

    toggle();

    const nav = container.querySelector('#notes-sidebar-nav');
    const back = container.querySelector('a.page-back-home');
    expect(nav.hidden).toBe(true);
    expect(nav.contains(back)).toBe(false);
    expect(back.closest('[hidden]')).toBeNull();
    expect(localStorage.getItem(WIDTH_KEY)).toBe('180');
    expect(localStorage.getItem('sidebar_collapsed')).toBe('true');
    expect(container.querySelector('#md-body')).toBe(body);
    expect(body.contains(frame)).toBe(true);
    expect(container.querySelector('[data-sidebar-content]')).toBe(navContent);
    expect(renderDocBody.mock.calls.length).toBe(paints);
    expect(container.querySelector('.sidebar-layout').style.getPropertyValue('--sidebar-layout-width')).toBe('180px');
  });

  it('keeps the list toolbar outside the hidden nav', () => {
    mount({});
    const toolbar = container.querySelector('.notes-page-toolbar');
    expect(toolbar.parentElement.classList.contains('notes-header-slot')).toBe(true);
    toggle();
    expect(container.querySelector('.notes-page-toolbar')).toBe(toolbar);
    expect(container.querySelector('#notes-sidebar-nav').contains(toolbar)).toBe(false);
    expect(toolbar.hidden).toBe(false);
  });
});
