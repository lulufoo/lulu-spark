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

import * as appLog from '../../frontend/src/host/app-log.ts';
import {
  beginAuthLoginWait,
  consumeMatchingAuthLoginPass,
} from '../../frontend/src/auth/login-wait.ts';
import { encodePassBag, extractPassIdFromScheme, PASS_QUERY } from '../../frontend/src/auth/pass.ts';

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
    if (cmd === 'log_app_event') return null;
    throw new Error(`unexpected invoke ${cmd}`);
  });
  const openUrl = vi.fn().mockResolvedValue(undefined);
  window.__TAURI__ = {
    core: { invoke },
    opener: { openUrl },
  };
  return { invoke, openUrl };
}

function clearAuthLoginWait() {
  beginAuthLoginWait('trace_clearwait01');
  consumeMatchingAuthLoginPass('trace_clearwait01');
}

function passQuery(id) {
  return `${PASS_QUERY}=${encodePassBag(id)}`;
}

function codeCallback(id, code = 'abc') {
  return `${CALLBACK}?${passQuery(id)}&code=${code}`;
}

function tokenCallback(id) {
  return `${CALLBACK}?${passQuery(id)}#access_token=access-aaa&refresh_token=refresh-bbb`;
}

async function startLogin(provider = 'google') {
  const url = provider === 'google' ? GOOGLE_URL : GITHUB_URL;
  signInWithOAuth.mockResolvedValue({ data: { url }, error: null });
  const seeded = seedInvoke(null);
  await startAuthLogin(provider);
  const redirectTo = signInWithOAuth.mock.calls.at(-1)[0].options.redirectTo;
  return { id: extractPassIdFromScheme(redirectTo), redirectTo, ...seeded };
}

function mockVaultSession(provider = 'google') {
  return {
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
          app_metadata: { provider },
        },
      },
    },
    error: null,
  };
}

beforeEach(async () => {
  signInWithOAuth.mockReset();
  exchangeCodeForSession.mockReset();
  setSession.mockReset();
  signOut.mockReset();
  linkIdentity.mockReset();
  createClient.mockClear();
  installLocalStorageMock();
  clearAuthLoginWait();
  const { authUserStore } = await import('../../frontend/src/auth/state/user.ts');
  authUserStore.set(null);
  vi.spyOn(appLog, 'logAppEvent').mockImplementation(() => {});
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

      const redirectTo = signInWithOAuth.mock.calls[0][0].options.redirectTo;
      const id = extractPassIdFromScheme(redirectTo);
      expect(signInWithOAuth).toHaveBeenCalledWith({
        provider,
        options: expect.objectContaining({
          skipBrowserRedirect: true,
          redirectTo,
        }),
      });
      expect(redirectTo.startsWith(`${CALLBACK}?pass=`)).toBe(true);
      expect(openUrl).toHaveBeenCalledWith(url);
      expect(id).toMatch(/^trace_[0-9a-f]{12}$/);
      expect(appLog.logAppEvent).toHaveBeenCalledWith(
        expect.objectContaining({
          business: 'login',
          event: 'auth.start',
          traceId: id,
          params: expect.objectContaining({ provider }),
        }),
      );
      expect(consumeMatchingAuthLoginPass(id)).toBe(true);
      expect(linkIdentity).not.toHaveBeenCalled();
      expect(invoke).not.toHaveBeenCalledWith('set_auth_session', expect.anything());
    },
  );

  it('writes open_fail when the system browser cannot open', async () => {
    signInWithOAuth.mockResolvedValue({ data: { url: GOOGLE_URL }, error: null });
    const { openUrl } = seedInvoke(null);
    openUrl.mockRejectedValue(new Error('no browser'));
    await expect(startAuthLogin('google')).rejects.toThrow('no browser');
    expect(appLog.logAppEvent).toHaveBeenCalledWith(
      expect.objectContaining({
        business: 'login',
        event: 'auth.start',
        params: expect.objectContaining({ provider: 'google', outcome: 'open_fail' }),
      }),
    );
  });
});

describe('completeAuthLogin', () => {
  it('exchanges a code callback and writes the Session through the vault command', async () => {
    const { invoke, id } = await startLogin();
    exchangeCodeForSession.mockResolvedValue(mockVaultSession());

    await expect(completeAuthLogin(codeCallback(id))).resolves.toBe('ok');
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
    const { invoke, id } = await startLogin();
    setSession.mockResolvedValue(mockVaultSession());

    const hashUrl =
      `${CALLBACK}?${passQuery(id)}#access_token=access-aaa&refresh_token=refresh-bbb` +
      `&expires_at=1800000000&provider_token=must-not-persist`;
    await expect(completeAuthLogin(hashUrl)).resolves.toBe('ok');
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
    const { invoke, id } = await startLogin();
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

    await expect(completeAuthLogin(codeCallback(id, 'from-github'))).resolves.toBe('ok');
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
    [`${CALLBACK}?error=access_denied`, 'fail'],
    [`${CALLBACK}?error=access_denied&code=abc`, 'fail'],
    [`${CALLBACK}#error=access_denied&access_token=access-aaa&refresh_token=refresh-bbb`, 'fail'],
  ])('returns %s and writes nothing for %s', async (url, outcome) => {
    const { invoke } = seedInvoke(null);
    await expect(completeAuthLogin(url)).resolves.toBe(outcome);
    expect(exchangeCodeForSession).not.toHaveBeenCalled();
    expect(setSession).not.toHaveBeenCalled();
    expect(invoke).not.toHaveBeenCalledWith('set_auth_session', expect.anything());
    expect(window.localStorage.length).toBe(0);
  });

  it('returns no_token when the paired callback has no ticket', async () => {
    const { id, invoke } = await startLogin();
    await expect(completeAuthLogin(`${CALLBACK}?${passQuery(id)}`)).resolves.toBe('no_token');
    expect(exchangeCodeForSession).not.toHaveBeenCalled();
    expect(setSession).not.toHaveBeenCalled();
    expect(invoke).not.toHaveBeenCalledWith('set_auth_session', expect.anything());
  });
});

describe('completeAuthLogin pass pairing', () => {
  it('returns ok and writes the vault session when the callback pass matches this start', async () => {
    const { id, invoke } = await startLogin('google');
    setSession.mockResolvedValue(mockVaultSession());
    await expect(completeAuthLogin(tokenCallback(id))).resolves.toBe('ok');
    expect(setSession).toHaveBeenCalledWith({
      access_token: 'access-aaa',
      refresh_token: 'refresh-bbb',
    });
    expect(invoke).toHaveBeenCalledWith('set_auth_session', expect.anything());
  });

  it('rejects a second complete with the same pass after a successful consume', async () => {
    const { id, invoke } = await startLogin();
    setSession.mockResolvedValue(mockVaultSession());
    await expect(completeAuthLogin(tokenCallback(id))).resolves.toBe('ok');
    setSession.mockClear();
    invoke.mockClear();
    await expect(completeAuthLogin(tokenCallback(id))).resolves.not.toBe('ok');
    expect(setSession).not.toHaveBeenCalled();
    expect(invoke).not.toHaveBeenCalledWith('set_auth_session', expect.anything());
  });

  it('rejects the first pass after a second start replaces the wait', async () => {
    const first = await startLogin();
    const second = await startLogin();
    setSession.mockResolvedValue(mockVaultSession());
    await expect(completeAuthLogin(tokenCallback(first.id))).resolves.not.toBe('ok');
    expect(setSession).not.toHaveBeenCalled();
    expect(second.invoke).not.toHaveBeenCalledWith('set_auth_session', expect.anything());
    await expect(completeAuthLogin(tokenCallback(second.id))).resolves.toBe('ok');
  });

  it('rejects a valid ticket when startAuthLogin was never called', async () => {
    const { invoke } = seedInvoke(null);
    setSession.mockResolvedValue(mockVaultSession());
    exchangeCodeForSession.mockResolvedValue(mockVaultSession());
    await expect(completeAuthLogin(tokenCallback('trace_neverstarted'))).resolves.not.toBe('ok');
    expect(setSession).not.toHaveBeenCalled();
    expect(exchangeCodeForSession).not.toHaveBeenCalled();
    expect(invoke).not.toHaveBeenCalledWith('set_auth_session', expect.anything());
  });

  it('does not consume in-flight when the callback pass mismatches', async () => {
    const { id, invoke } = await startLogin();
    setSession.mockResolvedValue(mockVaultSession());
    await expect(completeAuthLogin(tokenCallback('trace_otherpass01'))).resolves.not.toBe('ok');
    expect(setSession).not.toHaveBeenCalled();
    expect(exchangeCodeForSession).not.toHaveBeenCalled();
    expect(invoke).not.toHaveBeenCalledWith('set_auth_session', expect.anything());
    await expect(completeAuthLogin(tokenCallback(id))).resolves.toBe('ok');
    expect(setSession).toHaveBeenCalled();
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
    expect(src).toMatch(/AUTH_REDIRECT_TO|spark:\/\/auth-login\/callback/);
    expect(src).toMatch(/redirectTo = `\$\{AUTH_REDIRECT_TO\}\?/);
    expect(src).toMatch(/PASS_QUERY/);
    expect(src).toMatch(/encodePassBag/);
    expect(src).toMatch(/beginAuthLoginWait/);
    expect(src).toMatch(/consumeMatchingAuthLoginPass/);
    expect(src).not.toMatch(/linkIdentity/);
    expect(src).not.toMatch(/localStorage/);
    expect(src).not.toMatch(/@tauri-apps\//);
    expect(src).not.toMatch(/webview|Webview/i);
  });
});
