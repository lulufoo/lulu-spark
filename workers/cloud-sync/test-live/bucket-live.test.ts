// Runs against the real Google Cloud project, so it is not part of `npm test`. Run it with
//   GCS_SERVICE_ACCOUNT_KEY="$(cat ~/.config/lulu-spark/gcs-sync-sa-key.json)" npm run test:live
// It creates two real buckets per run (two throwaway users). The long-lived account cannot delete
// buckets (by design), so the owner removes them afterwards. Each name is appended to
// `.live-test-buckets.log` (git-ignored).
import { appendFileSync } from "node:fs";
import { beforeAll, describe, expect, it } from "vitest";
import { createAccessTokenProvider } from "../src/gcs/access-token-provider";
import { createBucketProvisioner } from "../src/gcs/bucket-provisioner";
import { createBucketService } from "../src/gcs/bucket-service";
import { createCredentialsService, type UserCredentials } from "../src/gcs/credentials-service";
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
  const key = parseServiceAccountKey(KEY_JSON);
  const longLived = createAccessTokenProvider({ key });
  // A throwaway name key and users: the real BUCKET_NAME_KEY is never needed here.
  const bucketNameKey = makeRandomHex(32);
  const buckets = createBucketService({
    bucketNameKey,
    projectId: PROJECT_ID,
    projectNumber: PROJECT_NUMBER,
    getAccessToken: () => longLived.getAccessToken(),
  });
  const credentialsService = createCredentialsService({
    serviceAccountKeyJson: KEY_JSON,
    bucketNameKey,
    projectId: PROJECT_ID,
    projectNumber: PROJECT_NUMBER,
  });
  const userId = crypto.randomUUID();
  const otherUserId = crypto.randomUUID();
  let bucket = "";
  let otherBucket = "";
  let firstResults: { bucket: string; created: boolean }[] = [];

  /** A call with the long-lived account's token: used to prepare and check things, never as the client. */
  async function callAsServer(url: string, init: RequestInit = {}): Promise<Response> {
    const headers = new Headers(init.headers);
    headers.set("Authorization", `Bearer ${await longLived.getAccessToken()}`);
    return fetch(url, { ...init, headers });
  }

  beforeAll(async () => {
    firstResults = await Promise.all([buckets.ensureUserBucket(userId), buckets.ensureUserBucket(userId)]);
    bucket = firstResults[0].bucket;
    otherBucket = (await buckets.ensureUserBucket(otherUserId)).bucket;
    for (const name of [bucket, otherBucket]) appendFileSync(".live-test-buckets.log", `${new Date().toISOString()} ${name}\n`);
  });

  it("lets two first calls at once both succeed, and only one of them creates the bucket", () => {
    expect(firstResults[1].bucket).toBe(bucket);
    expect(firstResults.filter((result) => result.created)).toHaveLength(1);
  });

  it("finds the bucket on a repeat call instead of creating it again", async () => {
    expect(await buckets.ensureUserBucket(userId)).toEqual({ bucket, created: false });
  });

  it("created the bucket in us-west1, with uniform access, versioning, and the 3-day cleanup rule", async () => {
    const response = await callAsServer(
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
      const upload = await callAsServer(`${UPLOAD}/b/${bucket}/o?uploadType=media&name=doc-1`, { method: "POST", body });
      expect(upload.status).toBe(200);
    }
    expect((await callAsServer(objectUrl, { method: "DELETE" })).status).toBe(204);

    const live = await (await callAsServer(`${STORAGE}/b/${bucket}/o?fields=items(name)`)).json();
    const withVersions = (await (await callAsServer(`${STORAGE}/b/${bucket}/o?versions=true&fields=items(name)`)).json()) as {
      items?: { name: string }[];
    };
    expect((live as { items?: unknown[] }).items ?? []).toHaveLength(0);
    expect(withVersions.items).toHaveLength(2);
  });

  it("reports an error, not success, for a bucket name that someone else already holds", async () => {
    const provisioner = createBucketProvisioner({
      projectId: PROJECT_ID,
      projectNumber: PROJECT_NUMBER,
      getAccessToken: () => longLived.getAccessToken(),
    });
    await expect(provisioner.ensureBucket(BUCKET_HELD_BY_SOMEONE_ELSE)).rejects.toBeInstanceOf(BucketNameTakenError);
  });

  describe("the token handed to the client", () => {
    let mine: UserCredentials;
    let asClient: (url: string, init?: RequestInit) => Promise<Response>;

    beforeAll(async () => {
      for (const [name, body] of [
        [bucket, "mine"],
        [otherBucket, "someone else's"],
      ]) {
        const upload = await callAsServer(`${UPLOAD}/b/${name}/o?uploadType=media&name=readable.txt`, { method: "POST", body });
        expect(upload.status).toBe(200);
      }
      mine = await credentialsService.issueUserCredentials(userId);
      asClient = (url, init = {}) => {
        const headers = new Headers(init.headers);
        headers.set("Authorization", `Bearer ${mine.accessToken}`);
        return fetch(url, { ...init, headers });
      };
    });

    it("is for the user's bucket and lasts between half an hour and an hour", () => {
      expect(mine.bucket).toBe(bucket);
      const secondsLeft = mine.expiresAt - Date.now() / 1000;
      expect(secondsLeft).toBeGreaterThan(29 * 60);
      expect(secondsLeft).toBeLessThanOrEqual(60 * 60);
    });

    it("reads and lists the user's own objects", async () => {
      const read = await asClient(`${STORAGE}/b/${bucket}/o/readable.txt?alt=media`);
      expect(read.status).toBe(200);
      expect(await read.text()).toBe("mine");
      const list = await asClient(`${STORAGE}/b/${bucket}/o?fields=items(name)`);
      expect(list.status).toBe(200);
      expect(((await list.json()) as { items: { name: string }[] }).items.map((item) => item.name)).toContain("readable.txt");
    });

    it("cannot upload, overwrite or delete", async () => {
      const upload = await asClient(`${UPLOAD}/b/${bucket}/o?uploadType=media&name=new.txt`, { method: "POST", body: "x" });
      const overwrite = await asClient(`${UPLOAD}/b/${bucket}/o?uploadType=media&name=readable.txt`, { method: "POST", body: "x" });
      const remove = await asClient(`${STORAGE}/b/${bucket}/o/readable.txt`, { method: "DELETE" });
      expect([upload.status, overwrite.status, remove.status]).toEqual([403, 403, 403]);
      // And nothing changed.
      expect(await (await callAsServer(`${STORAGE}/b/${bucket}/o/readable.txt?alt=media`)).text()).toBe("mine");
    });

    it("cannot touch another user's bucket, nor create or inspect buckets", async () => {
      const readOther = await asClient(`${STORAGE}/b/${otherBucket}/o/readable.txt?alt=media`);
      const listOther = await asClient(`${STORAGE}/b/${otherBucket}/o`);
      const create = await asClient(`${STORAGE}/b?project=${PROJECT_ID}`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ name: `spark-denied-${makeRandomHex(8)}`, location: "US-WEST1" }),
      });
      const inspect = await asClient(`${STORAGE}/b/${bucket}?fields=name`);
      expect([readOther.status, listOther.status, create.status, inspect.status]).toEqual([403, 403, 403, 403]);
    });
  });
});
