import { describe, expect, it } from "vitest";
import { createBucketService } from "../../src/gcs/bucket-service";
import { InvalidSecretError, InvalidServiceAccountKeyError } from "../../src/gcs/errors";
import { jsonReply, makeFakeFetch, type RecordedRequest } from "./fake-fetch";
import { TOKEN_URI, makeServiceAccount } from "./service-account-fixture";

const BUCKET_NAME_KEY = "test-bucket-name-key-0123456789abcdef";
const USER_ID = "123e4567-e89b-12d3-a456-426614174000";
const BUCKET = "spark-dee0434c91366b18561ffdcad4380e86";

/** Google's token endpoint plus a storage API where buckets that exist so far are kept in a set. */
function makeFakeGoogle(existing: Set<string> = new Set()) {
  return makeFakeFetch((request: RecordedRequest) => {
    if (request.url === TOKEN_URI) return jsonReply(200, { access_token: "tok", expires_in: 3600 });
    const bucket = /\/b\/([^?/]+)/.exec(request.url)?.[1];
    if (request.method === "GET" && bucket) {
      return existing.has(bucket)
        ? jsonReply(200, { projectNumber: "55973988960" })
        : jsonReply(404, { error: { message: "not found" } });
    }
    if (request.method === "POST") {
      const name: string = JSON.parse(request.body).name;
      // Like Cloud Storage: a name that exists answers 409 to a second create.
      if (existing.has(name)) return jsonReply(409, { error: { message: "already own it" } });
      existing.add(name);
      return jsonReply(200, {});
    }
    throw new Error(`unexpected request: ${request.method} ${request.url}`);
  });
}

async function makeService(fetchFn: typeof fetch, overrides: { bucketNameKey?: string; keyJson?: string } = {}) {
  const account = await makeServiceAccount();
  return createBucketService({
    serviceAccountKeyJson: overrides.keyJson ?? account.keyJson,
    bucketNameKey: "bucketNameKey" in overrides ? overrides.bucketNameKey : BUCKET_NAME_KEY,
    projectId: "spark-cloud-sync",
    projectNumber: "55973988960",
    fetchFn,
  });
}

describe("createBucketService", () => {
  it("creates the user's bucket on first use and finds it afterwards", async () => {
    const { fetchFn } = makeFakeGoogle();
    const service = await makeService(fetchFn);

    expect(await service.ensureUserBucket(USER_ID)).toEqual({ bucket: BUCKET, created: true });
    expect(await service.ensureUserBucket(USER_ID)).toEqual({ bucket: BUCKET, created: false });
  });

  it("asks Google for an access token once for many calls", async () => {
    const { fetchFn, requests } = makeFakeGoogle();
    const service = await makeService(fetchFn);

    await service.ensureUserBucket(USER_ID);
    await service.ensureUserBucket(USER_ID);

    expect(requests.filter((request) => request.url === TOKEN_URI)).toHaveLength(1);
  });

  it("gives two devices that start together the same bucket, created once", async () => {
    const { fetchFn, requests } = makeFakeGoogle();
    const service = await makeService(fetchFn);

    const results = await Promise.all([service.ensureUserBucket(USER_ID), service.ensureUserBucket(USER_ID)]);

    expect(results.map((result) => result.bucket)).toEqual([BUCKET, BUCKET]);
    expect(results.filter((result) => result.created)).toHaveLength(1);
    // Whether the second device read "missing" first (and got a 409 on create) or found the bucket
    // already there depends on timing; the provisioner tests pin the 409 path. Either way: one create wins.
    const creates = requests.filter((request) => request.method === "POST" && request.url !== TOKEN_URI);
    expect(creates.length).toBeGreaterThanOrEqual(1);
  });

  it("refuses to start without BUCKET_NAME_KEY, or with a cut-off one", async () => {
    const { fetchFn } = makeFakeGoogle();
    await expect(makeService(fetchFn, { bucketNameKey: undefined })).rejects.toBeInstanceOf(InvalidSecretError);
    await expect(makeService(fetchFn, { bucketNameKey: "short" })).rejects.toBeInstanceOf(InvalidSecretError);
  });

  it("refuses to start without a usable service account key", async () => {
    const { fetchFn } = makeFakeGoogle();
    await expect(makeService(fetchFn, { keyJson: "{}" })).rejects.toBeInstanceOf(InvalidServiceAccountKeyError);
  });

  it("never lets the key material into the result", async () => {
    const { fetchFn } = makeFakeGoogle();
    const account = await makeServiceAccount();
    const service = createBucketService({
      serviceAccountKeyJson: account.keyJson,
      bucketNameKey: BUCKET_NAME_KEY,
      projectId: "spark-cloud-sync",
      projectNumber: "55973988960",
      fetchFn,
    });

    const text = JSON.stringify(await service.ensureUserBucket(USER_ID));

    expect(Object.keys(JSON.parse(text)).sort()).toEqual(["bucket", "created"]);
    expect(text).not.toContain(BUCKET_NAME_KEY);
    expect(text).not.toContain("PRIVATE KEY");
  });
});
