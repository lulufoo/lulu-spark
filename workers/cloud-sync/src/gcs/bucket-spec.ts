export const BUCKET_LOCATION = "US-WEST1";
export const NONCURRENT_RETENTION_DAYS = 3;

/**
 * What a user's bucket is created with. Everything is set at creation, so the long-lived account
 * needs only the "create bucket" permission:
 * - fixed region (the free-tier regions are us-west1, us-central1, us-east1);
 * - uniform bucket-level access (no per-object ACLs);
 * - object versioning, so an overwrite or delete keeps history inside GCS;
 * - old versions are deleted once they have been non-current for 3 days.
 */
export function buildBucketSpec(name: string) {
  return {
    name,
    location: BUCKET_LOCATION,
    storageClass: "STANDARD",
    versioning: { enabled: true },
    iamConfiguration: { uniformBucketLevelAccess: { enabled: true } },
    lifecycle: {
      rule: [
        {
          action: { type: "Delete" },
          condition: { isLive: false, daysSinceNoncurrentTime: NONCURRENT_RETENTION_DAYS },
        },
      ],
    },
  };
}
