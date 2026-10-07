// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const PROJECT_URL = 'https://ysvsmsvfyzahevkximog.supabase.co';
const PUBLISHABLE_KEY = 'sb_publishable_XnDXtdkVsQxVKW8pov_-JQ_9HDRWJUT';
const ANON_JWT =
  'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZSIsInJlZiI6InlzdnNtc3ZmeXphaGV2a3hpbW9nIiwicm9sZSI6ImFub24iLCJpYXQiOjE3OTEyNzQ1NTQsImV4cCI6MjEwNjg1MDU1NH0.vq-m8pESSsSdnWyXEf3qsV4jyTE1XjoRN34n1EdWH4Y';

const createClient = vi.fn(() => ({ auth: {} }));

vi.mock('@supabase/supabase-js', () => ({
  createClient: (...args) => createClient(...args),
}));

const { AUTH_REDIRECT_TO, createSparkAuthClient, vaultAuthStorage } = await import(
  '../../frontend/src/auth/session-store.ts'
);

function readRel(rel) {
  return readFileSync(join(repoRoot, rel), 'utf8');
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
  const invoke = vi.fn(async (cmd, args) => {
    if (cmd === 'get_auth_session') return session;
    if (cmd === 'set_auth_session') return null;
    if (cmd === 'delete_auth_session') return null;
    throw new Error(`unexpected invoke ${cmd} ${JSON.stringify(args)}`);
  });
  window.__TAURI__ = { core: { invoke } };
  return invoke;
}

beforeEach(() => {
  createClient.mockClear();
  installLocalStorageMock();
});

afterEach(() => {
  delete window.__TAURI__;
  window.localStorage.clear();
  vi.restoreAllMocks();
});

describe('createSparkAuthClient', () => {
  it('uses the t1 hosted URL and publishable/anon key', () => {
    createSparkAuthClient();
    expect(createClient).toHaveBeenCalled();
    const [url, key] = createClient.mock.calls[0];
    expect(url).toBe(PROJECT_URL);
    expect([PUBLISHABLE_KEY, ANON_JWT]).toContain(key);
    expect(key).not.toMatch(/service_role/);
  });

  it('turns off default persistSession/localStorage and uses custom storage', () => {
    createSparkAuthClient();
    const options = createClient.mock.calls[0][2];
    expect(options.auth.persistSession).toBe(false);
    expect(options.auth.storage).toBeTruthy();
    expect(options.auth.storage).not.toBe(window.localStorage);
    expect(options.auth.detectSessionInUrl).toBe(false);
  });
});

describe('vaultAuthStorage', () => {
  it('getItem/setItem/removeItem go through get/set/delete_auth_session', async () => {
    const stored = {
      access_token: 'access-aaa',
      refresh_token: 'refresh-bbb',
      expires_at: 1_800_000_000,
      user: {
        id: 'user-42',
        email: 'ada@example.com',
        name: 'Ada',
        avatar: 'https://example.com/a.png',
        provider: 'google',
      },
    };
    const invoke = seedInvoke(stored);
    const storage = vaultAuthStorage();

    const raw = await storage.getItem('sb-auth-token');
    expect(invoke).toHaveBeenCalledWith('get_auth_session');
    expect(raw).toBeTruthy();
    const parsed = JSON.parse(raw);
    expect(parsed.access_token).toBe('access-aaa');
    expect(parsed.user.id).toBe('user-42');

    await storage.setItem(
      'sb-auth-token',
      JSON.stringify({
        ...stored,
        provider_token: 'google-access-should-not-persist',
        provider_refresh_token: 'google-refresh-should-not-persist',
      }),
    );
    expect(invoke).toHaveBeenCalledWith(
      'set_auth_session',
      expect.objectContaining({
        session: expect.objectContaining({
          access_token: 'access-aaa',
          refresh_token: 'refresh-bbb',
          expires_at: 1_800_000_000,
          user: expect.objectContaining({
            id: 'user-42',
            email: 'ada@example.com',
            name: 'Ada',
            avatar: 'https://example.com/a.png',
            provider: 'google',
          }),
        }),
      }),
    );
    const setArgs = invoke.mock.calls.find((call) => call[0] === 'set_auth_session')[1];
    expect(JSON.stringify(setArgs)).not.toContain('provider_token');
    expect(JSON.stringify(setArgs)).not.toContain('google-access-should-not-persist');

    await storage.removeItem('sb-auth-token');
    expect(invoke).toHaveBeenCalledWith('delete_auth_session');
    expect(window.localStorage.length).toBe(0);
  });

  it('does not write a Session into localStorage', async () => {
    const invoke = seedInvoke(null);
    const storage = vaultAuthStorage();
    await storage.setItem(
      'sb-auth-token',
      JSON.stringify({
        access_token: 'access-aaa',
        refresh_token: 'refresh-bbb',
        expires_at: 1_800_000_000,
        user: { id: 'user-42' },
      }),
    );
    expect(invoke).toHaveBeenCalledWith('set_auth_session', expect.anything());
    expect(window.localStorage.getItem('sb-auth-token')).toBeNull();
    expect(window.localStorage.length).toBe(0);
  });
});

describe('session-store source contract', () => {
  it('points AUTH_REDIRECT_TO at the Gateway landing URL', () => {
    expect(AUTH_REDIRECT_TO).toBe('https://localhost:7654/auth-login/landing');
    const src = readRel('frontend/src/auth/session-store.ts');
    expect(src).toContain("export const AUTH_REDIRECT_TO = 'https://localhost:7654/auth-login/landing'");
    expect(src).not.toMatch(/spark:\/\/auth-login\/callback/);
  });

  it('does not call linkIdentity or keep provider tokens', () => {
    const src = readRel('frontend/src/auth/session-store.ts');
    expect(src).toMatch(/createClient/);
    expect(src).toMatch(/persistSession:\s*false/);
    expect(src).toContain(PROJECT_URL);
    expect(src).not.toMatch(/linkIdentity/);
    expect(src).not.toMatch(/localStorage/);
    expect(src).not.toMatch(/service_role/);
  });

  it('lists official supabase-js in package.json', () => {
    const pkg = JSON.parse(readRel('package.json'));
    expect(pkg.dependencies['@supabase/supabase-js']).toBeTruthy();
  });
});
