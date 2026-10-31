import { createRemoteJWKSet, type JWTVerifyGetKey } from "jose";
import { handleRequest } from "./handler";

// One key set per JWKS URL for the life of the isolate: jose caches the fetched keys inside it,
// so rebuilding it per request would fetch the keys on every request.
const keySets = new Map<string, JWTVerifyGetKey>();

function getKeySet(jwksUrl: string): JWTVerifyGetKey {
  let keys = keySets.get(jwksUrl);
  if (!keys) {
    keys = createRemoteJWKSet(new URL(jwksUrl));
    keySets.set(jwksUrl, keys);
  }
  return keys;
}

export default {
  async fetch(request, env): Promise<Response> {
    const issuer = `${env.SUPABASE_URL}/auth/v1`;
    return handleRequest(request, {
      issuer,
      keys: getKeySet(`${issuer}/.well-known/jwks.json`),
    });
  },
} satisfies ExportedHandler<Env>;
