import { createAccessTokenProvider } from "./access-token-provider";
import { createBucketService } from "./bucket-service";
import { createClientTokenIssuer } from "./client-token-issuer";
import { parseServiceAccountKey } from "./service-account-key";

export type UserCredentials = {
  bucket: string;
  /** Reads and lists objects in `bucket`. Nothing else, and nothing in any other bucket. */
  accessToken: string;
  /** Seconds since the epoch. */
  expiresAt: number;
};

export type CredentialsService = {
  /** Makes sure the user's bucket exists, then hands out a read-only token for it. */
  issueUserCredentials(userId: string): Promise<UserCredentials>;
};

export type CredentialsServiceConfig = {
  /** The key file of the long-lived service account, as JSON text. */
  serviceAccountKeyJson: string | undefined;
  /** The `BUCKET_NAME_KEY` secret. */
  bucketNameKey: string | undefined;
  projectId: string;
  projectNumber: string;
  /** Injected by tests. */
  fetchFn?: typeof fetch;
  /** Milliseconds since the epoch. Injected by tests. */
  now?: () => number;
};

/**
 * Wires the pieces together around one Google access token for the long-lived account, so the
 * bucket check and the token exchange share it.
 */
export function createCredentialsService(config: CredentialsServiceConfig): CredentialsService {
  const tokens = createAccessTokenProvider({
    key: parseServiceAccountKey(config.serviceAccountKeyJson),
    fetchFn: config.fetchFn,
    now: config.now,
  });
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

  return {
    async issueUserCredentials(userId) {
      const { bucket } = await buckets.ensureUserBucket(userId);
      const { accessToken, expiresAt } = await issuer.issueReadOnlyToken(bucket);
      return { bucket, accessToken, expiresAt };
    },
  };
}
