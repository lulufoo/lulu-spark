import { createAccessTokenProvider } from "./access-token-provider";
import { deriveBucketName } from "./bucket-name";
import { createBucketProvisioner } from "./bucket-provisioner";
import { InvalidSecretError } from "./errors";
import { parseServiceAccountKey } from "./service-account-key";

// A generated key is 64 hex characters. Anything shorter was not generated, or was cut off.
const MIN_BUCKET_NAME_KEY_LENGTH = 32;

export type UserBucket = {
  bucket: string;
  created: boolean;
};

export type BucketService = {
  /** The user's own bucket, created on first use. Safe to call again and from two devices at once. */
  ensureUserBucket(userId: string): Promise<UserBucket>;
};

export type BucketServiceConfig = {
  /** The key file of the long-lived service account, as JSON text. */
  serviceAccountKeyJson: string | undefined;
  /** The `BUCKET_NAME_KEY` secret. Names derived with a wrong key would orphan every user's data. */
  bucketNameKey: string | undefined;
  projectId: string;
  projectNumber: string;
  /** Injected by tests. */
  fetchFn?: typeof fetch;
  now?: () => number;
};

export function createBucketService(config: BucketServiceConfig): BucketService {
  const { bucketNameKey } = config;
  if (!bucketNameKey || bucketNameKey.length < MIN_BUCKET_NAME_KEY_LENGTH) {
    throw new InvalidSecretError("BUCKET_NAME_KEY is missing or too short");
  }
  const tokens = createAccessTokenProvider({
    key: parseServiceAccountKey(config.serviceAccountKeyJson),
    fetchFn: config.fetchFn,
    now: config.now,
  });
  const provisioner = createBucketProvisioner({
    projectId: config.projectId,
    projectNumber: config.projectNumber,
    getAccessToken: () => tokens.getAccessToken(),
    fetchFn: config.fetchFn,
  });

  return {
    async ensureUserBucket(userId) {
      const bucket = await deriveBucketName(userId, bucketNameKey);
      const { created } = await provisioner.ensureBucket(bucket);
      return { bucket, created };
    },
  };
}
