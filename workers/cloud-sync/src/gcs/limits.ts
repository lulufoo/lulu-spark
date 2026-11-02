import { InvalidLimitsError } from "./errors";

// Our own sane range for the setting, not Google's: a link under a minute is hard to use, and one
// over an hour is longer than any upload of the allowed size needs.
const MIN_LINK_LIFETIME_SECONDS = 60;
const MAX_LINK_LIFETIME_SECONDS = 3600;

export type Limits = {
  /** One uploaded object may be at most this many bytes. */
  maxObjectBytes: number;
  /** All of a user's objects together, old versions included, may be at most this many bytes. */
  maxUserTotalBytes: number;
};

function readByteCount(value: unknown, name: string): number {
  const parsed = typeof value === "string" && /^\d+$/.test(value.trim()) ? Number(value) : value;
  if (typeof parsed !== "number" || !Number.isSafeInteger(parsed) || parsed <= 0) {
    throw new InvalidLimitsError(`${name} is missing or not a positive whole number of bytes`);
  }
  return parsed;
}

/**
 * Reads the two size limits from the Workers settings. There is no default on purpose: a missing
 * setting must stop the service, not quietly switch the limit off.
 */
export function parseLimits(maxObjectBytes: unknown, maxUserTotalBytes: unknown): Limits {
  const limits = {
    maxObjectBytes: readByteCount(maxObjectBytes, "MAX_OBJECT_BYTES"),
    maxUserTotalBytes: readByteCount(maxUserTotalBytes, "MAX_USER_TOTAL_BYTES"),
  };
  if (limits.maxObjectBytes > limits.maxUserTotalBytes) {
    throw new InvalidLimitsError("MAX_OBJECT_BYTES is larger than MAX_USER_TOTAL_BYTES");
  }
  return limits;
}

/**
 * Reads how long an upload link works, in seconds, from the Workers settings. There is no default
 * for the same reason as the size limits.
 */
export function parseUploadLinkLifetime(value: unknown): number {
  const parsed = typeof value === "string" && /^\d+$/.test(value.trim()) ? Number(value) : value;
  if (
    typeof parsed !== "number" ||
    !Number.isSafeInteger(parsed) ||
    parsed < MIN_LINK_LIFETIME_SECONDS ||
    parsed > MAX_LINK_LIFETIME_SECONDS
  ) {
    throw new InvalidLimitsError(
      `UPLOAD_LINK_LIFETIME_SECONDS is missing or not a whole number from ${MIN_LINK_LIFETIME_SECONDS} to ${MAX_LINK_LIFETIME_SECONDS}`,
    );
  }
  return parsed;
}
