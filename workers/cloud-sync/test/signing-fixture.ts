import {
  SignJWT,
  createLocalJWKSet,
  exportJWK,
  generateKeyPair,
  type JWTVerifyGetKey,
} from "jose";

export const ISSUER = "https://test-ref.supabase.co/auth/v1";
export const USER_ID = "123e4567-e89b-12d3-a456-426614174000";
const KEY_ID = "test-key";

type TokenOptions = {
  /** `null` leaves the `sub` claim out. */
  subject?: string | null;
  issuer?: string;
  audience?: string;
  /** Epoch seconds, or a jose duration such as "1h". */
  expiresAt?: number | string;
};

export type Signer = {
  /** The public half, as the verifier sees it (what the Supabase JWKS endpoint would serve). */
  keys: JWTVerifyGetKey;
  signToken: (options?: TokenOptions) => Promise<string>;
};

/** A fresh ES256 key pair: tokens it signs verify against `keys` and nothing else. */
export async function makeSigner(): Promise<Signer> {
  const { publicKey, privateKey } = await generateKeyPair("ES256");
  const jwk = { ...(await exportJWK(publicKey)), kid: KEY_ID, alg: "ES256", use: "sig" };

  async function signToken(options: TokenOptions = {}): Promise<string> {
    const jwt = new SignJWT({ role: "authenticated" })
      .setProtectedHeader({ alg: "ES256", kid: KEY_ID })
      .setIssuer(options.issuer ?? ISSUER)
      .setAudience(options.audience ?? "authenticated")
      .setIssuedAt()
      .setExpirationTime(options.expiresAt ?? "1h");
    if (options.subject !== null) jwt.setSubject(options.subject ?? USER_ID);
    return jwt.sign(privateKey);
  }

  return { keys: createLocalJWKSet({ keys: [jwk] }), signToken };
}
