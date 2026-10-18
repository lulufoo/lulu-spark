// @vitest-environment jsdom
import { createElement } from 'react';
import { flushSync } from 'react-dom';
import { createRoot } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const apiMocks = vi.hoisted(() => ({ searchSpark: vi.fn(), searchChats: vi.fn() }));
vi.mock('../../frontend/src/host/api.ts', () => ({
  searchSpark: (...args) => apiMocks.searchSpark(...args),
  searchChats: (...args) => apiMocks.searchChats(...args),
}));

const { SearchModal } = await import('../../frontend/src/shared/search-modal/search-modal.tsx');
const { addSearchHistory, getSearchHistory, CHAT_SEARCH_HISTORY_KEY, SEARCH_HISTORY_KEY } = await import(
  '../../frontend/src/shared/search-modal/history.ts'
);

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

const NAMES = { workspace: () => '', project: () => '' };
const ALL = { kind: 'all-workspaces' };
const secs = (y, mo, d, h = 12) => new Date(y, mo - 1, d, h).getTime() / 1000;
const THIS_YEAR = new Date().getFullYear();
const CHAT_HITS = [
  { session_id: 's1', title: 'Fix the login redirect', updated_at: secs(THIS_YEAR, 3, 4) },
  { session_id: 's2', title: 'Old review', updated_at: secs(2019, 12, 31) },
];

function typeInto(input, value) {
  Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(input, value);
  input.dispatchEvent(new Event('input', { bubbles: true }));
}

function key(input, k) {
  input.dispatchEvent(new KeyboardEvent('keydown', { key: k, bubbles: true, cancelable: true }));
}

describe('SearchModal in chats mode', () => {
  let host;
  let root;
  let props;

  function render(next = {}) {
    props = {
      open: true,
      mode: 'chats',
      scope: ALL,
      names: NAMES,
      onOpenDoc: vi.fn(),
      onOpenChat: vi.fn(),
      onClose: vi.fn(),
      ...props,
      ...next,
    };
    flushSync(() => root.render(createElement(SearchModal, props)));
  }
  const $ = (sel) => host.querySelector(sel);
  const $$ = (sel) => [...host.querySelectorAll(sel)];
  const input = () => $('[data-role="search-modal-input"]');

  async function search(text) {
    typeInto(input(), text);
    await vi.waitFor(() => expect(apiMocks.searchChats).toHaveBeenCalled(), { timeout: 2000 });
  }

  beforeEach(() => {
    installLocalStorageMock();
    apiMocks.searchSpark.mockReset();
    apiMocks.searchChats.mockReset();
    apiMocks.searchChats.mockResolvedValue({ hits: CHAT_HITS });
    host = document.createElement('div');
    document.body.appendChild(host);
    root = createRoot(host);
    props = {};
  });

  afterEach(() => {
    flushSync(() => root.unmount());
    host.remove();
  });

  it('says it searches chats', () => {
    render();
    expect($('[role="dialog"]').getAttribute('aria-label')).toBe('Search chats');
    expect(input().placeholder).toBe('Search chats by title…');
    expect($('.search-modal-status').textContent).toBe('Type to search chats.');
  });

  it('searches chats only, with the result limit, and never the documents', async () => {
    render();
    await search('login');
    expect(apiMocks.searchChats).toHaveBeenCalledWith('login', 8);
    expect(apiMocks.searchSpark).not.toHaveBeenCalled();
  });

  it('shows the title and the date of each chat, the year only when it is not this year', async () => {
    render();
    await search('login');
    await vi.waitFor(() => expect($$('[data-role="search-hit"]')).toHaveLength(2));
    const rows = $$('[data-role="search-hit"]');
    expect(rows.map((r) => r.querySelector('.search-modal-row-title').textContent)).toEqual([
      'Fix the login redirect',
      'Old review',
    ]);
    expect(rows.map((r) => r.querySelector('.search-modal-row-sub').textContent)).toEqual([
      'Mar 4',
      'Dec 31, 2019',
    ]);
    expect(rows.map((r) => r.dataset.sessionId)).toEqual(['s1', 's2']);
  });

  it('choosing a row opens that chat (not a document), saves the query, and closes', async () => {
    render();
    await search('login');
    await vi.waitFor(() => expect($$('[data-role="search-hit"]')).toHaveLength(2));
    key(input(), 'ArrowDown');
    await vi.waitFor(() => expect($$('[data-role="search-hit"]')[1].dataset.active).toBe('true'));
    key(input(), 'Enter');
    expect(props.onOpenChat).toHaveBeenCalledWith('s2');
    expect(props.onOpenDoc).not.toHaveBeenCalled();
    expect(props.onClose).toHaveBeenCalled();
    expect(getSearchHistory(CHAT_SEARCH_HISTORY_KEY)).toEqual(['login']);
  });

  it('clicking a row opens that chat', async () => {
    render();
    await search('login');
    await vi.waitFor(() => expect($$('[data-role="search-hit"]')).toHaveLength(2));
    $$('[data-role="search-hit"]')[0].click();
    expect(props.onOpenChat).toHaveBeenCalledWith('s1');
  });

  it('keeps the recent searches of chats and of documents apart', () => {
    addSearchHistory('doc query');
    addSearchHistory('chat query', CHAT_SEARCH_HISTORY_KEY);
    render();
    expect($$('[data-role="search-history-item"]').map((el) => el.dataset.q)).toEqual(['chat query']);
    render({ mode: 'docs' });
    expect($$('[data-role="search-history-item"]').map((el) => el.dataset.q)).toEqual(['doc query']);
  });

  it('removing a recent chat search leaves the document history alone', () => {
    addSearchHistory('doc query');
    addSearchHistory('chat query', CHAT_SEARCH_HISTORY_KEY);
    render();
    $('[data-role="search-history-remove"]').click();
    expect(getSearchHistory(CHAT_SEARCH_HISTORY_KEY)).toEqual([]);
    expect(JSON.parse(localStorage.getItem(SEARCH_HISTORY_KEY))).toEqual(['doc query']);
  });

  it('says so when no chat matches, and when the search fails', async () => {
    apiMocks.searchChats.mockResolvedValueOnce({ hits: [] });
    render();
    await search('zzz');
    await vi.waitFor(() => expect($('.search-modal-status')?.textContent).toBe('No matching chats'));

    apiMocks.searchChats.mockRejectedValueOnce(new Error('boom'));
    typeInto(input(), 'again');
    await vi.waitFor(() => expect($('.search-modal-status')?.textContent).toBe('Search error'), { timeout: 2000 });
  });

  it('drops a slow answer that arrives after a newer one', async () => {
    let releaseFirst;
    apiMocks.searchChats
      .mockImplementationOnce(() => new Promise((resolve) => { releaseFirst = () => resolve({ hits: [CHAT_HITS[1]] }); }))
      .mockResolvedValueOnce({ hits: [CHAT_HITS[0]] });
    render();
    typeInto(input(), 'a');
    key(input(), 'Enter');
    await vi.waitFor(() => expect(apiMocks.searchChats).toHaveBeenCalledTimes(1));
    typeInto(input(), 'ab');
    key(input(), 'Enter');
    await vi.waitFor(() => expect($$('[data-role="search-hit"]')).toHaveLength(1));
    releaseFirst();
    await new Promise((resolve) => setTimeout(resolve, 20));
    expect($$('[data-role="search-hit"]').map((r) => r.dataset.sessionId)).toEqual(['s1']);
  });

  it('starts fresh when the mode changes while it is open', async () => {
    render();
    typeInto(input(), 'login');
    render({ mode: 'docs' });
    expect(input().value).toBe('');
    expect($('[role="dialog"]').getAttribute('aria-label')).toBe('Search documents');
  });

  it('documents mode still searches documents and opens a document', async () => {
    apiMocks.searchSpark.mockResolvedValue({ hits: [{ id: 'd1', title: 'Alpha doc', workspace_id: 'w', project_id: 'p' }] });
    render({ mode: 'docs' });
    typeInto(input(), 'alpha');
    await vi.waitFor(() => expect(apiMocks.searchSpark).toHaveBeenCalled(), { timeout: 2000 });
    await vi.waitFor(() => expect($$('[data-role="search-hit"]')).toHaveLength(1));
    expect($('[data-role="search-hit"]').dataset.noteId).toBe('d1');
    $('[data-role="search-hit"]').click();
    expect(props.onOpenDoc).toHaveBeenCalledWith('d1');
    expect(props.onOpenChat).not.toHaveBeenCalled();
    expect(apiMocks.searchChats).not.toHaveBeenCalled();
  });
});
