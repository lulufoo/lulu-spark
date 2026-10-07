import { LOGIN_EVENT_START, logLoginHop } from './hop.ts';
import { encodePassBag, newLoginTraceId, PASS_QUERY } from './pass.ts';
import { authUserStore } from './state/user.ts';
import {
  AUTH_REDIRECT_TO,
  getSparkAuthClient,
  toVaultSession,
  type VaultAuthSession,
} from './session-store.ts';

export type AuthExchangeOutcome = 'ok' | 'fail' | 'no_token';

export type AuthUserView = {
  user_id: string;
  email: string | null;
  display_name: string | null;
  avatar_url: string | null;
  provider: string | null;
};

function getTauriInvoke() {
  if (typeof window === 'undefined') return null;
  const invoke =
    window.__TAURI__?.core?.invoke || window.__TAURI_INTERNALS__?.invoke;
  return typeof invoke === 'function' ? invoke : null;
}

function getTauriOpener() {
  if (typeof window === 'undefined') return null;
  const opener = window.__TAURI__?.opener;
  const openUrl = opener?.openUrl;
  return typeof openUrl === 'function' ? openUrl.bind(opener) : null;
}

function toUserView(session: VaultAuthSession): AuthUserView {
  return {
    user_id: session.user.id,
    email: session.user.email ?? null,
    display_name: session.user.name ?? null,
    avatar_url: session.user.avatar ?? null,
    provider: session.user.provider ?? null,
  };
}

export async function startAuthLogin(provider: 'google' | 'github'): Promise<void> {
  const id = newLoginTraceId();
  const redirectTo = `${AUTH_REDIRECT_TO}?${PASS_QUERY}=${encodePassBag(id)}`;
  logLoginHop(LOGIN_EVENT_START, id, { provider });
  const { data, error } = await getSparkAuthClient().auth.signInWithOAuth({
    provider,
    options: {
      skipBrowserRedirect: true,
      redirectTo,
    },
  });
  if (error || !data.url) {
    logLoginHop(LOGIN_EVENT_START, id, { provider, outcome: 'open_fail' });
    throw error ?? new Error('signInWithOAuth returned no url');
  }
  const openUrl = getTauriOpener();
  if (!openUrl) {
    logLoginHop(LOGIN_EVENT_START, id, { provider, outcome: 'open_fail' });
    throw new Error('Tauri opener unavailable');
  }
  try {
    await openUrl(data.url);
  } catch (err) {
    logLoginHop(LOGIN_EVENT_START, id, { provider, outcome: 'open_fail' });
    throw err;
  }
}

function parseAuthCallbackUrl(url: string): URL | null {
  let parsed: URL;
  try {
    parsed = new URL(url);
  } catch {
    return null;
  }
  const path = parsed.pathname.replace(/\/+$/, '');
  if (parsed.protocol !== 'spark:' || parsed.hostname !== 'auth-login' || path !== '/callback') {
    return null;
  }
  return parsed;
}

function hashParams(parsed: URL): URLSearchParams {
  const raw = parsed.hash.startsWith('#') ? parsed.hash.slice(1) : parsed.hash;
  return new URLSearchParams(raw);
}

function callbackError(parsed: URL): string | null {
  return parsed.searchParams.get('error') || hashParams(parsed).get('error');
}

function callbackCode(parsed: URL): string | null {
  return parsed.searchParams.get('code');
}

function callbackTokens(parsed: URL): { access_token: string; refresh_token: string } | null {
  const hash = hashParams(parsed);
  const access = hash.get('access_token');
  const refresh = hash.get('refresh_token');
  if (!access || !refresh) return null;
  return { access_token: access, refresh_token: refresh };
}

async function persistVaultSession(raw: unknown): Promise<boolean> {
  const session = toVaultSession(raw);
  if (!session) return false;
  const invoke = getTauriInvoke();
  if (!invoke) return false;
  await invoke('set_auth_session', { session });
  authUserStore.set(toUserView(session));
  return true;
}

export async function completeAuthLogin(url: string): Promise<AuthExchangeOutcome> {
  const parsed = parseAuthCallbackUrl(url);
  if (!parsed || callbackError(parsed)) return 'fail';
  const invoke = getTauriInvoke();
  if (!invoke) return 'fail';
  try {
    const code = callbackCode(parsed);
    if (code) {
      const { data, error } = await getSparkAuthClient().auth.exchangeCodeForSession(code);
      if (error) return 'fail';
      return (await persistVaultSession(data?.session)) ? 'ok' : 'fail';
    }
    const tokens = callbackTokens(parsed);
    if (!tokens) return 'no_token';
    const { data, error } = await getSparkAuthClient().auth.setSession(tokens);
    if (error) return 'fail';
    return (await persistVaultSession(data?.session)) ? 'ok' : 'fail';
  } catch {
    return 'fail';
  }
}

export async function signOutAuth(): Promise<void> {
  await getSparkAuthClient().auth.signOut();
  const invoke = getTauriInvoke();
  if (invoke) await invoke('delete_auth_session');
  authUserStore.set(null);
}

export async function getAuthUser(): Promise<AuthUserView | null> {
  const invoke = getTauriInvoke();
  if (!invoke) return null;
  const raw = await invoke('get_auth_session');
  const session = toVaultSession(raw);
  return session ? toUserView(session) : null;
}
