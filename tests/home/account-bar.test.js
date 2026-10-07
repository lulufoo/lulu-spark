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

vi.mock('../../frontend/src/auth/oauth.ts', () => ({
  getAuthUser: (...args) => getAuthUserMock(...args),
  startAuthLogin: (...args) => startAuthLoginMock(...args),
  signOutAuth: (...args) => signOutAuthMock(...args),
}));

function readRel(rel) {
  return readFileSync(join(repoRoot, rel), 'utf8');
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
    expect(page).toMatch(/id="btn-settings"/);
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
  });

  it('styles a single bottom account slot, not two side-by-side provider buttons', () => {
    const css = readRel('frontend/app.css');
    expect(css).toMatch(/\.home-account-bar\b/);
    expect(css).toMatch(/\.home-account-bar-trigger\b/);
    expect(css).not.toMatch(/\.home-account-bar-providers\s*\{[^}]*flex-direction:\s*row/);
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
    expect(container.querySelector('[data-role="account-bar-avatar"]')).toBeNull();

    render({
      user: signedUser({ display_name: '', email: null, avatar_url: null }),
    });
    expect(container.querySelector('[data-role="account-bar-name"]').textContent).toBe(
      'Signed in',
    );
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
    expect(container.querySelector('#btn-settings')).not.toBeNull();
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
