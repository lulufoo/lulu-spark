// Runs against the real Google Cloud project, so it is not part of `npm test`. Run it with
//   GCS_SERVICE_ACCOUNT_KEY="$(cat ~/.config/lulu-spark/gcs-sync-sa-key.json)" npm run test:live
// It creates one real bucket per run. The long-lived account cannot delete buckets (by design), so
// the owner removes it afterwards. Each name is appended to `.live-test-buckets.log` (git-ignored).
import { appendFileSync } from "node:fs";
import { beforeAll, describe, expect, it } from "vitest";
import { createAccessTokenProvider } from "../src/gcs/access-token-provider";
import { createBucketProvisioner } from "../src/gcs/bucket-provisioner";
import { createBucketService } from "../src/gcs/bucket-service";
import { BucketNameTakenError } from "../src/gcs/errors";
import { parseServiceAccountKey } from "../src/gcs/service-account-key";

const KEY_JSON = process.env.GCS_SERVICE_ACCOUNT_KEY;
const PROJECT_ID = "spark-cloud-sync";
const PROJECT_NUMBER = "55973988960";
const STORAGE = "https://storage.googleapis.com/storage/v1";
const UPLOAD = "https://storage.googleapis.com/upload/storage/v1";
// A public bucket owned by Google: the name is taken, and readable, so its project number is not ours.
const BUCKET_HELD_BY_SOMEONE_ELSE = "gcp-public-data-landsat";

function makeRandomHex(bytes: number): string {
  return Array.from(crypto.getRandomValues(new Uint8Array(bytes)), (b) => b.toString(16).padStart(2, "0")).join("");
}

describe.skipIf(!KEY_JSON)("real Google Cloud Storage", () => {
  const tokens = createAccessTokenProvider({ key: parseServiceAccountKey(KEY_JSON) });
  // A throwaway key and user: the real BUCKET_NAME_KEY is never needed here.
  const service = createBucketService({
    serviceAccountKeyJson: KEY_JSON,
    bucketNameKey: makeRandomHex(32),
    projectId: PROJECT_ID,
    projectNumber: PROJECT_NUMBER,
  });
  const userId = crypto.randomUUID();
  let bucket = "";
  let firstResults: { bucket: string; created: boolean }[] = [];

  async function callStorage(url: string, init: RequestInit = {}): Promise<Response> {
    const headers = new Headers(init.headers);
    headers.set("Authorization", `Bearer ${await tokens.getAccessToken()}`);
    return fetch(url, { ...init, headers });
  }

  beforeAll(async () => {
    firstResults = await Promise.all([service.ensureUserBucket(userId), service.ensureUserBucket(userId)]);
    bucket = firstResults[0].bucket;
    appendFileSync(".live-test-buckets.log", `${new Date().toISOString()} ${bucket}\n`);
  });

  it("lets two first calls at once both succeed, and only one of them creates the bucket", () => {
    expect(firstResults[1].bucket).toBe(bucket);
    expect(firstResults.filter((result) => result.created)).toHaveLength(1);
  });

  it("finds the bucket on a repeat call instead of creating it again", async () => {
    expect(await service.ensureUserBucket(userId)).toEqual({ bucket, created: false });
  });

  it("created the bucket in us-west1, with uniform access, versioning, and the 3-day cleanup rule", async () => {
    const response = await callStorage(
      `${STORAGE}/b/${bucket}?fields=location,projectNumber,versioning,iamConfiguration/uniformBucketLevelAccess/enabled,lifecycle`,
    );
    expect(response.status).toBe(200);
    expect(await response.json()).toEqual({
      location: "US-WEST1",
      projectNumber: PROJECT_NUMBER,
      versioning: { enabled: true },
      iamConfiguration: { uniformBucketLevelAccess: { enabled: true } },
      lifecycle: {
        rule: [{ action: { type: "Delete" }, condition: { isLive: false, daysSinceNoncurrentTime: 3 } }],
      },
    });
  });

  it("keeps the old versions of an object that is overwritten and then deleted", async () => {
    const objectUrl = `${STORAGE}/b/${bucket}/o/doc-1`;
    for (const body of ["first", "second, longer"]) {
      const upload = await callStorage(`${UPLOAD}/b/${bucket}/o?uploadType=media&name=doc-1`, { method: "POST", body });
      expect(upload.status).toBe(200);
    }
    expect((await callStorage(objectUrl, { method: "DELETE" })).status).toBe(204);

    const live = await (await callStorage(`${STORAGE}/b/${bucket}/o?fields=items(name)`)).json();
    const withVersions = (await (await callStorage(`${STORAGE}/b/${bucket}/o?versions=true&fields=items(name)`)).json()) as {
      items?: { name: string }[];
    };
    expect((live as { items?: unknown[] }).items ?? []).toHaveLength(0);
    expect(withVersions.items).toHaveLength(2);
  });

  it("reports an error, not success, for a bucket name that someone else already holds", async () => {
    const provisioner = createBucketProvisioner({
      projectId: PROJECT_ID,
      projectNumber: PROJECT_NUMBER,
      getAccessToken: () => tokens.getAccessToken(),
    });
    await expect(provisioner.ensureBucket(BUCKET_HELD_BY_SOMEONE_ELSE)).rejects.toBeInstanceOf(BucketNameTakenError);
  });
});
