// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { notifyState, state } from '../../frontend/src/host/state.ts';

const { setDocTitle } = vi.hoisted(() => ({ setDocTitle: vi.fn() }));
vi.mock('../../frontend/src/host/api.ts', async (importOriginal) => ({
  ...(await importOriginal()), setDocTitle,
}));

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
  setDocTitle.mockReset().mockResolvedValue({ ok: true });
  renderDocBody.mockReset();
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
  it('scrolls the title and creation time with the article body while keeping the toolbar outside', () => {
    state.viewer.outletMode = 'open';
    state.viewer.entry = { id: 'n1', title: 'A title', created_at: '20250101120000', project_id: 'p', translations: {} };
    state.viewer.rawText = 'A document';
    mount({ note: 'n1' });

    const reader = container.querySelector('.viewer-body');
    const header = container.querySelector('#md-header');
    const css = readFileSync(join(dirname(fileURLToPath(import.meta.url)), '../../frontend/app.css'), 'utf8');
    expect(css).toMatch(/\.viewer-body\s*\{[^}]*overflow-y:\s*auto;/);
    expect(header).not.toBeNull();
    expect(reader.contains(container.querySelector('[aria-label="Document title"]'))).toBe(true);
    expect(reader.contains(container.querySelector('.notes-document-created'))).toBe(true);
    expect(reader.contains(container.querySelector('#md-body'))).toBe(true);
    expect(reader.contains(header)).toBe(false);
  });

  it('keeps the independent title available while editing the body', () => {
    state.viewer.outletMode = 'open';
    state.viewer.entry = { id: 'n1', title: 'A title', created_at: '20250101120000', project_id: 'p', translations: {} };
    state.viewer.rawText = 'A document';
    state.viewer.editing = true;
    mount({ note: 'n1' });

    const title = container.querySelector('[aria-label="Document title"]');
    expect(title).not.toBeNull();
    expect(title.closest('[hidden], [style*="display: none"]')).toBeNull();
    expect(container.querySelector('#md-edit-area').style.display).not.toBe('none');
  });

  it('keeps the existing Sketch iframe mounted when a document title is saved', async () => {
    const entry = { id: 'n1', title: 'Original', created_at: '20250101120000', project_id: 'p', translations: {} };
    state.viewer.outletMode = 'open';
    state.viewer.entry = entry;
    state.viewer.rawText = 'hello sketch';
    state.index.data = { n1: entry };
    state.index.titleCache = new Map();
    renderDocBody.mockImplementation(() => {
      const frame = document.createElement('iframe');
      frame.title = 'Embedded Sketch';
      document.getElementById('md-body').appendChild(frame);
    });
    mount({ note: 'n1' });

    const body = container.querySelector('#md-body');
    const frame = body.querySelector('iframe');
    const paints = renderDocBody.mock.calls.length;
    const title = container.querySelector('[aria-label="Document title"]');
    expect(frame).not.toBeNull();

    act(() => {
      title.focus();
      Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, 'value').set.call(title, 'Edited title');
      title.dispatchEvent(new Event('input', { bubbles: true }));
    });
    await act(async () => title.blur());

    expect(setDocTitle).toHaveBeenCalledWith('n1', 'Edited title');
    expect(entry.title).toBe('Edited title');
    expect(container.querySelector('#md-body')).toBe(body);
    expect(body.querySelector('iframe')).toBe(frame);
    expect(renderDocBody).toHaveBeenCalledTimes(paints);
  });

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
    const pair = container.querySelector('.sidebar-tool-pair');
    const toolToggle = pair.querySelector('[data-sidebar-toggle]');
    const backBefore = pair.querySelector('a.page-back-home');
    expect(toolToggle.compareDocumentPosition(backBefore) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    const css = readFileSync(join(dirname(fileURLToPath(import.meta.url)), '../../frontend/app.css'), 'utf8');
    expect(css).toMatch(/\.sidebar-layout-tool \.window-drag-strip\.is-chrome \{[\s\S]*?padding-right:\s*10px;/);
    expect(css).toMatch(/\.sidebar-layout-tool \.page-back-home \{[\s\S]*?width:\s*28px;/);
    expect(css).toMatch(/\.sidebar-layout-tool \.page-back-home \{[\s\S]*?padding:\s*0 4px 0 0;/);
    expect(css).toMatch(/\.sidebar-layout-tool \.page-back-home \{[\s\S]*?justify-content:\s*flex-end;/);
    expect(css).toMatch(/\.sidebar-layout-tool \.page-back-home-icon \{[\s\S]*?width:\s*16px;/);
    expect(css).toMatch(/\.sidebar-layout-tool \.page-back-home:hover \{[\s\S]*?background:\s*var\(--bg-hover\);/);
    expect(container.querySelector('#md-header').parentElement.classList.contains('notes-header-slot')).toBe(true);

    toggle();

    const nav = container.querySelector('#notes-sidebar-nav');
    const back = container.querySelector('a.page-back-home');
    expect(nav.hidden).toBe(true);
    expect(nav.contains(back)).toBe(false);
    expect(back.closest('.sidebar-layout-tool')?.hasAttribute('hidden')).toBe(true);
    const restore = container.querySelector('.sidebar-header-restore [data-sidebar-toggle]');
    const title = container.querySelector('#md-panel-title');
    expect(restore.closest('[hidden]')).toBeNull();
    expect(restore.getAttribute('data-tip')).toBe('Show sidebar');
    expect(restore.compareDocumentPosition(title) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
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
