// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import * as api from '../../frontend/src/host/api.ts';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

const getAuthUserMock = vi.hoisted(() => vi.fn());
const startAuthLoginMock = vi.hoisted(() => vi.fn());
const signOutAuthMock = vi.hoisted(() => vi.fn());
const readVaultSessionMock = vi.hoisted(() => vi.fn());
const persistVaultSessionMock = vi.hoisted(() => vi.fn());

vi.mock('../../frontend/src/auth/oauth.ts', () => ({
  getAuthUser: (...args) => getAuthUserMock(...args),
  startAuthLogin: (...args) => startAuthLoginMock(...args),
  signOutAuth: (...args) => signOutAuthMock(...args),
  readVaultSession: (...args) => readVaultSessionMock(...args),
  persistVaultSession: (...args) => persistVaultSessionMock(...args),
}));

function readRel(rel) {
  return readFileSync(join(repoRoot, rel), 'utf8');
}

function doneResult(overrides = {}) {
  return {
    ok: true,
    uploaded: 0,
    pulled: 0,
    skipped: 0,
    kept: 0,
    gone: 0,
    not_tried: 0,
    failed: [],
    stopped: null,
    cancelled: false,
    ...overrides,
  };
}

function signedUser(overrides = {}) {
  return {
    user_id: 'user-42',
    email: 'ada@example.com',
    display_name: 'Ada',
    avatar_url: 'https://example.com/a.png',
    provider: 'google',
    ...overrides,
  };
}

describe('sync button · source', () => {
  it('sits in the account row between the account bar and the ?', () => {
    const page = readRel('frontend/src/home/page.tsx');
    expect(page).toMatch(/from ['"]\.\/ui\/cloud-sync-button\.tsx['"]/);
    const rowStart = page.indexOf('data-role="account-row"');
    const account = page.indexOf('<AccountBar', rowStart);
    const sync = page.indexOf('<CloudSyncButton', rowStart);
    const help = page.indexOf('<HelpMenu', rowStart);
    expect(rowStart).toBeGreaterThan(-1);
    expect(sync).toBeGreaterThan(account);
    expect(help).toBeGreaterThan(sync);
    expect(page).toMatch(/<CloudSyncButton sync=\{cloudSync\} onSync=\{onSyncStart\} onCancel=\{onSyncCancel\}/);
  });

  it('is pure paint: the page composes, the button only shows and calls back', () => {
    const src = readRel('frontend/src/home/ui/cloud-sync-button.tsx');
    expect(src).toMatch(/export function CloudSyncButton/);
    expect(src).toMatch(/<HomeNavIcon name="sync"/);
    expect(src).not.toMatch(
      /syncDocuments|cancelSyncDocuments|refreshAuthSession|runCloudSync|cancelCloudSync|cloudSyncStore|\/host\//,
    );

    const page = readRel('frontend/src/home/page.tsx');
    expect(page).toMatch(/runCloudSync/);
    expect(page).toMatch(/cancelCloudSync/);
    expect(page).toMatch(/resetCloudSync/);
  });

  it('has an icon of its own, so the button does not borrow another', () => {
    expect(readRel('frontend/src/home/ui/nav-icons.tsx')).toMatch(/^\s*sync: \(/m);
  });

  it('keeps turning while a sync goes on, and warns with a dot', () => {
    const css = readRel('frontend/app.css');
    expect(css).toMatch(/\.home-sync-button-trigger\b/);
    expect(css).toMatch(
      /\.home-sync-button-trigger\[data-turning="true"\] \.home-desktop-shortcut-icon\s*\{[^}]*animation:\s*home-sync-turn 1\.1s cubic-bezier\([^)]*\) infinite/,
    );
    expect(css).toMatch(/\.home-sync-button-trigger\[data-turning="true"\]\s*\{[^}]*color:\s*var\(--fg-accent\)/);
    expect(css).toMatch(/\.home-sync-button-trigger\s*\{[^}]*transition:\s*color/);
    expect(css).toMatch(
      /@keyframes home-sync-turn\s*\{[^}]*rotate\(180deg\)/,
    );
    expect(css).toMatch(
      /@media \(prefers-reduced-motion: reduce\)\s*\{[^@]*\[data-turning="true"\] \.home-desktop-shortcut-icon\s*\{[^}]*animation:\s*home-sync-breathe/,
    );
    expect(css).toMatch(
      /\.home-sync-button-trigger\[data-tone="warn"\]::after\s*\{[^}]*background:\s*var\(--fg-danger\)/,
    );
    expect(css).toMatch(
      /\.home-sync-button-status\[data-tone="warn"\] \.home-sync-button-title[^{]*\{[^}]*color:\s*var\(--fg-danger\)/,
    );
  });

  it('opens its panel over the whole account row, which holds it', () => {
    const css = readRel('frontend/app.css');
    expect(css).toMatch(/\.home-account-row\s*\{[^}]*position:\s*relative/);
    expect(css).toMatch(/\.home-sync-button-panel\s*\{[^}]*position:\s*absolute/);
  });
});

describe('CloudSyncButton', () => {
  let container;
  let root;
  let CloudSyncButton;
  let onSync;
  let onCancel;

  beforeEach(async () => {
    ({ CloudSyncButton } = await import('../../frontend/src/home/ui/cloud-sync-button.tsx'));
    container = document.createElement('div');
    document.body.appendChild(container);
    root = createRoot(container);
    onSync = vi.fn();
    onCancel = vi.fn();
  });

  afterEach(() => {
    act(() => {
      root?.unmount();
    });
    container?.remove();
  });

  function render(props = {}) {
    act(() => {
      root.render(createElement(CloudSyncButton, { sync: { phase: 'idle' }, onSync, onCancel, ...props }));
    });
  }

  const q = (role) => container.querySelector(`[data-role="${role}"]`);
  const trigger = () => q('sync-trigger');
  const title = () => q('sync-status-title')?.textContent;
  const detail = () => q('sync-status-detail')?.textContent;
  const cancel = () => q('sync-cancel');
  const press = () => {
    act(() => {
      trigger().click();
    });
  };

  const syncing = { phase: 'syncing', cancelling: false };
  const cancelling = { phase: 'syncing', cancelling: true };

  it('is an icon button that does nothing until it is pressed', () => {
    render();

    expect(trigger().tagName).toBe('BUTTON');
    expect(trigger().getAttribute('aria-label')).toBe('Sync');
    expect(trigger().getAttribute('data-state')).toBe('idle');
    expect(trigger().getAttribute('aria-busy')).toBe('false');
    expect(trigger().querySelector('[data-home-icon="sync"]')).not.toBeNull();
    expect(onSync).not.toHaveBeenCalled();
    expect(q('sync-status')).toBeNull();
  });

  it('asks for a sync when pressed, once per press, and shows the status as soon as there is one', () => {
    render();

    press();
    expect(onSync).toHaveBeenCalledTimes(1);
    render({ sync: syncing });

    expect(title()).toBe('Syncing…');
    expect(q('sync-status').getAttribute('role')).toBe('status');
  });

  it('keeps turning and stays busy for as long as the sync goes on', () => {
    render({ sync: syncing });

    expect(trigger().getAttribute('data-state')).toBe('syncing');
    expect(trigger().getAttribute('aria-busy')).toBe('true');
    expect(trigger().getAttribute('aria-label')).toBe('Syncing…');
    render({ sync: cancelling });
    expect(trigger().getAttribute('data-state')).toBe('syncing');
  });

  describe('the turn', () => {
    const turning = () => trigger().getAttribute('data-turning');
    // jsdom has no CSS animations, so React listens there for the prefixed name; a browser sends one.
    const endOfATurn = () => {
      act(() => {
        const icon = trigger().querySelector('[data-home-icon="sync"]');
        for (const name of ['animationiteration', 'webkitAnimationIteration']) {
          icon.dispatchEvent(new Event(name, { bubbles: true }));
        }
      });
    };
    const done = { phase: 'done', result: doneResult({ uploaded: 1 }) };

    it('does not turn before a sync, and starts with it', () => {
      render();
      expect(turning()).toBe('false');

      render({ sync: syncing });

      expect(turning()).toBe('true');
    });

    it('keeps turning through the end of every turn for as long as the sync goes on', () => {
      render({ sync: syncing });

      endOfATurn();
      endOfATurn();

      expect(turning()).toBe('true');
    });

    it.each([
      ['finished', done],
      ['cancelled', { phase: 'cancelled' }],
      ['refused', { phase: 'refused', reason: 'signed_out' }],
      ['failed', { phase: 'failed', status: null, message: 'x' }],
    ])('finishes the turn it is in when the sync %s, then rests', (_name, ended) => {
      render({ sync: syncing });

      render({ sync: ended });

      expect(trigger().getAttribute('data-state')).toBe('idle');
      expect(turning()).toBe('true');
      endOfATurn();
      expect(turning()).toBe('false');
    });

    it('keeps turning, without a break, when a new sync starts before the turn is finished', () => {
      render({ sync: syncing });
      render({ sync: done });

      render({ sync: syncing });
      endOfATurn();

      expect(turning()).toBe('true');
      render({ sync: done });
      endOfATurn();
      expect(turning()).toBe('false');
    });

    it('can turn again after it rested', () => {
      render({ sync: syncing });
      render({ sync: done });
      endOfATurn();
      expect(turning()).toBe('false');

      render({ sync: syncing });

      expect(turning()).toBe('true');
    });
  });

  it('shows the status, not a second sync, when pressed during a sync', () => {
    render({ sync: syncing });

    press();

    expect(onSync).not.toHaveBeenCalled();
    expect(title()).toBe('Syncing…');
    expect(detail()).toMatch(/^Cancel stops before the next document/);
    expect(cancel().textContent).toBe('Cancel');
    expect(cancel().disabled).toBe(false);
  });

  it('cancels from the panel, once per press', () => {
    render({ sync: syncing });
    press();

    act(() => {
      cancel().click();
    });

    expect(onCancel).toHaveBeenCalledTimes(1);
    expect(onSync).not.toHaveBeenCalled();
  });

  it('says cancelling and offers no second cancel once the user has asked', () => {
    render({ sync: syncing });
    press();

    render({ sync: cancelling });

    expect(title()).toBe('Cancelling…');
    expect(cancel().textContent).toBe('Cancelling…');
    expect(cancel().disabled).toBe(true);
    act(() => {
      cancel().click();
    });
    expect(onCancel).not.toHaveBeenCalled();
  });

  const ended = [
    [
      'a sync that was made',
      { phase: 'done', result: doneResult({ uploaded: 1, skipped: 2 }) },
      'neutral',
      'Sync finished',
      'Uploaded 1 · Pulled 0 · Unchanged 2',
    ],
    [
      'a sync with a failed document',
      { phase: 'done', result: doneResult({ failed: [{ id: 'd', op: 'upload', error: 'too big' }] }) },
      'warn',
      'Synced, with problems',
      'Uploaded 0 · Pulled 0 · Unchanged 0 · 1 failed · first: too big',
    ],
    [
      'a cancelled sync',
      { phase: 'done', result: doneResult({ uploaded: 1, not_tried: 2, cancelled: true }) },
      'neutral',
      'Sync cancelled',
      'Uploaded 1 · Pulled 0 · Unchanged 0 · 2 not tried, sync again to continue',
    ],
    ['a cancel before the host was asked', { phase: 'cancelled' }, 'neutral', 'Sync cancelled', 'Nothing was sent or changed.'],
    ['signed out', { phase: 'refused', reason: 'signed_out' }, 'warn', 'Cannot sync', 'Sign in first to sync.'],
    [
      'a host that could not sync',
      { phase: 'failed', status: 502, message: 'Cannot sync: cloud sync not reachable' },
      'warn',
      'Cannot sync',
      'Cannot sync: cloud sync not reachable',
    ],
  ];

  it.each(ended)('after %s: tells what happened, offers no cancel, and is ready to sync again', (_name, sync, tone, heading, text) => {
    render({ sync: syncing });
    press();

    render({ sync });

    expect(title()).toBe(heading);
    expect(detail()).toBe(text);
    expect(q('sync-status').getAttribute('data-tone')).toBe(tone);
    expect(cancel()).toBeNull();
    expect(trigger().getAttribute('data-state')).toBe('idle');
    expect(trigger().getAttribute('aria-busy')).toBe('false');
    expect(trigger().getAttribute('data-tone')).toBe(tone);
  });

  it('closes the panel on a press outside it, and a press on the icon then starts the next sync', () => {
    render({ sync: syncing });
    press();
    render({ sync: { phase: 'done', result: doneResult({ uploaded: 1 }) } });
    expect(title()).toBe('Sync finished');

    act(() => {
      q('close-sync-panel').click();
    });
    expect(q('sync-status')).toBeNull();
    expect(trigger().getAttribute('aria-expanded')).toBe('false');

    press();
    expect(onSync).toHaveBeenCalledTimes(1);
  });

  it('gives the panel back with a press on the icon while the sync still goes on', () => {
    render({ sync: syncing });
    press();
    act(() => {
      q('close-sync-panel').click();
    });
    expect(q('sync-status')).toBeNull();
    expect(trigger().getAttribute('data-state')).toBe('syncing');

    press();

    expect(title()).toBe('Syncing…');
    expect(onSync).not.toHaveBeenCalled();
  });

  it('shows no warning dot while a sync goes on, and none before the first', () => {
    render();
    expect(trigger().hasAttribute('data-tone')).toBe(false);
    render({ sync: { phase: 'failed', status: null, message: 'x' } });
    expect(trigger().getAttribute('data-tone')).toBe('warn');

    render({ sync: syncing });

    expect(trigger().hasAttribute('data-tone')).toBe(false);
  });

  it('stays usable when a callback throws', () => {
    onSync.mockImplementation(() => {
      throw new Error('sync failed');
    });
    onCancel.mockImplementation(() => {
      throw new Error('cancel failed');
    });
    render();
    press();
    render({ sync: syncing });

    act(() => {
      cancel().click();
    });

    expect(container.querySelectorAll('[data-role="sync-button"]').length).toBe(1);
    expect(title()).toBe('Syncing…');
  });
});

describe('sync button · from the home rail', () => {
  let container;
  let cleanup;
  let realFetch;
  let syncDocuments;
  let cancelSyncDocuments;

  const nowSeconds = () => Math.floor(Date.now() / 1000);
  const vaultSession = (expiresIn) => ({
    access_token: 'vault-access',
    refresh_token: 'vault-refresh',
    expires_at: nowSeconds() + expiresIn,
    user: { id: 'user-42', email: 'ada@example.com' },
  });
  const json = (status, body) =>
    new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } });
  const refreshed = () =>
    json(200, {
      access_token: 'new-access',
      refresh_token: 'new-refresh',
      token_type: 'bearer',
      expires_in: 3600,
      user: { id: 'user-42', email: 'ada@example.com', app_metadata: {}, user_metadata: {} },
    });

  beforeEach(async () => {
    container = document.createElement('div');
    document.body.appendChild(container);
    cleanup = null;
    realFetch = globalThis.fetch;
    getAuthUserMock.mockReset();
    startAuthLoginMock.mockReset();
    signOutAuthMock.mockReset();
    readVaultSessionMock.mockReset();
    persistVaultSessionMock.mockReset();
    getAuthUserMock.mockResolvedValue(null);
    readVaultSessionMock.mockResolvedValue(null);
    persistVaultSessionMock.mockResolvedValue(true);
    signOutAuthMock.mockImplementation(async () => {
      const { authUserStore } = await import('../../frontend/src/auth/state/user.ts');
      authUserStore.set(null);
    });
    const { authUserStore } = await import('../../frontend/src/auth/state/user.ts');
    authUserStore.set(null);
    window.__TAURI__ = { event: { listen: vi.fn(async () => vi.fn()) } };
    vi.spyOn(api, 'getMessageChannelUnread').mockResolvedValue(false);
    vi.spyOn(api, 'markMessageChannelRead').mockResolvedValue(undefined);
    vi.spyOn(api, 'invoke').mockImplementation(async (cmd) => {
      if (cmd === 'query_binding') return { state: 'unbound' };
      if (cmd === 'list_chat_sessions') return { sessions: [], current_session_id: '' };
      return {};
    });
    syncDocuments = vi
      .spyOn(api, 'syncDocuments')
      .mockResolvedValue(doneResult({ uploaded: 1, skipped: 2 }));
    cancelSyncDocuments = vi
      .spyOn(api, 'cancelSyncDocuments')
      .mockResolvedValue({ ok: true, cancelling: true });
    globalThis.fetch = vi.fn(async () => {
      throw new Error('the network was not expected to be used');
    });
  });

  afterEach(async () => {
    cleanup?.();
    vi.restoreAllMocks();
    globalThis.fetch = realFetch;
    const { cloudSyncStore } = await import('../../frontend/src/home/state/cloud-sync.ts');
    cloudSyncStore.set({ phase: 'idle' });
    container.remove();
    delete window.__TAURI__;
  });

  async function mountHome({ signedIn }) {
    if (signedIn) getAuthUserMock.mockResolvedValue(signedUser());
    const { mountHomeHub } = await import('../../frontend/src/home/hub.tsx');
    cleanup = mountHomeHub(container, { navigate: vi.fn() });
    await vi.waitFor(() => {
      expect(container.querySelector('[data-role="account-bar"]')).not.toBeNull();
    });
    if (signedIn) {
      await vi.waitFor(() => {
        expect(container.querySelector('[data-role="account-bar-name"]')?.textContent).toBe('Ada');
      });
    }
  }

  const q = (role) => container.querySelector(`[data-role="${role}"]`);
  const trigger = () => q('sync-trigger');
  const title = () => q('sync-status-title')?.textContent;
  const detail = () => q('sync-status-detail')?.textContent;
  const pressSync = () => {
    act(() => {
      trigger().click();
    });
  };
  const pressCancel = () => {
    act(() => {
      q('sync-cancel').click();
    });
  };
  const later = (ms) => act(async () => { await new Promise((resolve) => setTimeout(resolve, ms)); });

  it('puts the icon beside the ?, and leaves the account menu to signing in and out', async () => {
    await mountHome({ signedIn: true });

    const row = q('account-row');
    const order = [...row.querySelectorAll('[data-role="account-bar"],[data-role="sync-button"],[data-role="help-menu"]')]
      .map((el) => el.getAttribute('data-role'));
    expect(order).toEqual(['account-bar', 'sync-button', 'help-menu']);
    act(() => {
      q('account-bar-trigger').click();
    });
    expect(container.querySelector('[data-role="account-logout"]')).not.toBeNull();
    expect(container.querySelector('[data-role="account-sync"]')).toBeNull();
  });

  it('does nothing until the icon is pressed', async () => {
    readVaultSessionMock.mockResolvedValue(vaultSession(3600));
    await mountHome({ signedIn: true });
    await later(20);

    expect(syncDocuments).not.toHaveBeenCalled();
    expect(q('sync-status')).toBeNull();
    expect(trigger().getAttribute('data-state')).toBe('idle');
  });

  it('signed out: asks the user to sign in first and does not ask the host to sync', async () => {
    await mountHome({ signedIn: false });
    pressSync();

    await vi.waitFor(() => {
      expect(detail()).toBe('Sign in first to sync.');
    });
    expect(title()).toBe('Cannot sync');
    expect(syncDocuments).not.toHaveBeenCalled();
  });

  it('signed in: keeps loading for as long as the host works, then shows the counts', async () => {
    readVaultSessionMock.mockResolvedValue(vaultSession(3600));
    let finish;
    syncDocuments.mockReturnValue(new Promise((resolve) => { finish = resolve; }));
    await mountHome({ signedIn: true });
    pressSync();

    await vi.waitFor(() => {
      expect(title()).toBe('Syncing…');
    });
    for (let i = 0; i < 3; i += 1) {
      await later(30);
      expect(trigger().getAttribute('data-state')).toBe('syncing');
      expect(trigger().getAttribute('aria-busy')).toBe('true');
      expect(title()).toBe('Syncing…');
    }
    pressSync();
    expect(syncDocuments).toHaveBeenCalledTimes(1);

    await act(async () => {
      finish(doneResult({ uploaded: 1, skipped: 2 }));
    });

    await vi.waitFor(() => {
      expect(detail()).toBe('Uploaded 1 · Pulled 0 · Unchanged 2');
    });
    expect(title()).toBe('Sync finished');
    expect(trigger().getAttribute('data-state')).toBe('idle');
    expect(q('sync-cancel')).toBeNull();
    expect(globalThis.fetch).not.toHaveBeenCalled();
  });

  it('cancel: the host is asked, the panel says cancelling, and the answer says what was done', async () => {
    readVaultSessionMock.mockResolvedValue(vaultSession(3600));
    let finish;
    syncDocuments.mockReturnValue(new Promise((resolve) => { finish = resolve; }));
    await mountHome({ signedIn: true });
    pressSync();
    await vi.waitFor(() => {
      expect(syncDocuments).toHaveBeenCalledTimes(1);
    });
    expect(q('sync-cancel')).not.toBeNull();

    pressCancel();

    await vi.waitFor(() => {
      expect(title()).toBe('Cancelling…');
    });
    expect(cancelSyncDocuments).toHaveBeenCalledTimes(1);
    expect(q('sync-cancel').disabled).toBe(true);
    expect(trigger().getAttribute('data-state')).toBe('syncing');

    await act(async () => {
      finish(doneResult({ uploaded: 1, not_tried: 3, cancelled: true }));
    });

    await vi.waitFor(() => {
      expect(title()).toBe('Sync cancelled');
    });
    expect(detail()).toBe('Uploaded 1 · Pulled 0 · Unchanged 0 · 3 not tried, sync again to continue');
    expect(trigger().getAttribute('data-state')).toBe('idle');
    expect(q('sync-cancel')).toBeNull();
  });

  it('cancel while the sign-in is being refreshed: the refresh is kept, the host is never asked to sync', async () => {
    readVaultSessionMock.mockResolvedValue(vaultSession(30));
    let finishFetch;
    globalThis.fetch = vi.fn(() => new Promise((resolve) => { finishFetch = () => resolve(refreshed()); }));
    await mountHome({ signedIn: true });
    pressSync();
    await vi.waitFor(() => {
      expect(globalThis.fetch).toHaveBeenCalled();
    });

    pressCancel();
    await vi.waitFor(() => {
      expect(title()).toBe('Cancelling…');
    });
    await act(async () => {
      finishFetch();
    });

    await vi.waitFor(() => {
      expect(title()).toBe('Sync cancelled');
    });
    expect(detail()).toBe('Nothing was sent or changed.');
    expect(syncDocuments).not.toHaveBeenCalled();
    expect(persistVaultSessionMock).toHaveBeenCalledTimes(1);
  });

  it('a session about to expire is refreshed and stored first, then the host syncs', async () => {
    readVaultSessionMock.mockResolvedValue(vaultSession(30));
    globalThis.fetch = vi.fn(async () => refreshed());
    await mountHome({ signedIn: true });
    pressSync();

    await vi.waitFor(() => {
      expect(detail()).toBe('Uploaded 1 · Pulled 0 · Unchanged 2');
    });
    expect(persistVaultSessionMock).toHaveBeenCalledTimes(1);
    expect(persistVaultSessionMock.mock.calls[0][0]).toMatchObject({
      access_token: 'new-access',
      refresh_token: 'new-refresh',
    });
    expect(syncDocuments).toHaveBeenCalledTimes(1);
    expect(persistVaultSessionMock.mock.invocationCallOrder[0]).toBeLessThan(
      syncDocuments.mock.invocationCallOrder[0],
    );
  });

  it('a session that cannot be refreshed: asks to sign in again and does not sync', async () => {
    readVaultSessionMock.mockResolvedValue(vaultSession(30));
    globalThis.fetch = vi.fn(async () =>
      json(400, { error: 'invalid_grant', error_description: 'Invalid Refresh Token: Already Used' }),
    );
    await mountHome({ signedIn: true });
    pressSync();

    await vi.waitFor(() => {
      expect(detail()).toBe('Your sign-in is no longer valid. Sign in again to sync.');
    });
    expect(title()).toBe('Cannot sync');
    expect(syncDocuments).not.toHaveBeenCalled();
    expect(persistVaultSessionMock).not.toHaveBeenCalled();
    expect(trigger().getAttribute('data-state')).toBe('idle');
  });

  it('shows the reason when the host cannot make a sync', async () => {
    readVaultSessionMock.mockResolvedValue(vaultSession(3600));
    const refused = new Error('Cannot sync: cloud sync not reachable: connection refused');
    refused.status = 502;
    syncDocuments.mockRejectedValue(refused);
    await mountHome({ signedIn: true });
    pressSync();

    await vi.waitFor(() => {
      expect(detail()).toBe('Cannot sync: cloud sync not reachable: connection refused');
    });
    expect(trigger().getAttribute('data-tone')).toBe('warn');
  });

  it('syncs again when pressed after the panel was closed', async () => {
    readVaultSessionMock.mockResolvedValue(vaultSession(3600));
    await mountHome({ signedIn: true });
    pressSync();
    await vi.waitFor(() => {
      expect(title()).toBe('Sync finished');
    });
    act(() => {
      q('close-sync-panel').click();
    });

    pressSync();

    await vi.waitFor(() => {
      expect(syncDocuments).toHaveBeenCalledTimes(2);
    });
    await vi.waitFor(() => {
      expect(title()).toBe('Sync finished');
    });
  });

  it('forgets the last sync when the user signs out', async () => {
    readVaultSessionMock.mockResolvedValue(vaultSession(3600));
    syncDocuments.mockResolvedValue(doneResult({ failed: [{ id: 'd', op: 'upload', error: 'too big' }] }));
    await mountHome({ signedIn: true });
    pressSync();
    await vi.waitFor(() => {
      expect(title()).toBe('Synced, with problems');
    });
    expect(trigger().getAttribute('data-tone')).toBe('warn');
    act(() => {
      q('close-sync-panel').click();
    });

    act(() => {
      q('account-bar-trigger').click();
    });
    act(() => {
      container.querySelector('[data-role="account-logout"]').click();
    });
    await vi.waitFor(() => {
      expect(signOutAuthMock).toHaveBeenCalledTimes(1);
      expect(q('account-bar-name')).toBeNull();
    });

    await vi.waitFor(() => {
      expect(trigger().hasAttribute('data-tone')).toBe(false);
    });
    expect(q('sync-status')).toBeNull();
  });
});
