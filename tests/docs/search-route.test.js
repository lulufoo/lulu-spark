// @vitest-environment jsdom
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import * as api from '../../frontend/src/host/api.ts';
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

// The real Home page owns ⌘O / ⌘K too (LW-446); this test is about the notes side only.
vi.mock('../../frontend/src/home/page.tsx', () => ({ HomePage: () => null }));

import { ShellPages } from '../../frontend/src/shell-pages.tsx';

function press(init) {
  const event = new KeyboardEvent('keydown', { bubbles: true, cancelable: true, ...init });
  act(() => {
    document.body.dispatchEvent(event);
  });
  return event;
}

/**
 * The shell keeps the notes page mounted under every route (it is only display:none), so the
 * notes ⌘K must follow the route, not the mount.
 */
describe('notes search follows the route in the shell', () => {
  let container;
  let root;

  const show = (routeName) => {
    act(() => {
      root.render(createElement(ShellPages, { routeName, routeParams: {} }));
      notifyState();
    });
  };
  const modal = () => container.querySelector('[data-role="search-modal"]');

  beforeEach(() => {
    globalThis.localStorage = { getItem: () => null, setItem: () => {}, removeItem: () => {} };
    container = document.createElement('div');
    document.body.appendChild(container);
    root = createRoot(container);
    state.workspaces = { current: 'default', list: [] };
    state.viewer.outletMode = '';
    state.viewer.entry = null;
    state.viewer.createSession = null;
    state.ui.loadError = '';
    vi.spyOn(api, 'searchSpark').mockResolvedValue({ hits: [] });
  });

  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    vi.restoreAllMocks();
  });

  it('⌘K does nothing on the home route even though the notes page is mounted', () => {
    show('home');
    expect(container.querySelector('[data-role="notes-search"]')).not.toBeNull();
    const event = press({ key: 'k', metaKey: true });
    expect(modal()).toBeNull();
    expect(event.defaultPrevented).toBe(false);
  });

  it('⌘K opens the modal on the spark route', () => {
    show('spark');
    press({ key: 'k', metaKey: true });
    expect(modal()).not.toBeNull();
  });

  it('navigating away closes the modal; coming back does not reopen it', () => {
    show('spark');
    press({ key: 'k', metaKey: true });
    expect(modal()).not.toBeNull();
    show('home');
    expect(modal()).toBeNull();
    show('spark');
    expect(modal()).toBeNull();
  });

  it('read-later is not a notes route either', () => {
    show('read-later');
    press({ key: 'k', metaKey: true });
    expect(modal()).toBeNull();
  });
});
