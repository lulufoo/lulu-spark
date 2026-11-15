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
import { useLoadedWorkspaces } from '../helpers/workspaces.js';
import { state } from '../../frontend/src/host/state.ts';
import { getHomeState, resetHomeState, setHomeState } from '../../frontend/src/home/state/store.ts';
import { markHomeEntryRead, markHomeWorkspaceRead, refreshChannelUnread, resetChannelUnread } from '../../frontend/src/home/commands/channel-unread.ts';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

function isEntryUnread(root, entry) {
  return (
    root.querySelector(`[data-home-entry="${entry}"]`)?.getAttribute('data-home-unread') === 'true'
  );
}

describe('home workspace and Read Later unread', () => {
  let container;
  let cleanup;
  let navigate;
  let listenHandlers;
  let unreadByWorkspace;
  let readLaterUnread;
  let workspaceUnreadSpy;
  let workspaceMarkSpy;
  let unreadSpy;
  let markSpy;
  let invokeSpy;
  let restoreWorkspaces;

  beforeEach(() => {
    restoreWorkspaces = useLoadedWorkspaces();
    state.workspaces.list.push({ workspace_id: 'mac', workspace_name: 'Mac', view_mode: 'directory', is_default: false, icon: '' });
    container = document.createElement('div');
    document.body.appendChild(container);
    cleanup = null;
    navigate = vi.fn();
    listenHandlers = {};
    unreadByWorkspace = { mac: true };
    readLaterUnread = true;
    resetChannelUnread();
    resetHomeState();
    window.__TAURI__ = {
      event: {
        listen: vi.fn(async (name, handler) => {
          listenHandlers[name] = handler;
          return vi.fn();
        }),
      },
    };
    workspaceUnreadSpy = vi.spyOn(api, 'getMessageWorkspaceUnread').mockImplementation(async () => ({ ...unreadByWorkspace }));
    workspaceMarkSpy = vi.spyOn(api, 'markMessageWorkspaceRead').mockImplementation(async (id) => { delete unreadByWorkspace[id]; });
    unreadSpy = vi.spyOn(api, 'getMessageChannelUnread').mockImplementation(async () => readLaterUnread);
    markSpy = vi.spyOn(api, 'markMessageChannelRead').mockImplementation(async () => { readLaterUnread = false; });
    invokeSpy = vi.spyOn(api, 'invoke').mockImplementation(async (cmd) => {
      if (cmd === 'query_binding') return { state: 'unbound' };
      if (cmd === 'list_chat_sessions') return { sessions: [], current_session_id: '' };
      return {};
    });
  });

  afterEach(() => {
    cleanup?.();
    resetChannelUnread();
    resetHomeState();
    restoreWorkspaces();
    vi.restoreAllMocks();
    container.remove();
    delete window.__TAURI__;
  });

  async function mountReady(options = {}) {
    cleanup = mountHomeHub(container, { navigate, ...options });
    await vi.waitFor(() => {
      expect(workspaceUnreadSpy).toHaveBeenCalledOnce();
      expect(unreadSpy).toHaveBeenCalledExactlyOnceWith('read_later');
      expect(isEntryUnread(container, 'workspace:mac')).toBe(true);
    });
  }

  it('uses one workspace map to light Mac without lighting the default workspace', async () => {
    await mountReady();
    expect(isEntryUnread(container, 'spark')).toBe(false);
    expect(isEntryUnread(container, 'knowledge')).toBe(false);
    expect(isEntryUnread(container, 'read-later')).toBe(true);
    expect(container.querySelector('[data-home-entry="todo-tasks"]')).toBeNull();
  });

  it('refreshes each workspace after message-center:changed', async () => {
    await mountReady();
    await vi.waitFor(() => expect(listenHandlers['message-center:changed']).toEqual(expect.any(Function)));
    unreadByWorkspace = { 'ws-default': true, knowledge: true };
    readLaterUnread = false;
    workspaceUnreadSpy.mockClear();
    unreadSpy.mockClear();
    listenHandlers['message-center:changed']({});
    await vi.waitFor(() => {
      expect(workspaceUnreadSpy).toHaveBeenCalledOnce();
      expect(isEntryUnread(container, 'workspace:mac')).toBe(false);
      expect(isEntryUnread(container, 'spark')).toBe(true);
      expect(isEntryUnread(container, 'knowledge')).toBe(true);
      expect(isEntryUnread(container, 'read-later')).toBe(false);
    });
    expect(unreadSpy).toHaveBeenCalledExactlyOnceWith('read_later');
  });

  it('entering a workspace clears only its own documents; expanding leaves unread intact', async () => {
    unreadByWorkspace.knowledge = true;
    await mountReady();
    container.querySelector('[data-home-entry="workspace:mac"]').click();
    expect(navigate).not.toHaveBeenCalled();
    expect(workspaceMarkSpy).not.toHaveBeenCalled();
    container.querySelector('[data-home-open-workspace="mac"]').click();
    expect(navigate).toHaveBeenCalledWith('#/spark');
    await vi.waitFor(() => {
      expect(workspaceMarkSpy).toHaveBeenCalledExactlyOnceWith('mac');
      expect(isEntryUnread(container, 'workspace:mac')).toBe(false);
    });
    expect(isEntryUnread(container, 'knowledge')).toBe(true);
    expect(isEntryUnread(container, 'read-later')).toBe(true);
    expect(markSpy).not.toHaveBeenCalled();
    expect(invokeSpy.mock.calls.map((call) => call[0])).not.toContain('create_note');
  });

  it('Read Later clears independently without clearing workspace unread', async () => {
    const openReadLater = vi.fn();
    await mountReady({ openReadLater });
    container.querySelector('[data-home-entry="read-later"]').click();
    expect(openReadLater).toHaveBeenCalledTimes(1);
    await vi.waitFor(() => {
      expect(markSpy).toHaveBeenCalledExactlyOnceWith('read_later');
      expect(isEntryUnread(container, 'read-later')).toBe(false);
    });
    expect(isEntryUnread(container, 'workspace:mac')).toBe(true);
    expect(workspaceMarkSpy).not.toHaveBeenCalled();
  });

  it('reloads persisted workspace unread after the Home view remounts', async () => {
    unreadByWorkspace.knowledge = true;
    await mountReady();
    await markHomeWorkspaceRead('mac');
    cleanup();
    cleanup = null;
    workspaceUnreadSpy.mockClear();
    unreadSpy.mockClear();
    cleanup = mountHomeHub(container, { navigate });
    await vi.waitFor(() => {
      expect(workspaceUnreadSpy).toHaveBeenCalledOnce();
      expect(isEntryUnread(container, 'knowledge')).toBe(true);
      expect(isEntryUnread(container, 'workspace:mac')).toBe(false);
    });
  });

  it('does not restore a cleared dot from a refresh that started before the clear', async () => {
    unreadByWorkspace = { mac: true, knowledge: true };
    await refreshChannelUnread();
    let release;
    workspaceUnreadSpy.mockImplementationOnce(() => new Promise((resolve) => { release = resolve; }));
    const stale = refreshChannelUnread();
    await markHomeWorkspaceRead('mac');
    release({ mac: true, knowledge: true });
    await stale;
    expect(getHomeState().workspaceUnread).toEqual({ knowledge: true });
    expect(getHomeState().channelUnread.read_later).toBe(true);
  });

  it('reconciles a new unread document created after the host clear but before its response', async () => {
    await refreshChannelUnread();
    let releaseClear;
    workspaceMarkSpy.mockImplementationOnce(() => {
      delete unreadByWorkspace.mac;
      return new Promise((resolve) => { releaseClear = resolve; });
    });
    const marking = markHomeWorkspaceRead('mac');
    unreadByWorkspace.mac = true;
    await refreshChannelUnread();
    releaseClear();
    await marking;
    expect(getHomeState().workspaceUnread.mac).toBe(true);
  });

  it('keeps both dots when clearing one workspace fails', async () => {
    unreadByWorkspace = { mac: true, knowledge: true };
    await refreshChannelUnread();
    workspaceMarkSpy.mockRejectedValueOnce(new Error('persist failed'));
    await markHomeWorkspaceRead('mac');
    expect(getHomeState().workspaceUnread).toEqual({ mac: true, knowledge: true });
  });

  it('preserves unread when a refresh fails and rejects a superseded response', async () => {
    await refreshChannelUnread();
    workspaceUnreadSpy.mockRejectedValueOnce(new Error('offline'));
    await refreshChannelUnread();
    expect(getHomeState().workspaceUnread).toEqual({ mac: true });
    let release;
    workspaceUnreadSpy.mockImplementationOnce(() => new Promise((resolve) => { release = resolve; }));
    const stale = refreshChannelUnread();
    unreadByWorkspace = { knowledge: true };
    await refreshChannelUnread();
    release({ mac: true });
    await stale;
    expect(getHomeState().workspaceUnread).toEqual({ knowledge: true });
  });

  it('does not relight Read Later from a refresh started before clearing it', async () => {
    await refreshChannelUnread();
    let release;
    unreadSpy.mockImplementationOnce(() => new Promise((resolve) => { release = resolve; }));
    const stale = refreshChannelUnread();
    await markHomeEntryRead('read-later');
    release(true);
    await stale;
    expect(getHomeState().channelUnread.read_later).toBe(false);
    expect(getHomeState().workspaceUnread).toEqual({ mac: true });
  });

  it('ignores an old view clear response after resetting Home commands', async () => {
    await refreshChannelUnread();
    let release;
    workspaceMarkSpy.mockImplementationOnce(() => new Promise((resolve) => { release = resolve; }));
    const marking = markHomeWorkspaceRead('mac');
    resetChannelUnread();
    setHomeState((prev) => ({ ...prev, workspaceUnread: { mac: true, knowledge: true } }));
    release();
    await marking;
    expect(getHomeState().workspaceUnread).toEqual({ mac: true, knowledge: true });
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
