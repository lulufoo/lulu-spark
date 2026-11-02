import { TokenExchangeError } from "./errors";

const STS_URL = "https://sts.googleapis.com/v1/token";
const GRANT_TYPE = "urn:ietf:params:oauth:grant-type:token-exchange";
const ACCESS_TOKEN_TYPE = "urn:ietf:params:oauth:token-type:access_token";

// Read and list objects, nothing else (LW-711). The role also lists folder permissions, but the
// boundary can only narrow what the long-lived account has, and that account has none of them.
const READ_ONLY_ROLE = "inRole:roles/storage.objectViewer";

/**
 * A narrowed token expires when the token it was made from expires, and Google offers no way to ask
 * for a shorter or longer one. The source token is therefore chosen so that the client always gets
 * at least this long, out of the hour a fresh token lives.
 */
export const CLIENT_TOKEN_MIN_VALIDITY_MS = 30 * 60 * 1000;

const BUCKET_NAME_PATTERN = /^[a-z0-9][a-z0-9._-]{1,61}[a-z0-9]$/;

export type ClientToken = {
  accessToken: string;
  /** Seconds since the epoch. Taken from before the request was sent, so it is never too late. */
  expiresAt: number;
};

export type ClientTokenIssuer = {
  /** A token that can read and list objects in `bucket` and do nothing else, anywhere. */
  issueReadOnlyToken(bucket: string): Promise<ClientToken>;
};

export type ClientTokenIssuerOptions = {
  /** The long-lived account's access token, valid for at least the given number of milliseconds. */
  getAccessToken(minValidityMs: number): Promise<string>;
  /** Injected by tests. */
  fetchFn?: typeof fetch;
  /** Milliseconds since the epoch. Injected by tests. */
  now?: () => number;
};

/** Narrows the long-lived account's access token with a Credential Access Boundary of one rule. */
export function createClientTokenIssuer(options: ClientTokenIssuerOptions): ClientTokenIssuer {
  const fetchFn = options.fetchFn ?? ((input, init) => fetch(input, init));
  const now = options.now ?? Date.now;

  return {
    async issueReadOnlyToken(bucket) {
      // The name comes from our own HMAC, but it ends up inside a resource path that decides what
      // the token may touch, so a malformed one is refused instead of trusted.
      if (!BUCKET_NAME_PATTERN.test(bucket)) {
        throw new TokenExchangeError("refusing to build an access boundary for an invalid bucket name");
      }
      const sourceToken = await options.getAccessToken(CLIENT_TOKEN_MIN_VALIDITY_MS);
      const boundary = {
        accessBoundary: {
          accessBoundaryRules: [
            {
              availableResource: `//storage.googleapis.com/projects/_/buckets/${bucket}`,
              availablePermissions: [READ_ONLY_ROLE],
            },
          ],
        },
      };

      const requestedAtSeconds = Math.floor(now() / 1000);
      const response = await fetchFn(STS_URL, {
        method: "POST",
        headers: { "Content-Type": "application/x-www-form-urlencoded" },
        body: new URLSearchParams({
          grant_type: GRANT_TYPE,
          requested_token_type: ACCESS_TOKEN_TYPE,
          subject_token_type: ACCESS_TOKEN_TYPE,
          subject_token: sourceToken,
          // Google wants the JSON encoded once on its own, on top of the form encoding.
          options: encodeURIComponent(JSON.stringify(boundary)),
        }),
      });
      const body = (await response.json().catch(() => ({}))) as {
        access_token?: unknown;
        expires_in?: unknown;
        error?: unknown;
      };
      if (!response.ok) {
        const code = typeof body.error === "string" ? `: ${body.error}` : "";
        throw new TokenExchangeError(`token exchange answered HTTP ${response.status}${code}`);
      }
      if (typeof body.access_token !== "string" || typeof body.expires_in !== "number" || body.expires_in <= 0) {
        throw new TokenExchangeError("token exchange answered without a usable access token");
      }
      return { accessToken: body.access_token, expiresAt: requestedAtSeconds + body.expires_in };
    },
  };
}
