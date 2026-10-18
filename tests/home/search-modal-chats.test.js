// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import * as api from '../../frontend/src/host/api.ts';
import { mountHomeHub } from '../../frontend/src/home/hub.tsx';

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

describe('Home search follows the sidebar tab', () => {
  let container;
  let cleanup;
  let invokeSpy;
  let searchSpy;
  let chatSearchSpy;

  beforeEach(() => {
    installLocalStorageMock();
    openDocById.mockReset();
    container = document.createElement('div');
    document.body.appendChild(container);
    window.__TAURI__ = { event: { listen: vi.fn(async () => vi.fn()) } };
    vi.spyOn(api, 'createChannel').mockImplementation(async (onmessage) => ({ onmessage }));
    invokeSpy = vi.spyOn(api, 'invoke').mockImplementation(async (cmd) => {
      if (cmd === 'query_binding') return { state: 'bound' };
      if (cmd === 'list_chat_sessions') {
        return { sessions: [{ session_id: 's1', title: 'Recent chat', updated_at: 1 }], current_session_id: 's1' };
      }
      if (cmd === 'get_ai_assistant_binding') return { session_id: 'old', turns: [] };
      if (cmd === 'select_chat_session') return { session_id: 'old', turns: [] };
      return {};
    });
    vi.spyOn(api, 'fetchProjects').mockResolvedValue({ projects: [] });
    searchSpy = vi.spyOn(api, 'searchSpark').mockResolvedValue({ hits: [] });
    chatSearchSpy = vi.spyOn(api, 'searchChats').mockResolvedValue({
      hits: [{ session_id: 'old', title: 'An old design review', updated_at: 1_600_000_000 }],
    });
    cleanup = mountHomeHub(container, { navigate: vi.fn() });
  });

  afterEach(() => {
    cleanup?.();
    vi.restoreAllMocks();
    container.remove();
    delete window.__TAURI__;
  });

  const modal = () => container.querySelector('[data-role="search-modal"]');
  const input = () => container.querySelector('[data-role="search-modal-input"]');
  const hits = () => [...container.querySelectorAll('[data-role="search-hit"]')];
  const showSparkAiTab = () => container.querySelector('.home-sidebar-tab[data-pane="chat"]').click();

  async function openModal() {
    container.querySelector('[data-role="sidebar-search"]').click();
    await vi.waitFor(() => expect(modal()).not.toBeNull());
  }

  it('on the Home tab it searches documents', async () => {
    await openModal();
    expect(modal().querySelector('[role="dialog"]').getAttribute('aria-label')).toBe('Search documents');
    typeInto(input(), 'alpha');
    await vi.waitFor(() => expect(searchSpy).toHaveBeenCalled(), { timeout: 2000 });
    expect(chatSearchSpy).not.toHaveBeenCalled();
  });

  it('on the Spark AI tab it searches chats', async () => {
    showSparkAiTab();
    await openModal();
    expect(modal().querySelector('[role="dialog"]').getAttribute('aria-label')).toBe('Search chats');
    typeInto(input(), 'design');
    await vi.waitFor(() => expect(chatSearchSpy).toHaveBeenCalledWith('design', 8), { timeout: 2000 });
    expect(searchSpy).not.toHaveBeenCalled();
    await vi.waitFor(() => expect(hits()).toHaveLength(1));
    expect(hits()[0].querySelector('.search-modal-row-title').textContent).toBe('An old design review');
  });

  it('choosing a chat opens it and stays on the Spark AI tab', async () => {
    showSparkAiTab();
    await openModal();
    typeInto(input(), 'design');
    await vi.waitFor(() => expect(hits()).toHaveLength(1), { timeout: 2000 });
    hits()[0].click();
    await vi.waitFor(() =>
      expect(invokeSpy).toHaveBeenCalledWith('select_chat_session', { sessionId: 'old' }),
    );
    await vi.waitFor(() => expect(modal()).toBeNull());
    expect(container.querySelector('.home-chat-pane').hidden).toBe(false);
    expect(openDocById).not.toHaveBeenCalled();
  });

  it('switching tabs while it is open does not leak the other tab\'s query', async () => {
    await openModal();
    typeInto(input(), 'alpha');
    showSparkAiTab();
    await vi.waitFor(() => expect(modal().querySelector('[role="dialog"]').getAttribute('aria-label')).toBe('Search chats'));
    expect(input().value).toBe('');
  });
});
