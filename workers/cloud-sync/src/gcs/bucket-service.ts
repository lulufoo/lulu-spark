import { deriveBucketName } from "./bucket-name";
import { createBucketProvisioner } from "./bucket-provisioner";
import { InvalidSecretError } from "./errors";

// A generated key is 64 hex characters. Anything shorter was not generated, or was cut off.
const MIN_BUCKET_NAME_KEY_LENGTH = 32;

export type UserBucket = {
  bucket: string;
  created: boolean;
};

export type BucketService = {
  /** The user's own bucket, created on first use. Safe to call again and from two devices at once. */
  ensureUserBucket(userId: string): Promise<UserBucket>;
  /** The name of the user's bucket, without creating or checking anything. */
  nameFor(userId: string): Promise<string>;
};

export type BucketServiceConfig = {
  /** The `BUCKET_NAME_KEY` secret. Names derived with a wrong key would orphan every user's data. */
  bucketNameKey: string | undefined;
  projectId: string;
  projectNumber: string;
  /** The long-lived account's access token. */
  getAccessToken: () => Promise<string>;
  /** Injected by tests. */
  fetchFn?: typeof fetch;
};

export function createBucketService(config: BucketServiceConfig): BucketService {
  const { bucketNameKey } = config;
  if (!bucketNameKey || bucketNameKey.length < MIN_BUCKET_NAME_KEY_LENGTH) {
    throw new InvalidSecretError("BUCKET_NAME_KEY is missing or too short");
  }
  const provisioner = createBucketProvisioner({
    projectId: config.projectId,
    projectNumber: config.projectNumber,
    getAccessToken: config.getAccessToken,
    fetchFn: config.fetchFn,
  });

  return {
    async ensureUserBucket(userId) {
      const bucket = await deriveBucketName(userId, bucketNameKey);
      const { created } = await provisioner.ensureBucket(bucket);
      return { bucket, created };
    },
    nameFor: (userId) => deriveBucketName(userId, bucketNameKey),
  };
}
