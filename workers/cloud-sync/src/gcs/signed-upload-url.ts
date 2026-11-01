import { createSigningKeyLoader } from "./signing-key";
import type { ServiceAccountKey } from "./service-account-key";

// Cloud Storage "V4 signing process" for the XML API:
// https://cloud.google.com/storage/docs/authentication/canonical-requests
const STORAGE_HOST = "storage.googleapis.com";
const ALGORITHM = "GOOG4-RSA-SHA256";
const UNSIGNED_PAYLOAD = "UNSIGNED-PAYLOAD";

/**
 * Sorted by name, which is the order they must appear in. The client has to send the last three
 * with exactly the signed values, or Google answers 403.
 */
const SIGNED_HEADER_NAMES = ["host", "x-goog-content-length-range", "x-goog-if-generation-match", "x-goog-meta-version"];

export type UploadUrlRequest = {
  bucket: string;
  object: string;
  /** The body may be at most this many bytes; Google refuses a bigger one with 400. */
  maxBytes: number;
  /**
   * Google refuses the upload unless the object's generation is exactly this: "0" means the object
   * must not exist yet. A successful upload changes the generation, so the same link cannot be reused.
   * A string because generations are 64-bit integers.
   */
  ifGenerationMatch: string;
  /** Stored on the object as the `version` metadata key. */
  version: number;
};

export type SignedUpload = {
  method: "PUT";
  /** PUT the bytes here. */
  url: string;
  /** The headers the PUT must carry, exactly as given. */
  headers: Record<string, string>;
  /** Seconds since the epoch. */
  expiresAt: number;
};

export type UploadUrlSigner = {
  signUpload(request: UploadUrlRequest): Promise<SignedUpload>;
};

export type UploadUrlSignerOptions = {
  key: ServiceAccountKey;
  /**
   * A link stops working this long after it is signed, and cannot be renewed. The whole upload has
   * to finish inside this time: Google refuses an upload that is still running when it ends.
   */
  lifetimeSeconds: number;
  /** Milliseconds since the epoch. Injected by tests. */
  now?: () => number;
};

/** RFC 3986 percent-encoding, which is stricter than `encodeURIComponent`. */
function encodeRfc3986(text: string): string {
  return encodeURIComponent(text).replace(/[!'()*]/g, (char) => `%${char.charCodeAt(0).toString(16).toUpperCase()}`);
}

function toHex(bytes: ArrayBuffer): string {
  return Array.from(new Uint8Array(bytes), (byte) => byte.toString(16).padStart(2, "0")).join("");
}

async function sha256Hex(text: string): Promise<string> {
  return toHex(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(text)));
}

/** `20261101T000000Z` */
function formatTimestamp(ms: number): string {
  return new Date(ms).toISOString().replace(/[-:]|\.\d{3}/g, "");
}

/**
 * Signs PUT links for single objects with the service account's private key, locally: no request
 * to Google is made, and the link's rules (size, generation, version) are part of what is signed.
 */
export function createUploadUrlSigner(options: UploadUrlSignerOptions): UploadUrlSigner {
  const { key, lifetimeSeconds } = options;
  if (!Number.isSafeInteger(lifetimeSeconds) || lifetimeSeconds < 1) {
    throw new RangeError("lifetimeSeconds must be a positive whole number");
  }
  const now = options.now ?? Date.now;
  const loadSigningKey = createSigningKeyLoader(key.privateKey);

  return {
    async signUpload(request) {
      if (!Number.isSafeInteger(request.maxBytes) || request.maxBytes < 0) {
        throw new RangeError("maxBytes must be a non-negative integer");
      }
      if (!/^\d+$/.test(request.ifGenerationMatch)) {
        throw new RangeError("ifGenerationMatch must be a generation number, or 0");
      }
      if (!Number.isSafeInteger(request.version) || request.version < 0) {
        throw new RangeError("version must be a non-negative integer");
      }

      const issuedAtMs = now();
      const timestamp = formatTimestamp(issuedAtMs);
      const scope = `${timestamp.slice(0, 8)}/auto/storage/goog4_request`;

      const headers = {
        "x-goog-content-length-range": `0,${request.maxBytes}`,
        "x-goog-if-generation-match": request.ifGenerationMatch,
        "x-goog-meta-version": String(request.version),
      };
      const canonicalHeaders = [`host:${STORAGE_HOST}`, ...Object.entries(headers).map(([name, value]) => `${name}:${value}`)];
      const signedHeaders = SIGNED_HEADER_NAMES.join(";");

      const query = [
        ["X-Goog-Algorithm", ALGORITHM],
        ["X-Goog-Credential", `${key.clientEmail}/${scope}`],
        ["X-Goog-Date", timestamp],
        ["X-Goog-Expires", String(lifetimeSeconds)],
        ["X-Goog-SignedHeaders", signedHeaders],
      ]
        .map(([name, value]) => `${encodeRfc3986(name)}=${encodeRfc3986(value)}`)
        .sort()
        .join("&");

      const path = `/${encodeRfc3986(request.bucket)}/${request.object.split("/").map(encodeRfc3986).join("/")}`;
      const canonicalRequest = ["PUT", path, query, ...canonicalHeaders, "", signedHeaders, UNSIGNED_PAYLOAD].join("\n");
      const stringToSign = [ALGORITHM, timestamp, scope, await sha256Hex(canonicalRequest)].join("\n");

      const signature = await crypto.subtle.sign(
        "RSASSA-PKCS1-v1_5",
        await loadSigningKey(),
        new TextEncoder().encode(stringToSign),
      );

      return {
        method: "PUT",
        url: `https://${STORAGE_HOST}${path}?${query}&X-Goog-Signature=${toHex(signature)}`,
        headers,
        expiresAt: Math.floor(issuedAtMs / 1000) + lifetimeSeconds,
      };
    },
  };
}
