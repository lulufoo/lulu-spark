import { describe, expect, it } from "vitest";
import { createBucketService } from "../../src/gcs/bucket-service";
import { InvalidSecretError } from "../../src/gcs/errors";
import { jsonReply, makeFakeFetch, type RecordedRequest } from "./fake-fetch";

const BUCKET_NAME_KEY = "test-bucket-name-key-0123456789abcdef";
const USER_ID = "123e4567-e89b-12d3-a456-426614174000";
const BUCKET = "spark-dee0434c91366b18561ffdcad4380e86";

/** A storage API where the buckets that exist so far are kept in a set. */
function makeFakeStorage() {
  const existing = new Set<string>();
  return makeFakeFetch((request: RecordedRequest) => {
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

function makeService(fetchFn: typeof fetch, overrides: { bucketNameKey?: string } = {}) {
  return createBucketService({
    // `"bucketNameKey" in overrides`, not a default value: passing undefined must mean "missing".
    bucketNameKey: "bucketNameKey" in overrides ? overrides.bucketNameKey : BUCKET_NAME_KEY,
    projectId: "spark-cloud-sync",
    projectNumber: "55973988960",
    getAccessToken: async () => "tok",
    fetchFn,
  });
}

describe("createBucketService", () => {
  it("creates the user's bucket on first use and finds it afterwards", async () => {
    const service = makeService(makeFakeStorage().fetchFn);

    expect(await service.ensureUserBucket(USER_ID)).toEqual({ bucket: BUCKET, created: true });
    expect(await service.ensureUserBucket(USER_ID)).toEqual({ bucket: BUCKET, created: false });
  });

  it("gives two devices that start together the same bucket, created once", async () => {
    const { fetchFn, requests } = makeFakeStorage();
    const service = makeService(fetchFn);

    const results = await Promise.all([service.ensureUserBucket(USER_ID), service.ensureUserBucket(USER_ID)]);

    expect(results.map((result) => result.bucket)).toEqual([BUCKET, BUCKET]);
    expect(results.filter((result) => result.created)).toHaveLength(1);
    // Whether the second device read "missing" first (and got a 409 on create) or found the bucket
    // already there depends on timing; the provisioner tests pin the 409 path. Either way: one create wins.
    expect(requests.filter((request) => request.method === "POST").length).toBeGreaterThanOrEqual(1);
  });

  it("refuses to start without BUCKET_NAME_KEY, or with a cut-off one", () => {
    const { fetchFn } = makeFakeStorage();
    expect(() => makeService(fetchFn, { bucketNameKey: undefined })).toThrow(InvalidSecretError);
    expect(() => makeService(fetchFn, { bucketNameKey: "short" })).toThrow(InvalidSecretError);
  });

  it("returns only the bucket name and whether it was created", async () => {
    const service = makeService(makeFakeStorage().fetchFn);

    const text = JSON.stringify(await service.ensureUserBucket(USER_ID));

    expect(Object.keys(JSON.parse(text)).sort()).toEqual(["bucket", "created"]);
    expect(text).not.toContain(BUCKET_NAME_KEY);
  });
});
