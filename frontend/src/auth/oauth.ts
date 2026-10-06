import {
  getSparkAuthClient,
  toVaultSession,
  type VaultAuthSession,
} from './session-store.ts';

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
  const { data, error } = await getSparkAuthClient().auth.signInWithOAuth({
    provider,
    options: {
      skipBrowserRedirect: true,
      redirectTo: 'spark://auth-login/callback',
    },
  });
  if (error || !data.url) {
    throw error ?? new Error('signInWithOAuth returned no url');
  }
  const openUrl = getTauriOpener();
  if (!openUrl) {
    throw new Error('Tauri opener unavailable');
  }
  await openUrl(data.url);
}

function readAuthCallback(url: string): { code: string } | null {
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
  const error = parsed.searchParams.get('error');
  const code = parsed.searchParams.get('code');
  if (error || !code) return null;
  return { code };
}

export async function completeAuthLogin(url: string): Promise<boolean> {
  const parsed = readAuthCallback(url);
  if (!parsed) return false;
  const invoke = getTauriInvoke();
  if (!invoke) return false;
  try {
    const { data, error } = await getSparkAuthClient().auth.exchangeCodeForSession(
      parsed.code,
    );
    const session = toVaultSession(data?.session);
    if (error || !session) return false;
    await invoke('set_auth_session', { session });
    return true;
  } catch {
    return false;
  }
}

export async function signOutAuth(): Promise<void> {
  await getSparkAuthClient().auth.signOut();
  const invoke = getTauriInvoke();
  if (invoke) await invoke('delete_auth_session');
}

export async function getAuthUser(): Promise<AuthUserView | null> {
  const invoke = getTauriInvoke();
  if (!invoke) return null;
  const raw = await invoke('get_auth_session');
  const session = toVaultSession(raw);
  return session ? toUserView(session) : null;
}
