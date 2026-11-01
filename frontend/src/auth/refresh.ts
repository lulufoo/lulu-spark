import { persistVaultSession, readVaultSession } from './oauth.ts';
import { getSparkAuthClient } from './session-store.ts';

/**
 * How far the sign-in session is from being usable by Rust, which reads it from the vault and
 * never refreshes it.
 *
 * - `signed_out`: the vault holds no session.
 * - `ok`: the session in the vault is fresh enough; if it was about to expire it has been
 *   refreshed and the new one is in the vault.
 * - `failed`: there is a session but it could not be made fresh. `network` means the sign-in
 *   service could not be reached (trying again later may work); `invalid` means it refused the
 *   session or the new one could not be stored (the user has to sign in again).
 */
export type AuthRefreshOutcome =
  | { kind: 'signed_out' }
  | { kind: 'ok' }
  | { kind: 'failed'; reason: 'network' | 'invalid' };

/** A refresh may take the network; a call that hangs is told apart from one that was refused. */
export const REFRESH_TIMEOUT_MS = 10_000;

/**
 * A sync or a delete can run for minutes and the cloud checks the session on every call, so a
 * session with less than this left is refreshed first.
 */
export const REFRESH_MARGIN_SECONDS = 600;

let inFlight: Promise<AuthRefreshOutcome> | null = null;

/**
 * Make the session in the vault good for at least `REFRESH_MARGIN_SECONDS`. Call this before
 * anything that sends the session to the cloud.
 *
 * The sign-in client keeps its session in memory only, so it cannot be asked for "the session":
 * the vault is read here, and a session that is about to expire is refreshed with its refresh
 * token and written back to the vault. Calls that overlap share one refresh, because the refresh
 * token is used up by it.
 */
export function refreshAuthSession(): Promise<AuthRefreshOutcome> {
  inFlight ??= refresh().finally(() => {
    inFlight = null;
  });
  return inFlight;
}

async function refresh(): Promise<AuthRefreshOutcome> {
  const session = await readVaultSession();
  if (!session) return { kind: 'signed_out' };
  if (session.expires_at - REFRESH_MARGIN_SECONDS > Date.now() / 1000) return { kind: 'ok' };
  return withTimeout(renew(session.refresh_token));
}

async function renew(refreshToken: string): Promise<AuthRefreshOutcome> {
  let renewed;
  try {
    renewed = await getSparkAuthClient().auth.refreshSession({ refresh_token: refreshToken });
  } catch {
    return { kind: 'failed', reason: 'network' };
  }
  const { data, error } = renewed;
  if (error) return { kind: 'failed', reason: isNetworkError(error) ? 'network' : 'invalid' };
  if (!data?.session) return { kind: 'failed', reason: 'invalid' };
  return (await stored(data.session)) ? { kind: 'ok' } : { kind: 'failed', reason: 'invalid' };
}

/** A refreshed session that is not in the vault is of no use to Rust, and has used up the old token. */
async function stored(session: unknown): Promise<boolean> {
  try {
    return await persistVaultSession(session);
  } catch {
    return false;
  }
}

async function withTimeout(asked: Promise<AuthRefreshOutcome>): Promise<AuthRefreshOutcome> {
  let timer: ReturnType<typeof setTimeout> | undefined;
  const gaveUp = new Promise<AuthRefreshOutcome>((resolve) => {
    timer = setTimeout(() => resolve({ kind: 'failed', reason: 'network' }), REFRESH_TIMEOUT_MS);
  });
  try {
    return await Promise.race([asked, gaveUp]);
  } finally {
    clearTimeout(timer);
  }
}

/** The sign-in library names the error of a request that never got an answer. */
function isNetworkError(error: unknown): boolean {
  return (error as { name?: unknown } | null)?.name === 'AuthRetryableFetchError';
}
