// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

// No part of the sign-in library is replaced here: only the network (`fetch`) and the vault
// (`invoke`) are fake. This is what shows that `refresh.ts` works with the client as it is
// configured in `session-store.ts`.
const { refreshAuthSession } = await import('../../frontend/src/auth/refresh.ts');
const { createSparkAuthClient } = await import('../../frontend/src/auth/session-store.ts');

function nowSeconds() {
  return Math.floor(Date.now() / 1000);
}

let vault;
let invoke;
let fetchMock;

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

function nearlyExpired() {
  return {
    access_token: 'old-access',
    refresh_token: 'old-refresh',
    expires_at: nowSeconds() + 30,
    user: { id: 'user-1', email: 'a@example.com' },
  };
}

function json(status, body) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'Content-Type': 'application/json' },
  });
}

beforeEach(() => {
  fetchMock = vi.fn();
  globalThis.fetch = fetchMock;
});

afterEach(() => {
  delete window.__TAURI__;
  vi.restoreAllMocks();
});

describe('refreshAuthSession · on the real sign-in library', () => {
  it('trades the vault refresh token for a new session and stores it in the vault', async () => {
    useVault(nearlyExpired());
    fetchMock.mockResolvedValue(
      json(200, {
        access_token: 'new-access',
        refresh_token: 'new-refresh',
        token_type: 'bearer',
        expires_in: 3600,
        user: { id: 'user-1', email: 'a@example.com', app_metadata: {}, user_metadata: {} },
      }),
    );

    expect(await refreshAuthSession()).toEqual({ kind: 'ok' });

    expect(fetchMock).toHaveBeenCalledTimes(1);
    const [url, init] = fetchMock.mock.calls[0];
    expect(String(url)).toContain('/auth/v1/token?grant_type=refresh_token');
    expect(JSON.parse(init.body)).toEqual({ refresh_token: 'old-refresh' });
    expect(vault).toMatchObject({ access_token: 'new-access', refresh_token: 'new-refresh' });
    expect(vault.expires_at).toBeGreaterThan(nowSeconds() + 3500);
  });

  it('asks the service for nothing while the session in the vault is fresh', async () => {
    useVault({ ...nearlyExpired(), expires_at: nowSeconds() + 3600 });

    expect(await refreshAuthSession()).toEqual({ kind: 'ok' });
    expect(fetchMock).not.toHaveBeenCalled();
    expect(vault.access_token).toBe('old-access');
  });

  it('says invalid when the service refuses the refresh token, and leaves the vault alone', async () => {
    useVault(nearlyExpired());
    fetchMock.mockResolvedValue(
      json(400, { error: 'invalid_grant', error_description: 'Invalid Refresh Token: Already Used' }),
    );

    expect(await refreshAuthSession()).toEqual({ kind: 'failed', reason: 'invalid' });
    expect(vault.access_token).toBe('old-access');
    expect(invoke.mock.calls.some(([cmd]) => cmd === 'set_auth_session')).toBe(false);
  });

  it('says why `refresh.ts` reads the vault itself: the client does not', async () => {
    useVault({ ...nearlyExpired(), expires_at: nowSeconds() + 3600 });

    const { data } = await createSparkAuthClient().auth.getSession();

    // A new client, because the shared one keeps what an earlier refresh gave it.
    // `persistSession` is off, so the client has its own memory and never looks in the vault.
    // If this starts to fail, the client reads the vault and `refresh.ts` can be simpler.
    expect(data.session).toBeNull();
    expect(invoke).not.toHaveBeenCalled();
  });
});
