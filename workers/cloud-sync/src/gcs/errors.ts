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

/** A Workers setting that must be a number (a size limit) is missing or not usable. */
export class InvalidLimitsError extends Error {
  override name = "InvalidLimitsError";
}

/** The caller's request is malformed. The message is written to be sent back to the caller. */
export class InvalidRequestError extends Error {
  override name = "InvalidRequestError";
}

/** The upload is bigger than one object may be. */
export class ObjectTooLargeError extends Error {
  override name = "ObjectTooLargeError";
  constructor(readonly limitBytes: number) {
    super(`object is larger than the limit of ${limitBytes} bytes`);
  }
}

/** The user's bucket would hold more than a user may store, counting old versions not yet cleaned up. */
export class QuotaExceededError extends Error {
  override name = "QuotaExceededError";
  constructor(
    readonly limitBytes: number,
    readonly usedBytes: number,
  ) {
    super(`user storage would exceed the limit of ${limitBytes} bytes (${usedBytes} in use)`);
  }
}

/** There are too many objects to add up in one request. Treated as a failure, never as "no usage". */
export class ListingTooLargeError extends Error {
  override name = "ListingTooLargeError";
}
