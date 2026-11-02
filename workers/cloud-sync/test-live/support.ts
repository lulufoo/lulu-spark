import { appendFileSync } from "node:fs";
import type { AccessTokenProvider } from "../src/gcs/access-token-provider";
import { createBucketService } from "../src/gcs/bucket-service";

/** The key file's JSON text. The live tests are skipped when it is not set. */
export const KEY_JSON = process.env.GCS_SERVICE_ACCOUNT_KEY;
export const PROJECT_ID = "spark-cloud-sync";
export const PROJECT_NUMBER = "55973988960";
export const STORAGE = "https://storage.googleapis.com/storage/v1";
export const UPLOAD = "https://storage.googleapis.com/upload/storage/v1";

export function makeRandomHex(bytes: number): string {
  return Array.from(crypto.getRandomValues(new Uint8Array(bytes)), (b) => b.toString(16).padStart(2, "0")).join("");
}

/**
 * The bucket to test in: `GCS_LIVE_BUCKET` when set (reuse one, leave no new bucket behind), else a
 * new one for a throwaway user, noted in `.live-test-buckets.log`. The long-lived account cannot
 * delete buckets, so the owner cleans those up.
 */
export async function pickLiveBucket(longLived: AccessTokenProvider): Promise<string> {
  if (process.env.GCS_LIVE_BUCKET) return process.env.GCS_LIVE_BUCKET;
  const buckets = createBucketService({
    bucketNameKey: makeRandomHex(32),
    projectId: PROJECT_ID,
    projectNumber: PROJECT_NUMBER,
    getAccessToken: () => longLived.getAccessToken(),
  });
  const { bucket } = await buckets.ensureUserBucket(crypto.randomUUID());
  appendFileSync(".live-test-buckets.log", `${new Date().toISOString()} ${bucket}\n`);
  return bucket;
}
