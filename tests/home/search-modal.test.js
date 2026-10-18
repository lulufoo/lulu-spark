// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import * as api from '../../frontend/src/host/api.ts';
import { state } from '../../frontend/src/host/state.ts';
import { mountHomeHub } from '../../frontend/src/home/hub.tsx';
import { homeSearchNames } from '../../frontend/src/home/state/search-names.ts';
import { addSearchHistory, SEARCH_HISTORY_KEY } from '../../frontend/src/shared/search-modal/history.ts';

const openDocById = vi.hoisted(() => vi.fn());
vi.mock('../../frontend/src/home/commands/open-doc-link.ts', async (importOriginal) => ({
  ...(await importOriginal()),
  openDocById,
}));

function installLocalStorageMock() {
  const store = {};
  globalThis.localStorage = {
    getItem: (k) => (Object.prototype.hasOwnProperty.call(store, k) ? store[k] : null),
    setItem: (k, v) => {
      store[k] = String(v);
    },
    removeItem: (k) => {
      delete store[k];
    },
  };
}

function typeInto(input, value) {
  Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(input, value);
  input.dispatchEvent(new Event('input', { bubbles: true }));
}

const WORKSPACES = [
  { workspace_id: 'default', workspace_name: 'Default', is_default: true },
  { workspace_id: 'mem', workspace_name: 'Memory', is_default: false },
];

describe('Home search modal', () => {
  let container;
  let cleanup;
  let searchSpy;
  let listBefore;

  beforeEach(() => {
    installLocalStorageMock();
    openDocById.mockReset();
    container = document.createElement('div');
    document.body.appendChild(container);
    window.__TAURI__ = { event: { listen: vi.fn(async () => vi.fn()) } };
    listBefore = state.workspaces.list;
    state.workspaces.list = WORKSPACES;
    vi.spyOn(api, 'createChannel').mockImplementation(async (onmessage) => ({ onmessage }));
    vi.spyOn(api, 'invoke').mockImplementation(async (cmd) => {
      if (cmd === 'query_binding') return { state: 'bound' };
      if (cmd === 'list_chat_sessions') return { sessions: [], current_session_id: '' };
      return {};
    });
    vi.spyOn(api, 'fetchProjects').mockImplementation(async (id) => ({
      projects: id === 'mem' ? [{ project_id: 'p2', project_name: 'Notes' }] : [{ project_id: 'p1', project_name: 'Inbox' }],
    }));
    searchSpy = vi.spyOn(api, 'searchSpark').mockResolvedValue({
      hits: [
        { id: 'd1', title: 'Alpha doc', workspace_id: 'default', project_id: 'p1', body: 'BODY-SNIPPET' },
        { id: 'd2', title: 'Beta doc', workspace_id: 'mem', project_id: 'p2' },
      ],
    });
    cleanup = mountHomeHub(container, { navigate: vi.fn() });
  });

  afterEach(() => {
    cleanup?.();
    vi.restoreAllMocks();
    state.workspaces.list = listBefore;
    container.remove();
    delete window.__TAURI__;
  });

  const modal = () => container.querySelector('[data-role="search-modal"]');
  const input = () => container.querySelector('[data-role="search-modal-input"]');
  const hits = () => [...container.querySelectorAll('[data-role="search-hit"]')];

  async function openModal() {
    container.querySelector('[data-role="sidebar-search"]').click();
    await vi.waitFor(() => expect(modal()).not.toBeNull());
  }

  it('is closed until the search icon is clicked', async () => {
    expect(modal()).toBeNull();
    await openModal();
    expect(document.activeElement).toBe(input());
  });

  it('opens from the icon on either tab', async () => {
    container.querySelector('.home-sidebar-tab[data-pane="chat"]').click();
    await vi.waitFor(() => expect(container.querySelector('.home-chat-pane').hidden).toBe(false));
    await openModal();
  });

  it('empty query lists the recent searches; clicking one re-runs it', async () => {
    addSearchHistory('alpha');
    await openModal();
    expect([...container.querySelectorAll('[data-role="search-history-item"]')].map((el) => el.dataset.q)).toEqual(['alpha']);
    container.querySelector('[data-role="search-history-item"]').click();
    await vi.waitFor(() => expect(hits()).toHaveLength(2));
    expect(input().value).toBe('alpha');
  });

  it('the delete × removes a recent search from the shared list', async () => {
    addSearchHistory('keep');
    addSearchHistory('drop');
    await openModal();
    container.querySelector('[data-role="search-history-remove"][data-q="drop"]').click();
    await vi.waitFor(() =>
      expect(JSON.parse(localStorage.getItem(SEARCH_HISTORY_KEY))).toEqual(['keep']),
    );
  });

  it('searches all workspaces and shows "workspace / project" under each title, without the body', async () => {
    await openModal();
    typeInto(input(), 'doc');
    await vi.waitFor(() => expect(hits()).toHaveLength(2), { timeout: 2000 });
    expect(searchSpy).toHaveBeenCalledWith('doc', 8, '', { allWorkspaces: true });
    expect(hits().map((r) => r.querySelector('.search-modal-row-title').textContent)).toEqual(['Alpha doc', 'Beta doc']);
    await vi.waitFor(() =>
      expect(hits().map((r) => r.querySelector('.search-modal-row-sub')?.textContent)).toEqual([
        'Default / Inbox',
        'Memory / Notes',
      ]),
    );
    expect(modal().textContent).not.toContain('BODY-SNIPPET');
  });

  it('Enter opens the highlighted document through the Home opener and closes the modal', async () => {
    await openModal();
    typeInto(input(), 'doc');
    await vi.waitFor(() => expect(hits()).toHaveLength(2), { timeout: 2000 });
    input().dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true, cancelable: true }));
    await vi.waitFor(() => expect(hits()[1].dataset.active).toBe('true'));
    input().dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true }));
    expect(openDocById).toHaveBeenCalledWith('d2');
    await vi.waitFor(() => expect(modal()).toBeNull());
    expect(JSON.parse(localStorage.getItem(SEARCH_HISTORY_KEY))).toEqual(['doc']);
  });

  it('esc closes it', async () => {
    await openModal();
    input().dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }));
    await vi.waitFor(() => expect(modal()).toBeNull());
  });

  it('keeps New chat and the tabs working underneath', async () => {
    await openModal();
    expect(container.querySelector('[data-role="new-session"]')).not.toBeNull();
    expect(container.querySelectorAll('.home-sidebar-tab')).toHaveLength(2);
  });
});

describe('homeSearchNames', () => {
  const names = homeSearchNames(WORKSPACES, { mem: [{ id: 'p2', title: 'Notes', icon: '' }] });

  it('names workspaces and projects the sidebar has loaded', () => {
    expect(names.workspace('mem')).toBe('Memory');
    expect(names.project('mem', 'p2')).toBe('Notes');
  });

  it('gives "" for what is not loaded, so the row leaves it out', () => {
    expect(names.workspace('gone')).toBe('');
    expect(names.project('mem', 'gone')).toBe('');
    expect(names.project('gone', 'p2')).toBe('');
  });
});
