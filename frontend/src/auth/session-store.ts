import { createClient, type SupabaseClient } from '@supabase/supabase-js';

export const SPARK_AUTH_URL = 'https://ysvsmsvfyzahevkximog.supabase.co';
export const SPARK_AUTH_ANON_KEY =
  'sb_publishable_XnDXtdkVsQxVKW8pov_-JQ_9HDRWJUT';
export const AUTH_REDIRECT_TO = 'https://localhost:7654/auth-login/landing';
export const AUTH_SCHEME_OPENED_EVENT = 'spark-scheme:opened';

export type VaultAuthUser = {
  id: string;
  email?: string | null;
  name?: string | null;
  avatar?: string | null;
  provider?: string | null;
};

export type VaultAuthSession = {
  access_token: string;
  refresh_token: string;
  expires_at: number;
  user: VaultAuthUser;
};

const memory = new Map<string, string>();
let client: SupabaseClient | null = null;

function getTauriInvoke() {
  if (typeof window === 'undefined') return null;
  const invoke =
    window.__TAURI__?.core?.invoke || window.__TAURI_INTERNALS__?.invoke;
  return typeof invoke === 'function' ? invoke : null;
}

function pickStr(value: unknown): string | null {
  return typeof value === 'string' && value ? value : null;
}

function asRecord(value: unknown): Record<string, unknown> | null {
  return value && typeof value === 'object' && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : null;
}

export function toVaultSession(raw: unknown): VaultAuthSession | null {
  const rec = asRecord(raw);
  if (!rec) return null;
  const access = pickStr(rec.access_token);
  const refresh = pickStr(rec.refresh_token);
  if (!access || !refresh) return null;
  const user = asRecord(rec.user) ?? {};
  const meta = asRecord(user.user_metadata) ?? {};
  const app = asRecord(user.app_metadata) ?? {};
  const expiresAt =
    typeof rec.expires_at === 'number' ? rec.expires_at : Number(rec.expires_at) || 0;
  return {
    access_token: access,
    refresh_token: refresh,
    expires_at: expiresAt,
    user: {
      id: pickStr(user.id) ?? '',
      email: pickStr(user.email),
      name: pickStr(user.name) ?? pickStr(meta.name) ?? pickStr(meta.full_name),
      avatar:
        pickStr(user.avatar) ?? pickStr(meta.avatar_url) ?? pickStr(meta.picture),
      provider: pickStr(user.provider) ?? pickStr(app.provider),
    },
  };
}

export function vaultAuthStorage() {
  return {
    async getItem(key: string): Promise<string | null> {
      if (memory.has(key)) return memory.get(key) ?? null;
      const invoke = getTauriInvoke();
      if (!invoke) return null;
      const session = await invoke('get_auth_session');
      if (!session) return null;
      const raw = JSON.stringify(session);
      memory.set(key, raw);
      return raw;
    },
    async setItem(key: string, value: string): Promise<void> {
      let parsed: unknown = value;
      try {
        parsed = JSON.parse(value);
      } catch {
        memory.set(key, value);
        return;
      }
      const session = toVaultSession(parsed);
      if (!session) {
        memory.set(key, value);
        return;
      }
      const invoke = getTauriInvoke();
      if (!invoke) return;
      await invoke('set_auth_session', { session });
      memory.set(key, JSON.stringify(session));
    },
    async removeItem(key: string): Promise<void> {
      memory.delete(key);
      const invoke = getTauriInvoke();
      if (!invoke) return;
      await invoke('delete_auth_session');
    },
  };
}

export function createSparkAuthClient(): SupabaseClient {
  return createClient(SPARK_AUTH_URL, SPARK_AUTH_ANON_KEY, {
    auth: {
      persistSession: false,
      autoRefreshToken: false,
      detectSessionInUrl: false,
      storage: vaultAuthStorage(),
    },
  });
}

export function getSparkAuthClient(): SupabaseClient {
  if (!client) client = createSparkAuthClient();
  return client;
}
