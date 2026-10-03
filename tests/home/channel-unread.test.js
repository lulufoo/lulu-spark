// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import * as api from '../../frontend/src/host/api.ts';
import { mountHomeHub } from '../../frontend/src/home/hub.tsx';
import { mountHomeEntryShell } from '../../frontend/src/home-entry-shell/shell.tsx';
import { getBaselineEntries } from '../../frontend/src/home-entry-shell/entry-config.ts';
import { createContentRegistry } from '../../frontend/src/home-entry-shell/content-registry.ts';
import { readFrontendJs } from '../helpers/read-frontend-js.js';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

function isEntryUnread(root, entry) {
  return (
    root.querySelector(`[data-home-entry="${entry}"]`)?.getAttribute('data-home-unread') === 'true'
  );
}

describe('home three-entry boolean unread', () => {
  let container;
  let cleanup;
  let navigate;
  let listenHandlers;
  /** @type {import('vitest').MockInstance} */
  let unreadSpy;
  /** @type {import('vitest').MockInstance} */
  let markSpy;
  /** @type {import('vitest').MockInstance} */
  let invokeSpy;
  /** @type {Record<string, boolean>} */
  let unreadByChannel;

  beforeEach(() => {
    container = document.createElement('div');
    document.body.appendChild(container);
    cleanup = null;
    navigate = vi.fn();
    listenHandlers = {};
    unreadByChannel = {
      notes: true,
      read_later: true,
    };
    window.__TAURI__ = {
      event: {
        listen: vi.fn(async (name, handler) => {
          listenHandlers[name] = handler;
          return vi.fn();
        }),
      },
    };
    unreadSpy = vi.spyOn(api, 'getMessageChannelUnread').mockImplementation(async (channel) => {
      return Boolean(unreadByChannel[String(channel)]);
    });
    markSpy = vi.spyOn(api, 'markMessageChannelRead').mockImplementation(async (channel) => {
      unreadByChannel[String(channel)] = false;
    });
    invokeSpy = vi.spyOn(api, 'invoke').mockImplementation(async (cmd) => {
      if (cmd === 'query_binding') return { state: 'unbound' };
      if (cmd === 'list_chat_sessions') return { sessions: [], current_session_id: '' };
      return {};
    });
  });

  afterEach(() => {
    cleanup?.();
    unreadSpy.mockRestore();
    markSpy.mockRestore();
    invokeSpy.mockRestore();
    container.remove();
    delete window.__TAURI__;
  });

  async function mountReady() {
    cleanup = mountHomeHub(container, { navigate });
    await vi.waitFor(() => {
      expect(unreadSpy.mock.calls.map((call) => call[0]).sort()).toEqual([
        'notes',
        'read_later',
      ]);
    });
  }

  it('refreshes Notes / Read Later boolean dots from host unread on start', async () => {
    await mountReady();
    expect(isEntryUnread(container, 'spark')).toBe(true);
    expect(isEntryUnread(container, 'read-later')).toBe(true);
    expect(container.querySelector('[data-home-entry="todo-tasks"]')).toBeNull();
    expect(unreadSpy).toHaveBeenCalledWith('notes');
    expect(unreadSpy).toHaveBeenCalledWith('read_later');
    expect(unreadSpy.mock.calls.map((call) => call[0])).not.toContain('todos');
    expect(unreadSpy.mock.calls.map((call) => call[0])).not.toContain('knowledge');
  });

  it('refreshes again after message-center:changed', async () => {
    await mountReady();
    await vi.waitFor(() => {
      expect(listenHandlers['message-center:changed']).toEqual(expect.any(Function));
    });
    unreadByChannel.read_later = false;
    unreadSpy.mockClear();
    listenHandlers['message-center:changed']({});
    await vi.waitFor(() => {
      expect(unreadSpy.mock.calls.map((call) => call[0]).sort()).toEqual([
        'notes',
        'read_later',
      ]);
      expect(isEntryUnread(container, 'read-later')).toBe(false);
    });
  });

  it('clicking a dotted entry navigates and marks that channel read', async () => {
    await mountReady();
    container.querySelector('[data-home-entry="spark"]').click();
    expect(navigate).toHaveBeenCalledWith('#/spark');
    await vi.waitFor(() => {
      expect(markSpy).toHaveBeenCalledWith('notes');
      expect(isEntryUnread(container, 'spark')).toBe(false);
    });
    expect(isEntryUnread(container, 'read-later')).toBe(true);
    expect(invokeSpy.mock.calls.map((call) => call[0])).not.toContain('create_note');
    expect(container.querySelector('[data-role="message-list"]')).toBeNull();
  });

  it('clicking Read Later marks the matching channel', async () => {
    const openReadLater = vi.fn();
    cleanup = mountHomeHub(container, { navigate, openReadLater });
    await vi.waitFor(() => {
      expect(unreadSpy.mock.calls.map((call) => call[0]).sort()).toEqual([
        'notes',
        'read_later',
      ]);
    });

    container.querySelector('[data-home-entry="read-later"]').click();
    expect(openReadLater).toHaveBeenCalledTimes(1);
    await vi.waitFor(() => {
      expect(markSpy).toHaveBeenCalledWith('read_later');
      expect(isEntryUnread(container, 'read-later')).toBe(false);
    });
    expect(container.querySelector('[data-home-entry="todo-tasks"]')).toBeNull();
  });

  it('hangs unread only on workbench / read-later', async () => {
    await mountReady();
    expect(isEntryUnread(container, 'spark')).toBe(true);
    expect(isEntryUnread(container, 'read-later')).toBe(true);
    expect(isEntryUnread(container, 'knowledge')).toBe(false);
    const knowledge = container.querySelector('[data-home-entry="knowledge"]');
    expect(knowledge).not.toBeNull();
    expect(knowledge.getAttribute('data-home-unread')).not.toBe('true');
    knowledge.click();
    expect(navigate).toHaveBeenCalledWith('#/knowledge');
    expect(markSpy).not.toHaveBeenCalled();
  });
});

describe('home unread wiring constraints', () => {
  it('listens with existing hub.ts __TAURI__.event.listen and does not register invoke', () => {
    const hubSrc = readFileSync(join(repoRoot, 'frontend/src/home/commands/hub.ts'), 'utf8');
    expect(hubSrc).toMatch(/__TAURI__/);
    expect(hubSrc).toMatch(/event\?\.listen/);
    expect(hubSrc).toMatch(/message-center:changed/);
    expect(hubSrc).not.toMatch(/register[_A-Z]*invoke|register_consumer|register_listener/i);
  });

  it('page and hub go through host/api, not invoke maps', () => {
    const pageSrc = readFileSync(join(repoRoot, 'frontend/src/home/page.tsx'), 'utf8');
    const hubSrc = readFileSync(join(repoRoot, 'frontend/src/home/commands/hub.ts'), 'utf8');
    expect(hubSrc).toMatch(/from ['"]\.\.\/\.\.\/host\/api\.ts['"]/);
    expect(pageSrc).not.toMatch(/readApiInvokeMap|writeApiInvokeMap/);
    expect(hubSrc).not.toMatch(/readApiInvokeMap|writeApiInvokeMap/);
  });

  it('does not hang unread on home-entry FAB or builders', () => {
    const registry = createContentRegistry();
    for (const entry of getBaselineEntries()) {
      registry.register(entry.contentKey, { mount() {} });
    }
    const anchor = document.createElement('div');
    document.body.appendChild(anchor);
    const shell = mountHomeEntryShell(anchor, {
      config: getBaselineEntries(),
      registry,
      host: {},
    });
    try {
      expect(anchor.querySelector('[data-home-unread="true"]')).toBeNull();
      expect(anchor.querySelector('[data-role="hub"]')?.getAttribute('data-home-unread')).not.toBe(
        'true',
      );
      for (const entry of getBaselineEntries()) {
        const btn = anchor.querySelector(`[data-entry-id="${entry.id}"]`);
        expect(btn?.getAttribute('data-home-unread')).not.toBe('true');
      }
    } finally {
      shell.unmount();
      anchor.remove();
    }

    const shellSrc = readFrontendJs('frontend/src/home-entry-shell/shell.tsx');
    expect(shellSrc).not.toMatch(/getMessageChannelUnread|markMessageChannelRead|data-home-unread/);
  });
});
