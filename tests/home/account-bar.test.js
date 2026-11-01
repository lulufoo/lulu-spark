// @vitest-environment jsdom
import { existsSync, readFileSync } from 'node:fs';
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

describe('account bar · source', () => {
  it('puts one AccountBar at the bottom of the left rail, not inside Settings', () => {
    const page = readRel('frontend/src/home/page.tsx');
    expect(page).toMatch(/from ['"]\.\/ui\/account-bar\.tsx['"]/);
    expect(page).toMatch(/<AccountBar/);
    expect(page).toMatch(/getAuthUser|startAuthLogin|signOutAuth/);
    expect(page).not.toMatch(/linkIdentity/);

    const asideStart = page.indexOf('className="home-chat-sidebar"');
    const asideEnd = page.indexOf('</aside>');
    const sessions = page.indexOf('data-role="session-list"', asideStart);
    const account = page.indexOf('<AccountBar', asideStart);
    expect(asideStart).toBeGreaterThan(-1);
    expect(sessions).toBeGreaterThan(asideStart);
    expect(account).toBeGreaterThan(sessions);
    expect(account).toBeLessThan(asideEnd);

    const settings = readRel('frontend/src/app-shell/ui/settings/dialog.tsx');
    const settingsChrome = readRel('frontend/src/app-shell/ui/settings/chrome.tsx');
    expect(page).toMatch(/<HelpMenu/);
    expect(readRel('frontend/src/home/ui/help-menu.tsx')).toMatch(/data-role="help-menu-about"/);
    expect(`${settings}\n${settingsChrome}`).not.toMatch(/startAuthLogin|signOutAuth|AccountBar/);
  });

  it('defines AccountBar with the work-order props and no Auth HTTP', () => {
    const path = join(repoRoot, 'frontend/src/home/ui/account-bar.tsx');
    expect(existsSync(path)).toBe(true);
    const src = readRel('frontend/src/home/ui/account-bar.tsx');
    expect(src).toMatch(/export function AccountBar/);
    expect(src).toMatch(/onLogin/);
    expect(src).toMatch(/onLogout/);
    expect(src).toMatch(/display_name/);
    expect(src).not.toMatch(/linkIdentity/);
    expect(src).not.toMatch(/startAuthLogin|signOutAuth|getAuthUser/);
    expect(src).toMatch(/isDebug/);
    expect(src).toMatch(/data-debug/);
    expect(src).not.toMatch(/getPackageSnapshot/);
  });

  it('keeps the sync entry pure paint: the page composes, AccountBar only shows and calls back', () => {
    const bar = readRel('frontend/src/home/ui/account-bar.tsx');
    expect(bar).toMatch(/onSync/);
    expect(bar).toMatch(/data-role="account-sync"/);
    expect(bar).not.toMatch(/syncDocuments|refreshAuthSession|runCloudSync|cloudSyncStore/);

    const page = readRel('frontend/src/home/page.tsx');
    expect(page).toMatch(/runCloudSync/);
    expect(page).toMatch(/resetCloudSync/);
    expect(page).toMatch(/onSync=\{onAccountSync\}/);
    expect(page).toMatch(/sync=\{cloudSync\}/);

    const css = readRel('frontend/app.css');
    expect(css).toMatch(/\.home-account-bar-sync-status\b/);
    expect(css).toMatch(
      /\.home-account-bar-sync-status\[data-tone="warn"\]\s*\{[^}]*color:\s*var\(--fg-danger\)/,
    );
  });

  it('styles a single bottom account slot, not two side-by-side provider buttons', () => {
    const css = readRel('frontend/app.css');
    expect(css).toMatch(/\.home-account-bar\b/);
    expect(css).toMatch(/\.home-account-bar-trigger\b/);
    expect(css).not.toMatch(/\.home-account-bar-providers\s*\{[^}]*flex-direction:\s*row/);
  });

  it('paints only the avatar background when data-debug is true', () => {
    const css = readRel('frontend/app.css');
    expect(css).toMatch(
      /\.home-account-bar\[data-debug="true"\][^{]*\{[^}]*background:\s*var\(--bg-danger\)/,
    );
    expect(css).not.toMatch(
      /\.home-account-bar\[data-debug="true"\][^{]*\{[^}]*(box-shadow|outline|border):/,
    );
  });
});

describe('AccountBar', () => {
  let container;
  let root;
  let AccountBar;

  beforeEach(async () => {
    ({ AccountBar } = await import('../../frontend/src/home/ui/account-bar.tsx'));
    container = document.createElement('div');
    document.body.appendChild(container);
    root = createRoot(container);
  });

  afterEach(() => {
    act(() => {
      root?.unmount();
    });
    container?.remove();
  });

  function render(props) {
    act(() => {
      root.render(
        createElement(AccountBar, {
          user: null,
          onLogin: vi.fn(),
          onLogout: vi.fn(),
          ...props,
        }),
      );
    });
  }

  function bars() {
    return container.querySelectorAll('[data-role="account-bar"]');
  }

  function trigger() {
    return container.querySelector('[data-role="account-bar-trigger"]');
  }

  it('renders one clickable slot and reveals Google / GitHub only after click', () => {
    const onLogin = vi.fn();
    render({ onLogin });
    expect(bars().length).toBe(1);
    expect(trigger()).not.toBeNull();
    const guest = container.querySelector('[data-role="account-bar-avatar"]');
    expect(guest).not.toBeNull();
    expect(guest.getAttribute('data-placeholder')).toBe('true');
    expect(bars()[0].getAttribute('data-debug')).toBe('false');
    expect(container.querySelector('.home-account-bar-label').textContent).toBe('Sign in');
    expect(container.querySelector('[data-role="account-login-google"]')).toBeNull();
    expect(container.querySelector('[data-role="account-login-github"]')).toBeNull();

    act(() => {
      trigger().click();
    });
    const google = container.querySelector('[data-role="account-login-google"]');
    const github = container.querySelector('[data-role="account-login-github"]');
    expect(google).not.toBeNull();
    expect(github).not.toBeNull();
    expect(bars().length).toBe(1);

    act(() => {
      google.click();
    });
    expect(onLogin).toHaveBeenCalledWith('google');

    act(() => {
      trigger().click();
    });
    act(() => {
      container.querySelector('[data-role="account-login-github"]').click();
    });
    expect(onLogin).toHaveBeenCalledWith('github');
  });

  it('marks the bar data-debug when isDebug is true', () => {
    render({ isDebug: true });
    expect(bars()[0].getAttribute('data-debug')).toBe('true');
  });

  it('shows name and avatar when signed in, then name + sign out on click', () => {
    const onLogout = vi.fn();
    render({ user: signedUser(), onLogout });
    expect(container.querySelector('[data-role="account-bar-name"]').textContent).toBe('Ada');
    const avatar = container.querySelector('[data-role="account-bar-avatar"]');
    expect(avatar).not.toBeNull();
    expect(avatar.getAttribute('src')).toBe('https://example.com/a.png');
    expect(container.querySelector('[data-role="account-logout"]')).toBeNull();

    act(() => {
      trigger().click();
    });
    expect(container.textContent).toContain('Ada');
    const logout = container.querySelector('[data-role="account-logout"]');
    expect(logout).not.toBeNull();

    act(() => {
      logout.click();
    });
    expect(onLogout).toHaveBeenCalledTimes(1);
  });

  it('keeps a recognizable signed-in label when the name is empty', () => {
    render({
      user: signedUser({ display_name: '', avatar_url: null }),
    });
    expect(container.querySelector('[data-role="account-bar-name"]').textContent).toBe(
      'ada@example.com',
    );
    const placeholder = container.querySelector('[data-role="account-bar-avatar"]');
    expect(placeholder).not.toBeNull();
    expect(placeholder.getAttribute('data-placeholder')).toBe('true');
    expect(placeholder.textContent).toBe('A');

    render({
      user: signedUser({ display_name: '', email: null, avatar_url: null }),
    });
    expect(container.querySelector('[data-role="account-bar-name"]').textContent).toBe(
      'Signed in',
    );
    const fallback = container.querySelector('[data-role="account-bar-avatar"]');
    expect(fallback.getAttribute('data-placeholder')).toBe('true');
    expect(fallback.textContent).toBe('L');
  });

  it('falls back to a local placeholder when the avatar image fails', () => {
    render({ user: signedUser() });
    const avatar = container.querySelector('[data-role="account-bar-avatar"]');
    expect(avatar.tagName).toBe('IMG');
    act(() => {
      avatar.dispatchEvent(new Event('error'));
    });
    const placeholder = container.querySelector('[data-role="account-bar-avatar"]');
    expect(placeholder.getAttribute('data-placeholder')).toBe('true');
    expect(placeholder.textContent).toBe('A');
  });

  it('stays usable when login or logout callbacks throw', () => {
    const onLogin = vi.fn(() => {
      throw new Error('login failed');
    });
    const onLogout = vi.fn(() => {
      throw new Error('logout failed');
    });
    render({ onLogin });
    act(() => {
      trigger().click();
    });
    act(() => {
      container.querySelector('[data-role="account-login-google"]').click();
    });
    expect(bars().length).toBe(1);
    act(() => {
      trigger().click();
    });
    expect(container.querySelector('[data-role="account-login-github"]')).not.toBeNull();

    render({ user: signedUser(), onLogout });
    act(() => {
      trigger().click();
    });
    act(() => {
      container.querySelector('[data-role="account-logout"]').click();
    });
    expect(bars().length).toBe(1);
    act(() => {
      trigger().click();
    });
    expect(container.querySelector('[data-role="account-logout"]')).not.toBeNull();
  });
});

describe('AccountBar · sync entry', () => {
  let container;
  let root;
  let AccountBar;

  beforeEach(async () => {
    ({ AccountBar } = await import('../../frontend/src/home/ui/account-bar.tsx'));
    container = document.createElement('div');
    document.body.appendChild(container);
    root = createRoot(container);
  });

  afterEach(() => {
    act(() => {
      root?.unmount();
    });
    container?.remove();
  });

  function render(props) {
    act(() => {
      root.render(
        createElement(AccountBar, { user: null, onLogin: vi.fn(), onLogout: vi.fn(), ...props }),
      );
    });
  }

  function openMenu() {
    act(() => {
      container.querySelector('[data-role="account-bar-trigger"]').click();
    });
  }

  const syncButton = () => container.querySelector('[data-role="account-sync"]');
  const statusLine = () => container.querySelector('[data-role="account-sync-status"]');

  const who = [
    ['signed out', null],
    ['signed in', signedUser()],
  ];

  it.each(who)('is not offered without a way to sync (%s)', (_name, user) => {
    render({ user });
    openMenu();
    expect(syncButton()).toBeNull();
    expect(statusLine()).toBeNull();
  });

  it.each(who)('is offered in the menu, and only once it is open (%s)', (_name, user) => {
    render({ user, onSync: vi.fn() });
    expect(syncButton()).toBeNull();
    openMenu();
    expect(syncButton()).not.toBeNull();
    expect(syncButton().textContent).toBe('Sync now');
  });

  it.each(who)('calls back once per click and keeps the menu open (%s)', (_name, user) => {
    const onSync = vi.fn();
    render({ user, onSync });
    openMenu();

    act(() => {
      syncButton().click();
    });

    expect(onSync).toHaveBeenCalledTimes(1);
    expect(syncButton()).not.toBeNull();
  });

  it('shows Syncing…, disables the entry and ignores clicks while a sync is going on', () => {
    const onSync = vi.fn();
    render({ user: signedUser(), onSync, sync: { phase: 'syncing' } });
    openMenu();

    expect(syncButton().textContent).toBe('Syncing…');
    expect(syncButton().disabled).toBe(true);
    expect(syncButton().getAttribute('aria-busy')).toBe('true');
    expect(statusLine()).toBeNull();

    act(() => {
      syncButton().click();
    });
    expect(onSync).not.toHaveBeenCalled();
  });

  it('is ready again once the sync is over', () => {
    render({ user: signedUser(), onSync: vi.fn(), sync: { phase: 'syncing' } });
    openMenu();
    render({
      user: signedUser(),
      onSync: vi.fn(),
      sync: { phase: 'done', result: doneResult({ uploaded: 1 }) },
    });
    expect(syncButton().disabled).toBe(false);
    expect(syncButton().getAttribute('aria-busy')).toBe('false');
    expect(syncButton().textContent).toBe('Sync now');
  });

  const lines = [
    ['nothing yet', { phase: 'idle' }, null, null],
    [
      'a sync that was made',
      { phase: 'done', result: doneResult({ uploaded: 2, pulled: 1, skipped: 4 }) },
      'neutral',
      'Uploaded 2 · Pulled 1 · Unchanged 4',
    ],
    [
      'a sync with a failed document',
      { phase: 'done', result: doneResult({ failed: [{ id: 'd', op: 'pull', error: 'bad zip' }] }) },
      'warn',
      'Uploaded 0 · Pulled 0 · Unchanged 0 · 1 failed · first: bad zip',
    ],
    ['signed out', { phase: 'refused', reason: 'signed_out' }, 'warn', 'Sign in first to sync.'],
    [
      'a sign-in that cannot be refreshed',
      { phase: 'refused', reason: 'session_invalid' },
      'warn',
      'Your sign-in is no longer valid. Sign in again to sync.',
    ],
    [
      'a refused call',
      { phase: 'failed', status: 502, message: 'Cannot sync: cloud sync not reachable' },
      'warn',
      'Cannot sync: cloud sync not reachable',
    ],
  ];

  it.each(lines)('says what happened after %s', (_name, sync, tone, text) => {
    render({ user: signedUser(), onSync: vi.fn(), sync });
    openMenu();
    if (text === null) {
      expect(statusLine()).toBeNull();
      return;
    }
    expect(statusLine().textContent).toBe(text);
    expect(statusLine().getAttribute('data-tone')).toBe(tone);
    expect(statusLine().getAttribute('role')).toBe('status');
  });

  it('stays usable when the sync callback throws', () => {
    const onSync = vi.fn(() => {
      throw new Error('sync failed');
    });
    render({ user: signedUser(), onSync });
    openMenu();
    act(() => {
      syncButton().click();
    });
    expect(container.querySelectorAll('[data-role="account-bar"]').length).toBe(1);
    expect(syncButton()).not.toBeNull();
  });

  it('keeps the sign-out entry beside it when signed in', () => {
    render({ user: signedUser(), onSync: vi.fn() });
    openMenu();
    expect(container.querySelector('[data-role="account-logout"]')).not.toBeNull();
  });

  it('keeps both sign-in entries beside it when signed out', () => {
    render({ user: null, onSync: vi.fn() });
    openMenu();
    expect(container.querySelector('[data-role="account-login-google"]')).not.toBeNull();
    expect(container.querySelector('[data-role="account-login-github"]')).not.toBeNull();
  });
});

describe('account bar · home rail', () => {
  let container;
  let cleanup;

  beforeEach(async () => {
    container = document.createElement('div');
    document.body.appendChild(container);
    cleanup = null;
    getAuthUserMock.mockReset();
    startAuthLoginMock.mockReset();
    signOutAuthMock.mockReset();
    getAuthUserMock.mockResolvedValue(null);
    startAuthLoginMock.mockResolvedValue(undefined);
    signOutAuthMock.mockImplementation(async () => {
      const { authUserStore } = await import('../../frontend/src/auth/state/user.ts');
      authUserStore.set(null);
    });
    const { authUserStore } = await import('../../frontend/src/auth/state/user.ts');
    authUserStore.set(null);
    window.__TAURI__ = {
      event: {
        listen: vi.fn(async () => vi.fn()),
      },
    };
    vi.spyOn(api, 'getMessageChannelUnread').mockResolvedValue(false);
    vi.spyOn(api, 'markMessageChannelRead').mockResolvedValue(undefined);
    vi.spyOn(api, 'invoke').mockImplementation(async (cmd) => {
      if (cmd === 'query_binding') return { state: 'unbound' };
      if (cmd === 'list_chat_sessions') return { sessions: [], current_session_id: '' };
      return {};
    });
  });

  afterEach(() => {
    cleanup?.();
    vi.restoreAllMocks();
    container.remove();
    delete window.__TAURI__;
  });

  async function mountHome() {
    const { mountHomeHub } = await import('../../frontend/src/home/hub.tsx');
    cleanup = mountHomeHub(container, { navigate: vi.fn() });
    await vi.waitFor(() => {
      expect(container.querySelector('[data-role="account-bar"]')).not.toBeNull();
    });
  }

  it('loads the unsigned bar and calls startAuthLogin from the rail, not Settings', async () => {
    await mountHome();
    const sidebar = container.querySelector('.home-chat-sidebar');
    const bar = sidebar.querySelector('[data-role="account-bar"]');
    expect(bar).not.toBeNull();
    expect(container.querySelectorAll('[data-role="account-bar"]').length).toBe(1);
    expect(container.querySelector('[data-role="help-menu-trigger"]')).not.toBeNull();
    expect(container.querySelector('#settings-dialog [data-role="account-bar"]')).toBeNull();

    const sessions = sidebar.querySelector('[data-role="session-list"]');
    expect(
      sessions.compareDocumentPosition(bar) & Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();

    act(() => {
      bar.querySelector('[data-role="account-bar-trigger"]').click();
    });
    act(() => {
      bar.querySelector('[data-role="account-login-google"]').click();
    });
    expect(startAuthLoginMock).toHaveBeenCalledWith('google');
    expect(startAuthLoginMock).toHaveBeenCalledTimes(1);
  });

  it('shows the signed-in name, signs out, and returns to the unsigned bar', async () => {
    getAuthUserMock.mockResolvedValue(signedUser());
    await mountHome();
    await vi.waitFor(() => {
      expect(container.querySelector('[data-role="account-bar-name"]')?.textContent).toBe('Ada');
    });

    act(() => {
      container.querySelector('[data-role="account-bar-trigger"]').click();
    });
    act(() => {
      container.querySelector('[data-role="account-logout"]').click();
    });
    await vi.waitFor(() => {
      expect(signOutAuthMock).toHaveBeenCalledTimes(1);
    });
    getAuthUserMock.mockResolvedValue(null);
    await vi.waitFor(() => {
      expect(container.querySelector('[data-role="account-bar-name"]')).toBeNull();
      expect(container.querySelector('[data-role="account-bar-trigger"]')).not.toBeNull();
    });
  });

  it('does not crash when startAuthLogin or signOutAuth reject', async () => {
    startAuthLoginMock.mockRejectedValue(new Error('oauth down'));
    await mountHome();
    act(() => {
      container.querySelector('[data-role="account-bar-trigger"]').click();
    });
    act(() => {
      container.querySelector('[data-role="account-login-github"]').click();
    });
    await vi.waitFor(() => {
      expect(startAuthLoginMock).toHaveBeenCalledWith('github');
    });
    expect(container.querySelector('[data-role="account-bar"]')).not.toBeNull();
    act(() => {
      container.querySelector('[data-role="account-bar-trigger"]').click();
    });
    expect(container.querySelector('[data-role="account-login-google"]')).not.toBeNull();

    cleanup?.();
    cleanup = null;
    getAuthUserMock.mockResolvedValue(signedUser());
    signOutAuthMock.mockRejectedValue(new Error('sign-out down'));
    await mountHome();
    await vi.waitFor(() => {
      expect(container.querySelector('[data-role="account-bar-name"]')?.textContent).toBe('Ada');
    });
    act(() => {
      container.querySelector('[data-role="account-bar-trigger"]').click();
    });
    act(() => {
      container.querySelector('[data-role="account-logout"]').click();
    });
    await vi.waitFor(() => {
      expect(signOutAuthMock).toHaveBeenCalledTimes(1);
    });
    expect(container.querySelector('[data-role="account-bar"]')).not.toBeNull();
  });

  it('repaints the rail when the auth user store updates after mount', async () => {
    await mountHome();
    expect(container.querySelector('[data-role="account-bar-name"]')).toBeNull();
    const { authUserStore } = await import('../../frontend/src/auth/state/user.ts');
    act(() => {
      authUserStore.set(signedUser());
    });
    await vi.waitFor(() => {
      expect(container.querySelector('[data-role="account-bar-name"]')?.textContent).toBe('Ada');
    });
  });
});

describe('account bar · sync from the home rail', () => {
  let container;
  let cleanup;
  let realFetch;
  let syncDocuments;

  const nowSeconds = () => Math.floor(Date.now() / 1000);
  const vaultSession = (expiresIn) => ({
    access_token: 'vault-access',
    refresh_token: 'vault-refresh',
    expires_at: nowSeconds() + expiresIn,
    user: { id: 'user-42', email: 'ada@example.com' },
  });
  const json = (status, body) =>
    new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } });

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

  function openMenu() {
    act(() => {
      container.querySelector('[data-role="account-bar-trigger"]').click();
    });
  }

  function pressSync() {
    act(() => {
      container.querySelector('[data-role="account-sync"]').click();
    });
  }

  const statusText = () => container.querySelector('[data-role="account-sync-status"]')?.textContent;

  it('signed out: asks the user to sign in first and does not ask the host to sync', async () => {
    await mountHome({ signedIn: false });
    openMenu();
    pressSync();

    await vi.waitFor(() => {
      expect(statusText()).toBe('Sign in first to sync.');
    });
    expect(syncDocuments).not.toHaveBeenCalled();
    expect(container.querySelector('[data-role="account-login-google"]')).not.toBeNull();
  });

  it('signed in: shows Syncing… while the host works, then the counts', async () => {
    readVaultSessionMock.mockResolvedValue(vaultSession(3600));
    let finish;
    syncDocuments.mockReturnValue(new Promise((resolve) => { finish = resolve; }));
    await mountHome({ signedIn: true });
    openMenu();
    pressSync();

    await vi.waitFor(() => {
      expect(container.querySelector('[data-role="account-sync"]').textContent).toBe('Syncing…');
    });
    expect(container.querySelector('[data-role="account-sync"]').disabled).toBe(true);

    await act(async () => {
      finish(doneResult({ uploaded: 1, skipped: 2 }));
    });

    await vi.waitFor(() => {
      expect(statusText()).toBe('Uploaded 1 · Pulled 0 · Unchanged 2');
    });
    expect(container.querySelector('[data-role="account-sync"]').textContent).toBe('Sync now');
    expect(container.querySelector('[data-role="account-sync"]').disabled).toBe(false);
    expect(globalThis.fetch).not.toHaveBeenCalled();
  });

  it('a session about to expire is refreshed and stored first, then the host syncs', async () => {
    readVaultSessionMock.mockResolvedValue(vaultSession(30));
    globalThis.fetch = vi.fn(async () =>
      json(200, {
        access_token: 'new-access',
        refresh_token: 'new-refresh',
        token_type: 'bearer',
        expires_in: 3600,
        user: { id: 'user-42', email: 'ada@example.com', app_metadata: {}, user_metadata: {} },
      }),
    );
    await mountHome({ signedIn: true });
    openMenu();
    pressSync();

    await vi.waitFor(() => {
      expect(statusText()).toBe('Uploaded 1 · Pulled 0 · Unchanged 2');
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
    openMenu();
    pressSync();

    await vi.waitFor(() => {
      expect(statusText()).toBe('Your sign-in is no longer valid. Sign in again to sync.');
    });
    expect(syncDocuments).not.toHaveBeenCalled();
    expect(persistVaultSessionMock).not.toHaveBeenCalled();
    expect(container.querySelector('[data-role="account-sync"]').disabled).toBe(false);
  });

  it('shows the reason when the host cannot make a sync', async () => {
    readVaultSessionMock.mockResolvedValue(vaultSession(3600));
    const refused = new Error('Cannot sync: cloud sync not reachable: connection refused');
    refused.status = 502;
    syncDocuments.mockRejectedValue(refused);
    await mountHome({ signedIn: true });
    openMenu();
    pressSync();

    await vi.waitFor(() => {
      expect(statusText()).toBe('Cannot sync: cloud sync not reachable: connection refused');
    });
  });

  it('forgets the last sync when the user signs out', async () => {
    readVaultSessionMock.mockResolvedValue(vaultSession(3600));
    await mountHome({ signedIn: true });
    openMenu();
    pressSync();
    await vi.waitFor(() => {
      expect(statusText()).toBe('Uploaded 1 · Pulled 0 · Unchanged 2');
    });

    act(() => {
      container.querySelector('[data-role="account-logout"]').click();
    });
    await vi.waitFor(() => {
      expect(signOutAuthMock).toHaveBeenCalledTimes(1);
      expect(container.querySelector('[data-role="account-bar-name"]')).toBeNull();
    });

    openMenu();
    expect(container.querySelector('[data-role="account-sync"]')).not.toBeNull();
    expect(statusText()).toBeUndefined();
  });
});
