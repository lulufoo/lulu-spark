import { SignJWT, importPKCS8 } from "jose";
import { AccessTokenError } from "./errors";
import type { ServiceAccountKey } from "./service-account-key";

// Cloud Storage accepts narrower scopes, but this token is also the starting point for the narrower
// client tokens (see client-token-issuer.ts), and the service account's own role is already the minimum.
const SCOPE = "https://www.googleapis.com/auth/cloud-platform";
const GRANT_TYPE = "urn:ietf:params:oauth:grant-type:jwt-bearer";
const ASSERTION_LIFETIME_SECONDS = 3600;
const DEFAULT_MIN_VALIDITY_MS = 5 * 60 * 1000;

type SigningKey = Awaited<ReturnType<typeof importPKCS8>>;

export type AccessTokenProvider = {
  /**
   * A token that is still valid for at least `minValidityMs` (default five minutes). A token cut down
   * from this one by the Credential Access Boundary expires when this one does, so callers that hand
   * the result to someone else ask for as long as they want that someone to have.
   */
  getAccessToken(minValidityMs?: number): Promise<string>;
};

export type AccessTokenProviderOptions = {
  key: ServiceAccountKey;
  /** Injected by tests. */
  fetchFn?: typeof fetch;
  /** Milliseconds since the epoch. Injected by tests. */
  now?: () => number;
};

/**
 * Exchanges the long-lived key for short OAuth access tokens (JWT bearer grant).
 * A token is reused while it has enough life left for the caller, and concurrent callers share one request.
 */
export function createAccessTokenProvider(options: AccessTokenProviderOptions): AccessTokenProvider {
  const { key } = options;
  const fetchFn = options.fetchFn ?? ((input, init) => fetch(input, init));
  const now = options.now ?? Date.now;

  let signingKey: Promise<SigningKey> | undefined;
  let cached: { token: string; expiresAt: number } | undefined;
  let inFlight: Promise<string> | undefined;

  function loadSigningKey(): Promise<SigningKey> {
    signingKey ??= importPKCS8(key.privateKey, "RS256").catch((error) => {
      signingKey = undefined;
      throw error;
    });
    return signingKey;
  }

  async function requestToken(): Promise<string> {
    const issuedAt = Math.floor(now() / 1000);
    const assertion = await new SignJWT({ scope: SCOPE })
      .setProtectedHeader({ alg: "RS256", typ: "JWT" })
      .setIssuer(key.clientEmail)
      .setAudience(key.tokenUri)
      .setIssuedAt(issuedAt)
      .setExpirationTime(issuedAt + ASSERTION_LIFETIME_SECONDS)
      .sign(await loadSigningKey());

    const response = await fetchFn(key.tokenUri, {
      method: "POST",
      headers: { "Content-Type": "application/x-www-form-urlencoded" },
      body: new URLSearchParams({ grant_type: GRANT_TYPE, assertion }),
    });
    const body = (await response.json().catch(() => ({}))) as {
      access_token?: unknown;
      expires_in?: unknown;
      error?: unknown;
    };
    if (!response.ok) {
      const code = typeof body.error === "string" ? `: ${body.error}` : "";
      throw new AccessTokenError(`token endpoint answered HTTP ${response.status}${code}`);
    }
    if (typeof body.access_token !== "string" || typeof body.expires_in !== "number") {
      throw new AccessTokenError("token endpoint answered without a usable access token");
    }
    cached = { token: body.access_token, expiresAt: now() + body.expires_in * 1000 };
    return body.access_token;
  }

  return {
    async getAccessToken(minValidityMs = DEFAULT_MIN_VALIDITY_MS) {
      if (cached && cached.expiresAt - now() >= minValidityMs) return cached.token;
      inFlight ??= requestToken().finally(() => {
        inFlight = undefined;
      });
      return inFlight;
    },
  };
}
