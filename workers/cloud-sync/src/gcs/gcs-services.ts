import { createAccessTokenProvider } from "./access-token-provider";
import { createBucketService } from "./bucket-service";
import { createClientTokenIssuer } from "./client-token-issuer";
import { createCredentialsService, type CredentialsService } from "./credentials-service";
import { createDocumentStorage, type DocumentStorage } from "./document-storage";
import { parseLimits, parseUploadLinkLifetime } from "./limits";
import { createObjectStore } from "./object-store";
import { parseServiceAccountKey } from "./service-account-key";
import { createUploadUrlSigner } from "./signed-upload-url";

export type GcsServices = {
  credentials: CredentialsService;
  documents: DocumentStorage;
};

export type GcsServicesConfig = {
  /** The key file of the long-lived service account, as JSON text. */
  serviceAccountKeyJson: string | undefined;
  /** The `BUCKET_NAME_KEY` secret. */
  bucketNameKey: string | undefined;
  projectId: string;
  projectNumber: string;
  /** `MAX_OBJECT_BYTES` and `MAX_USER_TOTAL_BYTES`: numbers, or digits as text. */
  maxObjectBytes: unknown;
  maxUserTotalBytes: unknown;
  /** `UPLOAD_LINK_LIFETIME_SECONDS`: a number, or digits as text. */
  uploadLinkLifetimeSeconds: unknown;
  /** Injected by tests. */
  fetchFn?: typeof fetch;
  /** Milliseconds since the epoch. Injected by tests. */
  now?: () => number;
};

/**
 * Wires everything that talks to Google around one access token for the long-lived account, so the
 * bucket check, the token exchange and the object calls all share it.
 */
export function createGcsServices(config: GcsServicesConfig): GcsServices {
  const key = parseServiceAccountKey(config.serviceAccountKeyJson);
  const limits = parseLimits(config.maxObjectBytes, config.maxUserTotalBytes);
  const lifetimeSeconds = parseUploadLinkLifetime(config.uploadLinkLifetimeSeconds);
  const tokens = createAccessTokenProvider({ key, fetchFn: config.fetchFn, now: config.now });

  const buckets = createBucketService({
    bucketNameKey: config.bucketNameKey,
    projectId: config.projectId,
    projectNumber: config.projectNumber,
    getAccessToken: () => tokens.getAccessToken(),
    fetchFn: config.fetchFn,
  });
  const issuer = createClientTokenIssuer({
    getAccessToken: (minValidityMs) => tokens.getAccessToken(minValidityMs),
    fetchFn: config.fetchFn,
    now: config.now,
  });
  const store = createObjectStore({ getAccessToken: () => tokens.getAccessToken(), fetchFn: config.fetchFn });
  const signer = createUploadUrlSigner({ key, lifetimeSeconds, now: config.now });

  return {
    credentials: createCredentialsService({ buckets, issuer }),
    documents: createDocumentStorage({ buckets, store, signer, limits }),
  };
}
