import type { BucketService } from "./bucket-service";
import type { ClientTokenIssuer } from "./client-token-issuer";

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

export type CredentialsServiceDeps = {
  buckets: BucketService;
  issuer: ClientTokenIssuer;
};

export function createCredentialsService(deps: CredentialsServiceDeps): CredentialsService {
  return {
    async issueUserCredentials(userId) {
      const { bucket } = await deps.buckets.ensureUserBucket(userId);
      const { accessToken, expiresAt } = await deps.issuer.issueReadOnlyToken(bucket);
      return { bucket, accessToken, expiresAt };
    },
  };
}
