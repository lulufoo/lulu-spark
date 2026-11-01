// Error messages here end up in logs. They carry statuses and Google's error codes, never keys or tokens.

/** The service account key secret is missing or malformed. */
export class InvalidServiceAccountKeyError extends Error {
  override name = "InvalidServiceAccountKeyError";
}

/** A Workers secret that must be set is missing or too weak to use. */
export class InvalidSecretError extends Error {
  override name = "InvalidSecretError";
}

/** Google's OAuth endpoint refused to hand out an access token. */
export class AccessTokenError extends Error {
  override name = "AccessTokenError";
}

/** Google's token exchange refused to narrow an access token, or answered something unusable. */
export class TokenExchangeError extends Error {
  override name = "TokenExchangeError";
}

/** A Cloud Storage call answered something the caller did not expect. */
export class GcsRequestError extends Error {
  override name = "GcsRequestError";
  constructor(
    operation: string,
    readonly status: number,
    detail?: string,
  ) {
    super(`${operation} failed with HTTP ${status}${detail ? `: ${detail}` : ""}`);
  }
}

/** The derived bucket name exists but belongs to another project. */
export class BucketNameTakenError extends Error {
  override name = "BucketNameTakenError";
}
