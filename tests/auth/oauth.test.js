// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const CALLBACK = 'spark://auth-login/callback';
const GOOGLE_URL = 'https://accounts.google.com/o/oauth2/v2/auth?client=google';
const GITHUB_URL = 'https://github.com/login/oauth/authorize?client=github';

const signInWithOAuth = vi.fn();
const exchangeCodeForSession = vi.fn();
const setSession = vi.fn();
const signOut = vi.fn();
const linkIdentity = vi.fn();
const createClient = vi.fn(() => ({
  auth: {
    signInWithOAuth,
    exchangeCodeForSession,
    setSession,
    signOut,
    linkIdentity,
  },
}));

vi.mock('@supabase/supabase-js', () => ({
  createClient: (...args) => createClient(...args),
}));

const {
  startAuthLogin,
  completeAuthLogin,
  signOutAuth,
  getAuthUser,
} = await import('../../frontend/src/auth/oauth.ts');

function readRel(rel) {
  return readFileSync(join(repoRoot, rel), 'utf8');
}

function vaultSession(overrides = {}) {
  return {
    access_token: 'access-aaa',
    refresh_token: 'refresh-bbb',
    expires_at: 1_800_000_000,
    user: {
      id: 'user-42',
      email: 'ada@example.com',
      name: 'Ada',
      avatar: 'https://example.com/a.png',
      provider: 'google',
      ...overrides.user,
    },
    ...overrides,
  };
}

function installLocalStorageMock() {
  const store = {};
  const storage = {
    getItem(key) {
      return Object.prototype.hasOwnProperty.call(store, key) ? store[key] : null;
    },
    setItem(key, value) {
      store[key] = String(value);
    },
    removeItem(key) {
      delete store[key];
    },
    clear() {
      for (const key of Object.keys(store)) delete store[key];
    },
    get length() {
      return Object.keys(store).length;
    },
  };
  globalThis.localStorage = storage;
  window.localStorage = storage;
}

function seedInvoke(session = null) {
  const invoke = vi.fn(async (cmd) => {
    if (cmd === 'get_auth_session') return session;
    if (cmd === 'set_auth_session') return null;
    if (cmd === 'delete_auth_session') return null;
    throw new Error(`unexpected invoke ${cmd}`);
  });
  const openUrl = vi.fn().mockResolvedValue(undefined);
  window.__TAURI__ = {
    core: { invoke },
    opener: { openUrl },
  };
  return { invoke, openUrl };
}

beforeEach(async () => {
  signInWithOAuth.mockReset();
  exchangeCodeForSession.mockReset();
  setSession.mockReset();
  signOut.mockReset();
  linkIdentity.mockReset();
  createClient.mockClear();
  installLocalStorageMock();
  const { authUserStore } = await import('../../frontend/src/auth/state/user.ts');
  authUserStore.set(null);
});

afterEach(() => {
  delete window.__TAURI__;
  window.localStorage.clear();
  vi.restoreAllMocks();
});

describe('startAuthLogin', () => {
  it.each(['google', 'github'])(
    'asks supabase-js for a %s URL with skipBrowserRedirect and opens the system browser',
    async (provider) => {
      const url = provider === 'google' ? GOOGLE_URL : GITHUB_URL;
      signInWithOAuth.mockResolvedValue({ data: { url }, error: null });
      const { openUrl, invoke } = seedInvoke(null);

      await startAuthLogin(provider);

      expect(signInWithOAuth).toHaveBeenCalledWith({
        provider,
        options: expect.objectContaining({
          skipBrowserRedirect: true,
          redirectTo: CALLBACK,
        }),
      });
      expect(openUrl).toHaveBeenCalledWith(url);
      expect(linkIdentity).not.toHaveBeenCalled();
      expect(invoke).not.toHaveBeenCalledWith('set_auth_session', expect.anything());
    },
  );
});

describe('completeAuthLogin', () => {
  it('exchanges a code callback and writes the Session through the vault command', async () => {
    const { invoke } = seedInvoke(null);
    exchangeCodeForSession.mockResolvedValue({
      data: {
        session: {
          access_token: 'access-aaa',
          refresh_token: 'refresh-bbb',
          expires_at: 1_800_000_000,
          provider_token: 'must-not-persist',
          user: {
            id: 'user-42',
            email: 'ada@example.com',
            user_metadata: { name: 'Ada', avatar_url: 'https://example.com/a.png' },
            app_metadata: { provider: 'google' },
          },
        },
      },
      error: null,
    });

    await expect(completeAuthLogin(`${CALLBACK}?code=abc`)).resolves.toBe(true);
    expect(exchangeCodeForSession).toHaveBeenCalledWith('abc');
    expect(setSession).not.toHaveBeenCalled();
    expect(invoke).toHaveBeenCalledWith(
      'set_auth_session',
      expect.objectContaining({
        session: expect.objectContaining({
          access_token: 'access-aaa',
          refresh_token: 'refresh-bbb',
          user: expect.objectContaining({
            id: 'user-42',
            name: 'Ada',
          }),
        }),
      }),
    );
    const payload = JSON.stringify(
      invoke.mock.calls.find((call) => call[0] === 'set_auth_session')[1],
    );
    expect(payload).not.toContain('provider_token');
    expect(payload).not.toContain('must-not-persist');
    expect(linkIdentity).not.toHaveBeenCalled();
    expect(window.localStorage.length).toBe(0);
    const { authUserStore } = await import('../../frontend/src/auth/state/user.ts');
    expect(authUserStore.getSnapshot()?.user_id).toBe('user-42');
  });

  it('applies a hash token callback through setSession and writes the Session', async () => {
    const { invoke } = seedInvoke(null);
    setSession.mockResolvedValue({
      data: {
        session: {
          access_token: 'access-aaa',
          refresh_token: 'refresh-bbb',
          expires_at: 1_800_000_000,
          provider_token: 'must-not-persist',
          user: {
            id: 'user-42',
            email: 'ada@example.com',
            user_metadata: { name: 'Ada', avatar_url: 'https://example.com/a.png' },
            app_metadata: { provider: 'google' },
          },
        },
      },
      error: null,
    });

    const hashUrl =
      `${CALLBACK}#access_token=access-aaa&refresh_token=refresh-bbb` +
      `&expires_at=1800000000&provider_token=must-not-persist`;
    await expect(completeAuthLogin(hashUrl)).resolves.toBe(true);
    expect(exchangeCodeForSession).not.toHaveBeenCalled();
    expect(setSession).toHaveBeenCalledWith({
      access_token: 'access-aaa',
      refresh_token: 'refresh-bbb',
    });
    expect(invoke).toHaveBeenCalledWith(
      'set_auth_session',
      expect.objectContaining({
        session: expect.objectContaining({
          access_token: 'access-aaa',
          refresh_token: 'refresh-bbb',
          user: expect.objectContaining({
            id: 'user-42',
            name: 'Ada',
          }),
        }),
      }),
    );
    const payload = JSON.stringify(
      invoke.mock.calls.find((call) => call[0] === 'set_auth_session')[1],
    );
    expect(payload).not.toContain('provider_token');
    expect(payload).not.toContain('must-not-persist');
    const { authUserStore } = await import('../../frontend/src/auth/state/user.ts');
    expect(authUserStore.getSnapshot()?.display_name).toBe('Ada');
  });

  it('does not call linkIdentity when the same email signs in with another provider', async () => {
    const { invoke } = seedInvoke(null);
    exchangeCodeForSession.mockResolvedValue({
      data: {
        session: {
          access_token: 'access-github',
          refresh_token: 'refresh-github',
          expires_at: 1_800_000_001,
          user: {
            id: 'user-42',
            email: 'ada@example.com',
            user_metadata: { name: 'Ada', avatar_url: 'https://example.com/a.png' },
            app_metadata: { provider: 'github' },
          },
        },
      },
      error: null,
    });

    await expect(completeAuthLogin(`${CALLBACK}?code=from-github`)).resolves.toBe(true);
    expect(linkIdentity).not.toHaveBeenCalled();
    expect(invoke).toHaveBeenCalledWith(
      'set_auth_session',
      expect.objectContaining({
        session: expect.objectContaining({
          user: expect.objectContaining({
            email: 'ada@example.com',
            provider: 'github',
          }),
        }),
      }),
    );
  });

  it.each([
    [`${CALLBACK}?error=access_denied`, 'error'],
    [CALLBACK, 'no code'],
    [`${CALLBACK}?error=access_denied&code=abc`, 'error wins'],
    [`${CALLBACK}#error=access_denied&access_token=access-aaa&refresh_token=refresh-bbb`, 'hash error'],
    [`${CALLBACK}#access_token=access-aaa`, 'hash access only'],
  ])('returns false and writes nothing for %s', async (url) => {
    const { invoke } = seedInvoke(null);
    await expect(completeAuthLogin(url)).resolves.toBe(false);
    expect(exchangeCodeForSession).not.toHaveBeenCalled();
    expect(setSession).not.toHaveBeenCalled();
    expect(invoke).not.toHaveBeenCalledWith('set_auth_session', expect.anything());
    expect(window.localStorage.length).toBe(0);
  });
});

describe('signOutAuth and getAuthUser', () => {
  it('clears the vault Session on sign-out', async () => {
    signOut.mockResolvedValue({ error: null });
    const { invoke } = seedInvoke(vaultSession());
    const { authUserStore } = await import('../../frontend/src/auth/state/user.ts');
    authUserStore.set({
      user_id: 'user-42',
      email: 'ada@example.com',
      display_name: 'Ada',
      avatar_url: 'https://example.com/a.png',
      provider: 'google',
    });
    await signOutAuth();
    expect(signOut).toHaveBeenCalled();
    expect(invoke).toHaveBeenCalledWith('delete_auth_session');
    expect(linkIdentity).not.toHaveBeenCalled();
    expect(authUserStore.getSnapshot()).toBeNull();
  });

  it('returns null when no Session is stored', async () => {
    seedInvoke(null);
    await expect(getAuthUser()).resolves.toBeNull();
  });

  it('reads user_id and display name from the stored Session', async () => {
    seedInvoke(vaultSession());
    await expect(getAuthUser()).resolves.toEqual({
      user_id: 'user-42',
      email: 'ada@example.com',
      display_name: 'Ada',
      avatar_url: 'https://example.com/a.png',
      provider: 'google',
    });
  });
});

describe('oauth source contract', () => {
  it('does not import Tauri packages, call linkIdentity, or persist to localStorage', () => {
    const src = readRel('frontend/src/auth/oauth.ts');
    expect(src).toMatch(/signInWithOAuth/);
    expect(src).toMatch(/skipBrowserRedirect/);
    expect(src).toContain(CALLBACK);
    expect(src).not.toMatch(/linkIdentity/);
    expect(src).not.toMatch(/localStorage/);
    expect(src).not.toMatch(/@tauri-apps\//);
    expect(src).not.toMatch(/webview|Webview/i);
  });
});
