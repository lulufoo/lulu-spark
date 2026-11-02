import { errors, jwtVerify, type JWTVerifyGetKey } from "jose";

export type TokenVerifier = {
  /** Expected `iss`, e.g. `https://<ref>.supabase.co/auth/v1`. */
  issuer: string;
  keys: JWTVerifyGetKey;
};

export type VerifiedUser = { userId: string };

// Supabase signs user sessions with ES256 and sets aud to "authenticated".
const AUDIENCE = "authenticated";
const ALGORITHMS = ["ES256"];

/** The token in `Authorization: Bearer <token>`, or null when the header is absent or malformed. */
export function readBearerToken(request: Request): string | null {
  const header = request.headers.get("Authorization");
  if (!header) return null;
  const match = /^Bearer +(\S+)$/i.exec(header);
  return match ? match[1] : null;
}

/** Errors that mean "this token is not acceptable", as opposed to "we could not check it". */
function isRejectedToken(error: unknown): boolean {
  return (
    error instanceof errors.JWTExpired ||
    error instanceof errors.JWTClaimValidationFailed ||
    error instanceof errors.JWTInvalid ||
    error instanceof errors.JWSInvalid ||
    error instanceof errors.JWSSignatureVerificationFailed ||
    error instanceof errors.JOSEAlgNotAllowed ||
    error instanceof errors.JWKSNoMatchingKey ||
    error instanceof errors.JWKSMultipleMatchingKeys
  );
}

/**
 * The user behind a valid token, or null when the token is rejected.
 * Rethrows when the key set cannot be fetched (network, timeout, non-200): the caller must not
 * treat that as a bad token.
 */
export async function verifyAccessToken(
  token: string,
  verifier: TokenVerifier,
): Promise<VerifiedUser | null> {
  try {
    const { payload } = await jwtVerify(token, verifier.keys, {
      issuer: verifier.issuer,
      audience: AUDIENCE,
      algorithms: ALGORITHMS,
    });
    return typeof payload.sub === "string" && payload.sub !== "" ? { userId: payload.sub } : null;
  } catch (error) {
    if (isRejectedToken(error)) return null;
    throw error;
  }
}
