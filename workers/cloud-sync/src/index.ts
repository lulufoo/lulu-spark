import { createRemoteJWKSet, type JWTVerifyGetKey } from "jose";
import { createBucketService, type BucketService } from "./gcs/bucket-service";
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

// Built on first use and kept: it holds the cached Google access token. Building it only when a
// route needs it keeps a missing secret from taking down the routes that do not use storage.
let bucketService: BucketService | undefined;

function getBucketService(env: Env): BucketService {
  bucketService ??= createBucketService({
    serviceAccountKeyJson: env.GCS_SERVICE_ACCOUNT_KEY,
    bucketNameKey: env.BUCKET_NAME_KEY,
    projectId: env.GCS_PROJECT_ID,
    projectNumber: env.GCS_PROJECT_NUMBER,
  });
  return bucketService;
}

export default {
  async fetch(request, env): Promise<Response> {
    const issuer = `${env.SUPABASE_URL}/auth/v1`;
    return handleRequest(request, {
      verifier: { issuer, keys: getKeySet(`${issuer}/.well-known/jwks.json`) },
      buckets: { ensureUserBucket: (userId) => getBucketService(env).ensureUserBucket(userId) },
    });
  },
} satisfies ExportedHandler<Env>;
