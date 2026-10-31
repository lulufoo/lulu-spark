import { InvalidServiceAccountKeyError } from "./errors";

export type ServiceAccountKey = {
  clientEmail: string;
  privateKey: string;
  tokenUri: string;
};

const DEFAULT_TOKEN_URI = "https://oauth2.googleapis.com/token";

/** Reads the downloaded key file (JSON) kept in the `GCS_SERVICE_ACCOUNT_KEY` secret. */
export function parseServiceAccountKey(json: string | undefined): ServiceAccountKey {
  let parsed: unknown;
  try {
    parsed = JSON.parse(json ?? "");
  } catch {
    // Not echoing the parse error: it can quote part of the secret.
    throw new InvalidServiceAccountKeyError("service account key is not valid JSON");
  }
  const fields = (parsed ?? {}) as Record<string, unknown>;
  const { client_email: clientEmail, private_key: privateKey, token_uri: tokenUri } = fields;
  if (typeof clientEmail !== "string" || !clientEmail) {
    throw new InvalidServiceAccountKeyError("service account key has no client_email");
  }
  if (typeof privateKey !== "string" || !privateKey.includes("BEGIN PRIVATE KEY")) {
    throw new InvalidServiceAccountKeyError("service account key has no PKCS#8 private_key");
  }
  return {
    clientEmail,
    privateKey,
    tokenUri: typeof tokenUri === "string" && tokenUri ? tokenUri : DEFAULT_TOKEN_URI,
  };
}
