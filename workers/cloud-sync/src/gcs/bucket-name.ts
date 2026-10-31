const BUCKET_PREFIX = "spark-";
const HEX_CHARS_KEPT = 32; // 128 bits

function toHex(bytes: Uint8Array): string {
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
}

/**
 * `spark-` + the first 128 bits (hex) of HMAC-SHA256(secret, lower-cased user id): 38 characters.
 * The HMAC key is the UTF-8 bytes of the `BUCKET_NAME_KEY` secret text, as is.
 *
 * Once real users have buckets this must never change: neither the algorithm nor the secret.
 * A different result for the same user means their data can no longer be found.
 */
export async function deriveBucketName(userId: string, secret: string): Promise<string> {
  const encoder = new TextEncoder();
  const key = await crypto.subtle.importKey(
    "raw",
    encoder.encode(secret),
    { name: "HMAC", hash: "SHA-256" },
    false,
    ["sign"],
  );
  const mac = await crypto.subtle.sign("HMAC", key, encoder.encode(userId.toLowerCase()));
  return BUCKET_PREFIX + toHex(new Uint8Array(mac)).slice(0, HEX_CHARS_KEPT);
}
