import { createRemoteJWKSet, type JWTVerifyGetKey } from "jose";
import { createGcsServices, type GcsServices } from "./gcs/gcs-services";
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
let gcsServices: GcsServices | undefined;

function getGcsServices(env: Env): GcsServices {
  gcsServices ??= createGcsServices({
    serviceAccountKeyJson: env.GCS_SERVICE_ACCOUNT_KEY,
    bucketNameKey: env.BUCKET_NAME_KEY,
    projectId: env.GCS_PROJECT_ID,
    projectNumber: env.GCS_PROJECT_NUMBER,
    maxObjectBytes: env.MAX_OBJECT_BYTES,
    maxUserTotalBytes: env.MAX_USER_TOTAL_BYTES,
    uploadLinkLifetimeSeconds: env.UPLOAD_LINK_LIFETIME_SECONDS,
  });
  return gcsServices;
}

export default {
  async fetch(request, env): Promise<Response> {
    const issuer = `${env.SUPABASE_URL}/auth/v1`;
    return handleRequest(request, {
      verifier: { issuer, keys: getKeySet(`${issuer}/.well-known/jwks.json`) },
      credentials: { issueUserCredentials: (userId) => getGcsServices(env).credentials.issueUserCredentials(userId) },
      documents: {
        issueUpload: (userId, request) => getGcsServices(env).documents.issueUpload(userId, request),
        listDocuments: (userId) => getGcsServices(env).documents.listDocuments(userId),
        deleteDocument: (userId, documentId) => getGcsServices(env).documents.deleteDocument(userId, documentId),
      },
    });
  },
} satisfies ExportedHandler<Env>;
