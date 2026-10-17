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
vi.mock('../../frontend/src/notes/ui/sidebar.tsx', () => ({ NotesSidebar: () => null }));
vi.mock('../../frontend/src/notes/ui/cards.tsx', () => ({ NotesDocList: () => null }));
vi.mock('../../frontend/src/notes/ui/search.tsx', () => ({
  SparkSearch: () => createElement('div', { className: 'gs-search-wrap' }, createElement('input', { id: 'gs-wb-input' })),
}));

import { NotesMain } from '../../frontend/src/notes/page.tsx';

let container;
let root;

function setView(viewMode) {
  state.workspaces = {
    current: 'default',
    list: [{ id: 'default', name: 'Default', view_mode: viewMode, is_default: true }],
  };
}

function mount() {
  act(() => {
    root.render(createElement(NotesMain, { routeParams: {} }));
    notifyState();
  });
}

beforeEach(() => {
  container = document.createElement('div');
  document.body.appendChild(container);
  root = createRoot(container);
  state.viewer.outletMode = '';
  state.viewer.entry = null;
  state.viewer.createSession = null;
  state.viewer.editing = false;
  state.ui.loadError = '';
  state.ui.activeDate = '';
  state.index.groupedByDate = [];
  state.index.filteredGroups = [];
});

afterEach(() => {
  act(() => root.unmount());
  container.remove();
});

const toolbar = () => container.querySelector('.notes-page-toolbar');

describe('nothing selected: the hint shares the search box row', () => {
  it('directory view: the hint is in the toolbar before the search, with no arrow', () => {
    setView('directory');
    mount();
    const hint = toolbar().querySelector('#date-heading');
    expect(hint.textContent).toBe('Select a document in the tree');
    expect(hint.classList.contains('date-heading-hint')).toBe(true);
    expect(toolbar().querySelector('.gs-search-wrap')).not.toBeNull();
    expect(hint.compareDocumentPosition(toolbar().querySelector('.gs-search-wrap')) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  });

  it('timeline view: the date hint, no arrow', () => {
    setView('timeline');
    mount();
    expect(toolbar().querySelector('#date-heading').textContent).toBe('Select a date to view documents');
  });

  it('there is no separate hint row and no arrow anywhere on the page', () => {
    setView('directory');
    mount();
    expect(container.querySelector('#status').textContent).toBe('');
    expect(container.textContent).not.toMatch(/←/);
  });

  it('a load error still shows in #status with Retry', () => {
    setView('directory');
    state.ui.loadError = 'boom';
    mount();
    expect(container.querySelector('#status').textContent).toContain('boom');
    expect(container.querySelector('#status button').textContent).toBe('Retry');
  });
});

describe('something selected: the heading is back', () => {
  it('shows the day with its item count, not the hint', () => {
    setView('timeline');
    state.index.groupedByDate = [{ date: '20260710', entries: [{ id: 'a', entry: {} }] }];
    state.index.filteredGroups = state.index.groupedByDate;
    state.ui.activeDate = '20260710';
    mount();
    const heading = toolbar().querySelector('#date-heading');
    expect(heading.classList.contains('date-heading-hint')).toBe(false);
    expect(heading.textContent).toMatch(/1 items/);
    expect(heading.textContent).not.toMatch(/Select a/);
  });
});
