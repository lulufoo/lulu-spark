// @vitest-environment jsdom
import { createElement } from 'react';
import { flushSync } from 'react-dom';
import { createRoot } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const apiMocks = vi.hoisted(() => ({ searchSpark: vi.fn() }));
vi.mock('../../frontend/src/host/api.ts', () => ({
  searchSpark: (...args) => apiMocks.searchSpark(...args),
}));

const { SearchModal } = await import('../../frontend/src/shared/search-modal/search-modal.tsx');
const { hitLocation } = await import('../../frontend/src/shared/search-modal/hit-location.ts');
const { addSearchHistory, getSearchHistory, SEARCH_HISTORY_KEY } = await import(
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

const NAMES = {
  workspace: (id) => ({ default: 'Default', mem: 'Memory' })[id] || '',
  project: (ws, id) => ({ 'default:p1': 'Inbox', 'mem:p2': 'Notes' })[`${ws}:${id}`] || '',
};
const ALL = { kind: 'all-workspaces' };

const HITS = [
  { id: 'd1', title: 'Alpha doc', workspace_id: 'default', project_id: 'p1', body: 'SNIPPET-TEXT' },
  { id: 'd2', title: 'Beta doc', workspace_id: 'mem', project_id: 'p2' },
];

function typeInto(input, value) {
  Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(input, value);
  input.dispatchEvent(new Event('input', { bubbles: true }));
}

function key(input, k, init = {}) {
  input.dispatchEvent(new KeyboardEvent('keydown', { key: k, bubbles: true, cancelable: true, ...init }));
}

describe('SearchModal', () => {
  let host;
  let root;
  let props;

  function render(next = {}) {
    props = { open: true, scope: ALL, names: NAMES, onOpenDoc: vi.fn(), onClose: vi.fn(), ...props, ...next };
    flushSync(() => root.render(createElement(SearchModal, props)));
  }
  const $ = (sel) => host.querySelector(sel);
  const $$ = (sel) => [...host.querySelectorAll(sel)];
  const input = () => $('[data-role="search-modal-input"]');

  async function search(text) {
    typeInto(input(), text);
    await vi.waitFor(() => expect(apiMocks.searchSpark).toHaveBeenCalled(), { timeout: 2000 });
  }

  beforeEach(() => {
    installLocalStorageMock();
    apiMocks.searchSpark.mockReset();
    apiMocks.searchSpark.mockResolvedValue({ hits: HITS });
    host = document.createElement('div');
    document.body.appendChild(host);
    root = createRoot(host);
    props = {};
  });

  afterEach(() => {
    flushSync(() => root.unmount());
    host.remove();
  });

  it('renders nothing while closed and a focused dialog when open', () => {
    render({ open: false });
    expect($('[data-role="search-modal"]')).toBeNull();
    render({ open: true });
    expect($('[role="dialog"]')).not.toBeNull();
    expect(document.activeElement).toBe(input());
    expect($('.search-modal-esc').textContent).toBe('esc');
  });

  describe('empty query: recent searches', () => {
    it('lists them newest first from the shared history key', () => {
      addSearchHistory('first');
      addSearchHistory('second');
      expect(localStorage.getItem(SEARCH_HISTORY_KEY)).toBe(JSON.stringify(['second', 'first']));
      render();
      expect($('.search-modal-section').textContent).toBe('Recent searches');
      expect($$('[data-role="search-history-item"]').map((el) => el.dataset.q)).toEqual(['second', 'first']);
      expect(apiMocks.searchSpark).not.toHaveBeenCalled();
    });

    it('keeps at most 10, and shows no recent documents', () => {
      for (let i = 1; i <= 12; i += 1) addSearchHistory(`q${i}`);
      render();
      const items = $$('[data-role="search-history-item"]');
      expect(items).toHaveLength(10);
      expect(items[0].dataset.q).toBe('q12');
      expect($$('[data-role="search-hit"]')).toHaveLength(0);
    });

    it('says what to do when there is no history', () => {
      render();
      expect($('.search-modal-status').textContent).toBe('Type to search documents.');
    });

    it('clicking one fills the input and runs it', async () => {
      addSearchHistory('alpha');
      render();
      $('[data-role="search-history-item"]').click();
      await vi.waitFor(() => expect($$('[data-role="search-hit"]')).toHaveLength(2));
      expect(input().value).toBe('alpha');
      expect(apiMocks.searchSpark).toHaveBeenCalledTimes(1);
      expect(apiMocks.searchSpark.mock.calls[0][0]).toBe('alpha');
    });

    it('the delete × removes that entry only, and does not run the search', async () => {
      addSearchHistory('keep');
      addSearchHistory('drop');
      render();
      $('[data-role="search-history-remove"][data-q="drop"]').click();
      await vi.waitFor(() =>
        expect($$('[data-role="search-history-item"]').map((el) => el.dataset.q)).toEqual(['keep']),
      );
      expect(getSearchHistory()).toEqual(['keep']);
      expect(apiMocks.searchSpark).not.toHaveBeenCalled();
    });

    it('ignores a history value that is not a list', () => {
      localStorage.setItem(SEARCH_HISTORY_KEY, JSON.stringify({ not: 'a list' }));
      expect(getSearchHistory()).toEqual([]);
      localStorage.setItem(SEARCH_HISTORY_KEY, '{bad json');
      expect(getSearchHistory()).toEqual([]);
    });
  });

  describe('with a query', () => {
    it('shows the title and a "workspace / project" line, and no body snippet', async () => {
      render();
      await search('doc');
      await vi.waitFor(() => expect($$('[data-role="search-hit"]')).toHaveLength(2));
      const rows = $$('[data-role="search-hit"]');
      expect(rows.map((r) => r.querySelector('.search-modal-row-title').textContent)).toEqual(['Alpha doc', 'Beta doc']);
      expect(rows.map((r) => r.querySelector('.search-modal-row-sub').textContent)).toEqual([
        'Default / Inbox',
        'Memory / Notes',
      ]);
      expect(host.textContent).not.toContain('SNIPPET-TEXT');
    });

    it('searches every workspace for the all-workspaces scope', async () => {
      render();
      await search('doc');
      expect(apiMocks.searchSpark).toHaveBeenCalledWith('doc', 8, '', { allWorkspaces: true });
    });

    it('searches the one workspace for a workspace scope, and shows just the project', async () => {
      apiMocks.searchSpark.mockResolvedValue({
        hits: [{ id: 'm1', title: 'In memory', workspace_id: 'mem', project_id: 'p2' }],
      });
      render({ scope: { kind: 'workspace', workspaceId: 'mem' } });
      await search('doc');
      expect(apiMocks.searchSpark).toHaveBeenCalledWith('doc', 8, 'mem', { allWorkspaces: false });
      await vi.waitFor(() => expect($$('[data-role="search-hit"]')).toHaveLength(1));
      expect($('.search-modal-row-sub').textContent).toBe('Notes');
    });

    it('strips a leading # before searching', async () => {
      render();
      await search('#topic');
      expect(apiMocks.searchSpark.mock.calls[0][0]).toBe('topic');
    });

    it('Enter while the typing pause is still running searches right away', async () => {
      render();
      typeInto(input(), 'quick');
      key(input(), 'Enter');
      await vi.waitFor(() => expect(apiMocks.searchSpark).toHaveBeenCalledTimes(1));
      expect(apiMocks.searchSpark.mock.calls[0][0]).toBe('quick');
      await new Promise((r) => setTimeout(r, 400));
      expect(apiMocks.searchSpark).toHaveBeenCalledTimes(1);
    });

    it('waits for a typing pause: one search for fast typing', async () => {
      render();
      typeInto(input(), 'a');
      typeInto(input(), 'al');
      typeInto(input(), 'alp');
      await vi.waitFor(() => expect(apiMocks.searchSpark).toHaveBeenCalled(), { timeout: 2000 });
      expect(apiMocks.searchSpark).toHaveBeenCalledTimes(1);
      expect(apiMocks.searchSpark.mock.calls[0][0]).toBe('alp');
    });

    it('leaves out a location part it has no name for, instead of showing an id', async () => {
      apiMocks.searchSpark.mockResolvedValue({
        hits: [{ id: 'x', title: 'Gone project', workspace_id: 'mem', project_id: 'unknown-id' }],
      });
      render();
      await search('x');
      await vi.waitFor(() => expect($$('[data-role="search-hit"]')).toHaveLength(1));
      expect($('.search-modal-row-sub').textContent).toBe('Memory');
      expect(host.textContent).not.toContain('unknown-id');
    });

    it('reports an empty result, an unbuilt index, and an error', async () => {
      render();
      apiMocks.searchSpark.mockResolvedValueOnce({ hits: [] });
      await search('none');
      await vi.waitFor(() => expect($('.search-modal-status')?.textContent).toBe('No related results'));

      apiMocks.searchSpark.mockResolvedValueOnce({ hits: [], error: 'not_indexed' });
      $('[data-role="search-modal-input"]').focus();
      key(input(), 'Enter');
      await vi.waitFor(() => expect($('.search-modal-status')?.textContent).toBe('Index not built yet.'));

      apiMocks.searchSpark.mockRejectedValueOnce(new Error('boom'));
      key(input(), 'Enter');
      await vi.waitFor(() => expect($('.search-modal-status')?.textContent).toBe('Search error'));
    });

    it('drops an answer that arrives after a newer search was made', async () => {
      let resolveOld;
      apiMocks.searchSpark
        .mockImplementationOnce(() => new Promise((r) => { resolveOld = r; }))
        .mockResolvedValueOnce({ hits: [HITS[1]] });
      render();
      typeInto(input(), 'old');
      key(input(), 'Enter'); // searches now
      await vi.waitFor(() => expect(apiMocks.searchSpark).toHaveBeenCalledTimes(1));
      typeInto(input(), 'new');
      key(input(), 'Enter');
      await vi.waitFor(() => expect($$('[data-role="search-hit"]')).toHaveLength(1));
      resolveOld({ hits: [HITS[0]] });
      await new Promise((r) => setTimeout(r, 20));
      expect($$('[data-role="search-hit"]').map((r) => r.dataset.noteId)).toEqual(['d2']);
    });
  });

  describe('selecting and keyboard', () => {
    async function withHits() {
      render();
      await search('doc');
      await vi.waitFor(() => expect($$('[data-role="search-hit"]')).toHaveLength(2));
    }

    it('clicking a row opens that document, saves the query, and closes', async () => {
      await withHits();
      $$('[data-role="search-hit"]')[1].click();
      expect(props.onOpenDoc).toHaveBeenCalledWith('d2');
      expect(getSearchHistory()).toEqual(['doc']);
      expect(props.onClose).toHaveBeenCalledTimes(1);
    });

    it('highlights the first row; up / down move it, within bounds', async () => {
      await withHits();
      const active = () => $$('[data-role="search-hit"]').map((r) => r.dataset.active === 'true');
      expect(active()).toEqual([true, false]);
      key(input(), 'ArrowDown');
      await vi.waitFor(() => expect(active()).toEqual([false, true]));
      key(input(), 'ArrowDown');
      await vi.waitFor(() => expect(active()).toEqual([false, true]));
      key(input(), 'ArrowUp');
      await vi.waitFor(() => expect(active()).toEqual([true, false]));
      key(input(), 'ArrowUp');
      await vi.waitFor(() => expect(active()).toEqual([true, false]));
    });

    it('Enter opens the highlighted row', async () => {
      await withHits();
      key(input(), 'ArrowDown');
      await vi.waitFor(() => expect($$('[data-role="search-hit"]')[1].dataset.active).toBe('true'));
      key(input(), 'Enter');
      expect(props.onOpenDoc).toHaveBeenCalledWith('d2');
      expect(props.onClose).toHaveBeenCalled();
    });

    it('Enter that confirms an IME composition does nothing', async () => {
      await withHits();
      key(input(), 'Enter', { isComposing: true });
      expect(props.onOpenDoc).not.toHaveBeenCalled();
      expect(props.onClose).not.toHaveBeenCalled();
    });

    it('Enter on a highlighted recent search runs it', async () => {
      addSearchHistory('older');
      addSearchHistory('newer');
      render();
      key(input(), 'ArrowDown');
      await vi.waitFor(() => expect($$('[data-role="search-history-item"]')[1].dataset.active).toBe('true'));
      key(input(), 'Enter');
      await vi.waitFor(() => expect($$('[data-role="search-hit"]')).toHaveLength(2));
      expect(input().value).toBe('older');
    });

    it('esc, the esc chip, and a click outside close it; a click inside does not', () => {
      render();
      key(input(), 'Escape');
      expect(props.onClose).toHaveBeenCalledTimes(1);
      $('[data-role="search-modal-close"]').click();
      expect(props.onClose).toHaveBeenCalledTimes(2);
      $('.search-modal-backdrop').dispatchEvent(new MouseEvent('mousedown', { bubbles: true }));
      expect(props.onClose).toHaveBeenCalledTimes(3);
      $('.search-modal').dispatchEvent(new MouseEvent('mousedown', { bubbles: true }));
      expect(props.onClose).toHaveBeenCalledTimes(3);
    });

    it('opens through cta:open-entry when no opener is given', async () => {
      const seen = vi.fn();
      document.addEventListener('cta:open-entry', seen);
      try {
        props = { onOpenDoc: undefined };
        render({ onOpenDoc: undefined });
        await search('doc');
        await vi.waitFor(() => expect($$('[data-role="search-hit"]')).toHaveLength(2));
        key(input(), 'Enter');
        expect(seen).toHaveBeenCalledTimes(1);
        expect(seen.mock.calls[0][0].detail).toEqual({ id: 'd1' });
      } finally {
        document.removeEventListener('cta:open-entry', seen);
      }
    });

    it('starts fresh each time it is opened', async () => {
      await withHits();
      render({ open: false });
      render({ open: true });
      expect(input().value).toBe('');
      expect($$('[data-role="search-hit"]')).toHaveLength(0);
    });
  });
});

describe('hitLocation', () => {
  const hit = { workspace_id: 'mem', project_id: 'p2' };
  it('is "workspace / project" across workspaces and just "project" inside one', () => {
    expect(hitLocation(hit, ALL, NAMES)).toBe('Memory / Notes');
    expect(hitLocation(hit, { kind: 'workspace', workspaceId: 'mem' }, NAMES)).toBe('Notes');
  });

  it('falls back to the scope workspace when a hit names none', () => {
    expect(hitLocation({ project_id: 'p2' }, { kind: 'workspace', workspaceId: 'mem' }, NAMES)).toBe('Notes');
  });

  it('leaves out whatever it cannot name', () => {
    expect(hitLocation({ workspace_id: 'mem', project_id: 'zz' }, ALL, NAMES)).toBe('Memory');
    expect(hitLocation({ workspace_id: 'zz', project_id: 'p2' }, ALL, NAMES)).toBe('');
    expect(hitLocation({}, ALL, NAMES)).toBe('');
  });
});
