// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

// The sign-in client is replaced here; the vault is the real `oauth.ts` reading and writing
// through a fake `invoke`. `refresh-with-auth-js.test.js` runs the same code on the real library.
const client = vi.hoisted(() => ({ auth: { refreshSession: vi.fn() } }));

vi.mock('../../frontend/src/auth/session-store.ts', async (importOriginal) => ({
  ...(await importOriginal()),
  getSparkAuthClient: () => client,
}));

const { REFRESH_MARGIN_SECONDS, REFRESH_TIMEOUT_MS, refreshAuthSession } = await import(
  '../../frontend/src/auth/refresh.ts'
);
const { authUserStore } = await import('../../frontend/src/auth/state/user.ts');

const NOW_SECONDS = 1_800_000_000;

function vaultSession(expiresIn, overrides = {}) {
  return {
    access_token: 'vault-access',
    refresh_token: 'vault-refresh',
    expires_at: NOW_SECONDS + expiresIn,
    user: { id: 'user-1', email: 'a@example.com' },
    ...overrides,
  };
}

function renewed(overrides = {}) {
  return {
    data: {
      session: {
        access_token: 'new-access',
        refresh_token: 'new-refresh',
        expires_at: NOW_SECONDS + 3600,
        user: { id: 'user-1', email: 'a@example.com' },
      },
    },
    error: null,
    ...overrides,
  };
}

function failure(name) {
  const error = new Error(name);
  error.name = name;
  return { data: { session: null }, error };
}

let vault;
let invoke;

function useVault(session) {
  vault = session;
  invoke = vi.fn(async (cmd, args) => {
    if (cmd === 'get_auth_session') return vault;
    if (cmd === 'set_auth_session') {
      vault = args.session;
      return null;
    }
    throw new Error(`unexpected invoke ${cmd}`);
  });
  window.__TAURI__ = { core: { invoke } };
}

function writes() {
  return invoke.mock.calls.filter(([cmd]) => cmd === 'set_auth_session');
}

beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(NOW_SECONDS * 1000);
  client.auth.refreshSession.mockReset();
  authUserStore.set(null);
});

afterEach(() => {
  vi.useRealTimers();
  delete window.__TAURI__;
});

describe('refreshAuthSession · what is in the vault decides', () => {
  const cases = [
    ['no session in the vault', null, { kind: 'signed_out' }, false],
    ['a session with an hour left', vaultSession(3600), { kind: 'ok' }, false],
    ['one second more than the margin left', vaultSession(REFRESH_MARGIN_SECONDS + 1), { kind: 'ok' }, false],
    ['exactly the margin left', vaultSession(REFRESH_MARGIN_SECONDS), { kind: 'ok' }, true],
    ['a minute left', vaultSession(60), { kind: 'ok' }, true],
    ['already expired', vaultSession(-5), { kind: 'ok' }, true],
    ['no expiry on record', vaultSession(0, { expires_at: undefined }), { kind: 'ok' }, true],
  ];

  it.each(cases)('%s', async (_name, session, expected, refreshes) => {
    useVault(session);
    client.auth.refreshSession.mockResolvedValue(renewed());

    expect(await refreshAuthSession()).toEqual(expected);
    expect(client.auth.refreshSession).toHaveBeenCalledTimes(refreshes ? 1 : 0);
    expect(writes()).toHaveLength(refreshes ? 1 : 0);
  });
});

describe('refreshAuthSession · refreshing', () => {
  it('uses the refresh token from the vault and writes the new session back to it', async () => {
    useVault(vaultSession(30));
    client.auth.refreshSession.mockResolvedValue(renewed());

    expect(await refreshAuthSession()).toEqual({ kind: 'ok' });

    expect(client.auth.refreshSession).toHaveBeenCalledWith({ refresh_token: 'vault-refresh' });
    expect(writes()).toHaveLength(1);
    expect(vault).toMatchObject({
      access_token: 'new-access',
      refresh_token: 'new-refresh',
      expires_at: NOW_SECONDS + 3600,
    });
  });

  it('keeps the user shown in the app in step with the new session', async () => {
    useVault(vaultSession(30));
    client.auth.refreshSession.mockResolvedValue(renewed());

    await refreshAuthSession();

    expect(authUserStore.getSnapshot()).toMatchObject({ user_id: 'user-1', email: 'a@example.com' });
  });

  const failures = [
    ['the service cannot be reached', () => failure('AuthRetryableFetchError'), 'network'],
    ['the service refuses the refresh token', () => failure('AuthApiError'), 'invalid'],
    ['the error has no name', () => ({ data: { session: null }, error: {} }), 'invalid'],
    ['it answers with no session and no error', () => ({ data: { session: null }, error: null }), 'invalid'],
    ['it answers with nothing', () => ({ data: null, error: null }), 'invalid'],
  ];

  it.each(failures)('%s', async (_name, answer, reason) => {
    useVault(vaultSession(30));
    client.auth.refreshSession.mockResolvedValue(answer());

    expect(await refreshAuthSession()).toEqual({ kind: 'failed', reason });
    expect(writes()).toHaveLength(0);
    expect(vault.access_token).toBe('vault-access');
  });

  it('counts a call that throws as the service being out of reach', async () => {
    useVault(vaultSession(30));
    client.auth.refreshSession.mockRejectedValue(new TypeError('Failed to fetch'));

    expect(await refreshAuthSession()).toEqual({ kind: 'failed', reason: 'network' });
    expect(writes()).toHaveLength(0);
  });

  it('says invalid when the new session cannot be stored, so the user signs in again', async () => {
    useVault(vaultSession(30));
    client.auth.refreshSession.mockResolvedValue(renewed());
    invoke.mockImplementation(async (cmd) => {
      if (cmd === 'get_auth_session') return vault;
      throw new Error('vault is locked');
    });

    expect(await refreshAuthSession()).toEqual({ kind: 'failed', reason: 'invalid' });
  });

  it('says invalid when the new session is not one the vault can hold', async () => {
    useVault(vaultSession(30));
    client.auth.refreshSession.mockResolvedValue(
      renewed({ data: { session: { access_token: '', refresh_token: '' } } }),
    );

    expect(await refreshAuthSession()).toEqual({ kind: 'failed', reason: 'invalid' });
    expect(writes()).toHaveLength(0);
  });
});

describe('refreshAuthSession · waiting', () => {
  it('gives up on a refresh that hangs, after the timeout and not before', async () => {
    useVault(vaultSession(30));
    client.auth.refreshSession.mockReturnValue(new Promise(() => {}));

    let outcome = null;
    const asked = refreshAuthSession().then((value) => {
      outcome = value;
    });

    await vi.advanceTimersByTimeAsync(REFRESH_TIMEOUT_MS - 1);
    expect(outcome).toBeNull();
    await vi.advanceTimersByTimeAsync(1);
    await asked;
    expect(outcome).toEqual({ kind: 'failed', reason: 'network' });
  });

  it('leaves no timer behind once it has an answer', async () => {
    useVault(vaultSession(30));
    client.auth.refreshSession.mockResolvedValue(renewed());

    await refreshAuthSession();

    expect(vi.getTimerCount()).toBe(0);
  });

  it('starts no timer when no refresh is needed', async () => {
    useVault(vaultSession(3600));
    await refreshAuthSession();
    expect(vi.getTimerCount()).toBe(0);
  });
});

describe('refreshAuthSession · overlapping calls', () => {
  it('shares one refresh, because the refresh token is used up by it', async () => {
    useVault(vaultSession(30));
    let release;
    client.auth.refreshSession.mockReturnValue(new Promise((resolve) => { release = resolve; }));

    const first = refreshAuthSession();
    const second = refreshAuthSession();
    await vi.advanceTimersByTimeAsync(0);
    release(renewed());

    expect(await Promise.all([first, second])).toEqual([{ kind: 'ok' }, { kind: 'ok' }]);
    expect(client.auth.refreshSession).toHaveBeenCalledTimes(1);
    expect(writes()).toHaveLength(1);
  });

  it('starts a new refresh once the last one is over, whether it worked or not', async () => {
    useVault(vaultSession(30));
    client.auth.refreshSession.mockResolvedValueOnce(failure('AuthRetryableFetchError'));
    expect(await refreshAuthSession()).toEqual({ kind: 'failed', reason: 'network' });

    client.auth.refreshSession.mockResolvedValueOnce(renewed());
    expect(await refreshAuthSession()).toEqual({ kind: 'ok' });

    expect(client.auth.refreshSession).toHaveBeenCalledTimes(2);
  });

  it('passes on a vault that cannot be read, and the next call is not stuck behind it', async () => {
    useVault(vaultSession(3600));
    invoke.mockRejectedValueOnce(new Error('vault is locked'));

    await expect(refreshAuthSession()).rejects.toThrow('vault is locked');
    expect(await refreshAuthSession()).toEqual({ kind: 'ok' });
  });
});
