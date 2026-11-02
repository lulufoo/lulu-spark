import { importPKCS8 } from "jose";

export type SigningKeyLoader = () => Promise<CryptoKey>;

/**
 * Imports the service account's private key (RSA, PKCS#8) once and keeps it, for signing OAuth
 * assertions and signed URLs. A failed import is not kept, so the next call tries again.
 */
export function createSigningKeyLoader(privateKeyPem: string): SigningKeyLoader {
  let loaded: Promise<CryptoKey> | undefined;
  return () => {
    loaded ??= importPKCS8(privateKeyPem, "RS256").catch((error) => {
      loaded = undefined;
      throw error;
    });
    return loaded;
  };
}
