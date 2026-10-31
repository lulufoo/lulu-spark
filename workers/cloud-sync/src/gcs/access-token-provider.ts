import { SignJWT, importPKCS8 } from "jose";
import { AccessTokenError } from "./errors";
import type { ServiceAccountKey } from "./service-account-key";

// Cloud Storage accepts narrower scopes, but this token is also the starting point for narrower
// tokens later (see the issue on short-lived client tokens), and the service account's own role
// is already the minimum.
const SCOPE = "https://www.googleapis.com/auth/cloud-platform";
const GRANT_TYPE = "urn:ietf:params:oauth:grant-type:jwt-bearer";
const ASSERTION_LIFETIME_SECONDS = 3600;
const REFRESH_MARGIN_MS = 5 * 60 * 1000;

type SigningKey = Awaited<ReturnType<typeof importPKCS8>>;

export type AccessTokenProvider = {
  getAccessToken(): Promise<string>;
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
 * A token is reused until five minutes before it expires, and concurrent callers share one request.
 */
export function createAccessTokenProvider(options: AccessTokenProviderOptions): AccessTokenProvider {
  const { key } = options;
  const fetchFn = options.fetchFn ?? ((input, init) => fetch(input, init));
  const now = options.now ?? Date.now;

  let signingKey: Promise<SigningKey> | undefined;
  let cached: { token: string; refreshAt: number } | undefined;
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
    cached = { token: body.access_token, refreshAt: now() + body.expires_in * 1000 - REFRESH_MARGIN_MS };
    return body.access_token;
  }

  return {
    async getAccessToken() {
      if (cached && now() < cached.refreshAt) return cached.token;
      inFlight ??= requestToken().finally(() => {
        inFlight = undefined;
      });
      return inFlight;
    },
  };
}
