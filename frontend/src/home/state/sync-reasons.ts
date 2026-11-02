import type { AuthRefreshOutcome } from '../../auth/refresh.ts';

/** The one line for a round that could not start because the sign-in cannot be made fresh. */
export function loginRefusalReason(session: Exclude<AuthRefreshOutcome, { kind: 'ok' }>): string {
  if (session.kind === 'signed_out') return 'Sign in first to sync.';
  return session.reason === 'network'
    ? 'Could not reach the sign-in service. Check your connection and try again.'
    : 'Your sign-in is no longer valid. Sign in again to sync.';
}

/**
 * The one line for a round the host did not make. `status` is the one the host's answer carried,
 * if it carried one; `message` is the host's own words, which say what is wrong with the cloud or
 * the local database.
 */
export function hostRefusalReason(status: number | null, message: string): string {
  if (status === 401) return 'Your sign-in was not accepted. Sign in again to sync.';
  if (status === 409) return 'A sync is already running.';
  return message;
}
